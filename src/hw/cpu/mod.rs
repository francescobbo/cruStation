use std::cell::RefCell;
use std::rc::Weak;
use std::sync::mpsc;

use crate::hw::cpu::cop0::{Cop0, Exception};
use crate::hw::{Bios, Gte};

use bitfield::bitfield;

mod arith;
mod biu;
mod branch;
mod cop;
mod cop0;
mod debug;
mod icache;
mod load_store;

use biu::BIUCacheControl;
use icache::InstructionCache;

use std::time::{SystemTime, UNIX_EPOCH};

// Don't like using a bus::Thing here.
use crate::hw::bus::R3000Type;
use crate::hw::disasm;

pub trait PsxBus {
    fn read<T: R3000Type>(&self, address: u32) -> u32;
    fn write<T: R3000Type>(&self, address: u32, value: u32);
    fn update_cycles(&self, cycles: u64);
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct LoadDelaySlot {
    register: Option<u32>,
    value: u32,
}

bitfield! {
    pub struct Instruction(u32);
    impl Debug;

    #[inline]
    pub special_opcode, _: 5, 0;
    #[inline]
    pub opcode, _: 31, 26;
    #[inline]
    pub rs, _: 25, 21;
    #[inline]
    pub rt, _: 20, 16;
    #[inline]
    pub rd, _: 15, 11;
    #[inline]
    pub imm16, _: 15, 0;
    #[inline]
    pub i16, simm16, _: 15, 0;
    #[inline]
    pub imm5, _: 10, 6;
    #[inline]
    pub imm26, _: 25, 0;
}

pub struct Cpu<T: PsxBus> {
    pub pc: u32,
    pub regs: [u32; 32],
    pub hi: u32,
    pub lo: u32,

    pub cop0: Cop0,
    pub icache: InstructionCache,
    pub dcache: Vec<u8>,

    pub current_instruction: Instruction,
    pub branch_delay_slot: Option<(u32, u32)>,
    pub load_delay_slot: [LoadDelaySlot; 2],
    in_delay: bool,

    pub bus: *const T,
    pub gte: Gte,

    pub debugger: debug::Debugger,

    biu_cc: BIUCacheControl,
    i_stat: u32,
    i_mask: u32,

    ctrl_ch: mpsc::Receiver<bool>,
    irq_ch: mpsc::Receiver<u32>,

    ips: u64,
    ips_start: u128,
}

impl<T: PsxBus> Cpu<T> {
    pub fn new() -> Cpu<T> {
        let (_, debug_rx) = mpsc::channel();
        let (_, irq_rx) = mpsc::channel();

        Cpu {
            bus: std::ptr::null(),
            cop0: Cop0::new(),
            icache: InstructionCache::new(),
            dcache: vec![0; 0x400],

            pc: 0xbfc0_0000,
            regs: [0; 32],
            hi: 0,
            lo: 0,
            current_instruction: Instruction(0),
            gte: Gte::new(),
            branch_delay_slot: None,
            load_delay_slot: [
                LoadDelaySlot {
                    register: None,
                    value: 0,
                },
                LoadDelaySlot {
                    register: None,
                    value: 0,
                },
            ],
            in_delay: false,

            biu_cc: BIUCacheControl(0),
            i_stat: 0,
            i_mask: 0,

            debugger: debug::Debugger::new(),
            ctrl_ch: debug_rx,
            irq_ch: irq_rx,

            ips: 0,
            ips_start: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        }
    }

    pub fn link(&mut self, bus: &T) {
        self.bus = bus as *const T;
    }

    pub fn new_channel(&mut self) -> mpsc::Sender<bool> {
        let (tx, rx) = mpsc::channel();
        self.ctrl_ch = rx;

        tx
    }

    pub fn new_irq_channel(&mut self) -> mpsc::Sender<u32> {
        let (tx, rx) = mpsc::channel();
        self.irq_ch = rx;

        tx
    }

    pub fn fetch_at_pc(&mut self) -> u32 {
        // Uncomment for hardware-faithful implementation
        // if !self.biu_cc.is1() {
        //     return self.load::<u32>(self.pc);
        // }

        if self.pc >= 0xa000_0000 {
            return self.load::<u32>(self.pc);
        }

        match self.icache.load(self.pc) {
            None => {
                // Fetch and store the current instruction
                let ins: u32;
                unsafe {
                    ins = self.load::<u32>(self.pc);
                    self.icache.store(self.pc, ins);
                }

                // Fetch up to 4 words (from current PC up to next 16-byte
                // alignment). TODO: this might be 2 words (but unlikely to
                // ever be used).
                let mut next = self.pc.wrapping_add(4);
                while next & 0xf != 0 {
                    unsafe {
                        let ins = self.load::<u32>(next);
                        self.icache.store(next, ins);
                    }

                    next = next.wrapping_add(4);
                }

                ins
            }
            Some(ins) => ins,
        }
    }

    pub fn run(&mut self) {
        debug::Debugger::enter(self);

        loop {
            self.cycle();
        }
    }

    pub fn run_until(&mut self, desired_pc: u32) {
        loop {
            self.cycle();

            if self.pc == desired_pc {
                break;
            }
        }
    }

    pub fn cycle(&mut self) {
        if self.ctrl_ch.try_recv().is_ok() {
            println!();
            debug::Debugger::enter(self);
            // self.debugger.stepping = true;
        } else if debug::Debugger::should_break(self) {
            debug::Debugger::enter(self);
        }

        self.step();

        match self.pc() {
            0xa0 => Bios::call_a(self),
            0xb0 => Bios::call_b(self),
            0xc0 => Bios::call_c(self),
            _ => {}
        }

        if let Ok(irq) = self.irq_ch.try_recv() {
            self.request_interrupt(irq);
        }

        if self.cop0.should_interrupt() {
            self.interrupt();
        }

        unsafe {
            (*self.bus).update_cycles(1);
        }
    }

    pub fn pc(&self) -> u32 {
        if let Some((pc, _)) = self.branch_delay_slot {
            pc
        } else {
            self.pc
        }
    }

    pub fn step(&mut self) {
        if let Some((_pc, ins)) = self.branch_delay_slot {
            self.in_delay = true;
            self.current_instruction.0 = ins;
            self.branch_delay_slot = None;
            if self.debugger.stepping {
                println!("[{:08x}]", _pc);
            }
        } else {
            self.in_delay = false;

            if self.pc % 4 != 0 {
                self.exception(Exception::AddressErrorLoad);
                return;
            }

            self.current_instruction.0 = self.fetch_at_pc();
            if self.debugger.stepping {
                println!("[{:08x}]", self.pc);
            }
            self.pc = self.pc.wrapping_add(4);
        }


        match self.current_instruction.opcode() {
            0x00 => match self.current_instruction.special_opcode() {
                0x00 => self.ins_sll(),
                0x02 => self.ins_srl(),
                0x03 => self.ins_sra(),
                0x04 => self.ins_sllv(),
                0x06 => self.ins_srlv(),
                0x07 => self.ins_srav(),
                0x08 => self.ins_jr(),
                0x09 => self.ins_jalr(),
                0x0C => self.ins_syscall(),
                0x0D => self.ins_break(),
                0x10 => self.ins_mfhi(),
                0x11 => self.ins_mthi(),
                0x12 => self.ins_mflo(),
                0x13 => self.ins_mtlo(),
                0x18 => self.ins_mult(),
                0x19 => self.ins_multu(),
                0x1A => self.ins_div(),
                0x1B => self.ins_divu(),
                0x20 => self.ins_add(),
                0x21 => self.ins_addu(),
                0x22 => self.ins_sub(),
                0x23 => self.ins_subu(),
                0x24 => self.ins_and(),
                0x25 => self.ins_or(),
                0x26 => self.ins_xor(),
                0x27 => self.ins_nor(),
                0x2A => self.ins_slt(),
                0x2B => self.ins_sltu(),
                _ => {
                    println!(
                        "Unhandled instruction {:x} at {:08x}",
                        self.current_instruction.0, self.pc
                    );
                    self.exception(Exception::ReservedInstruction);
                }
            },
            0x01 => self.ins_bcondz(),
            0x02 => self.ins_j(),
            0x03 => self.ins_jal(),
            0x04 => self.ins_beq(),
            0x05 => self.ins_bne(),
            0x06 => self.ins_blez(),
            0x07 => self.ins_bgtz(),
            0x08 => self.ins_addi(),
            0x09 => self.ins_addiu(),
            0x0A => self.ins_slti(),
            0x0B => self.ins_sltiu(),
            0x0C => self.ins_andi(),
            0x0D => self.ins_ori(),
            0x0E => self.ins_xori(),
            0x0F => self.ins_lui(),
            0x10 => self.ins_cop0(),
            0x11 => self.ins_cop1(),
            0x12 => self.ins_cop2(),
            0x13 => self.ins_cop3(),
            0x20 => self.ins_lb(),
            0x21 => self.ins_lh(),
            0x22 => self.ins_lwl(),
            0x23 => self.ins_lw(),
            0x24 => self.ins_lbu(),
            0x25 => self.ins_lhu(),
            0x26 => self.ins_lwr(),
            0x28 => self.ins_sb(),
            0x29 => self.ins_sh(),
            0x2A => self.ins_swl(),
            0x2B => self.ins_sw(),
            0x2E => self.ins_swr(),
            0x30 => self.ins_lwc0(),
            0x31 => self.ins_lwc1(),
            0x32 => self.ins_lwc2(),
            0x33 => self.ins_lwc3(),
            0x38 => self.ins_swc0(),
            0x39 => self.ins_swc1(),
            0x3A => self.ins_swc2(),
            0x3B => self.ins_swc3(),
            _ => {
                println!("Unhandled instruction {:x}", self.current_instruction.0);
                self.exception(Exception::ReservedInstruction);
            }
        }

        self.in_delay = false;
        self.load_delays();
    }

    fn load_delays(&mut self) {
        if let Some(r) = self.load_delay_slot[0].register {
            self.regs[r as usize] = self.load_delay_slot[0].value;
        }

        self.load_delay_slot[0] = self.load_delay_slot[1];
        self.load_delay_slot[1].register = None;
    }

    pub fn request_interrupt(&mut self, irq_number: u32) {
        self.i_stat |= 1 << irq_number;
        self.check_interrupts();
    }

    pub fn check_interrupts(&mut self) {
        let stat = self.i_stat;
        let mask = self.i_mask;

        if stat & mask == 0 {
            self.cop0.clear_interrupt(10);
        } else {
            self.cop0.request_interrupt(10);
        }
    }

    #[inline(always)]
    fn r_rs(&self) -> u32 {
        self.regs[self.current_instruction.rs() as usize]
    }

    #[inline(always)]
    fn r_rt(&self) -> u32 {
        self.regs[self.current_instruction.rt() as usize]
    }

    #[inline(always)]
    pub fn write_reg(&mut self, reg: u32, value: u32) {
        if reg == 0 {
            return;
        }

        self.regs[reg as usize] = value;

        if self.load_delay_slot[0].register == Some(reg) {
            self.load_delay_slot[0].register = None;
        }
    }
}
