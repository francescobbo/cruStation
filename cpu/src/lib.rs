mod arith;
mod biu;
mod branch;
mod cop;
mod cop0;
pub mod gte;
mod icache;
mod instruction;
mod load_store;
mod scratchpad;

use std::sync::mpsc;
// use std::time::{SystemTime, UNIX_EPOCH};

use crustationlogger::*;

use biu::BIUCacheControl;
use cop0::{Cop0, Exception};
use gte::Gte;
use icache::InstructionCache;
use instruction::Instruction;
use scratchpad::Scratchpad;

pub trait PsxBus {
    fn read<const T: u32>(&mut self, address: u32) -> u32;
    fn write<const T: u32>(&mut self, address: u32, value: u32);
    fn update_cycles(&self, cycles: u64);
}

pub enum CpuCommand {
    Break,
    Irq(u32),
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct LoadDelaySlot {
    register: u32,
    value: u32,
}

pub struct Cpu {
    logger: Logger,

    command_rx: mpsc::Receiver<CpuCommand>,
    pub command_tx: mpsc::Sender<CpuCommand>,

    pub pc: u32,
    pub regs: [u32; 33],
    pub hi: u32,
    pub lo: u32,

    pub cop0: Cop0,
    pub gte: Gte,

    icache: InstructionCache,
    dcache: Scratchpad,

    biu_cc: BIUCacheControl,
    i_stat: u32,
    i_mask: u32,

    current_instruction: Instruction,
    branch_delay_slot: Option<(u32, u32)>,
    load_delay_slot: [LoadDelaySlot; 2],
    in_delay: bool,
}

impl Cpu {
    pub fn new() -> Cpu {
        let (tx, rx) = mpsc::channel();

        Cpu {
            logger: Logger::new("CPU", Level::Info),

            command_rx: rx,
            command_tx: tx,

            pc: 0xbfc0_0000,
            regs: [0; 33],
            hi: 0,
            lo: 0,

            cop0: Cop0::new(),
            gte: Gte::new(),

            icache: InstructionCache::new(),
            dcache: Scratchpad::new(),

            biu_cc: BIUCacheControl(0),
            i_stat: 0,
            i_mask: 0,

            current_instruction: Instruction(0),
            branch_delay_slot: None,
            load_delay_slot: [
                LoadDelaySlot {
                    register: 32,
                    value: 0,
                },
                LoadDelaySlot {
                    register: 32,
                    value: 0,
                },
            ],
            in_delay: false,
            // ips: 0,
            // ips_start: SystemTime::now()
            //     .duration_since(UNIX_EPOCH)
            //     .unwrap()
            //     .as_millis(),
        }
    }

    #[inline(always)]
    pub fn fetch_at_pc<B: PsxBus>(&mut self, bus: &mut B) -> u32 {
        // Uncomment for hardware-faithful implementation
        // if !self.biu_cc.is1() {
        //     return self.load::<u32>(self.pc);
        // }

        if self.pc >= 0xa000_0000 {
            return self.load::<4, B>(bus, self.pc);
        }

        match self.icache.load(self.pc) {
            Some(ins) => ins,
            None => {
                // Fetch and store the current instruction
                let ins: u32;
                ins = self.load::<4, B>(bus, self.pc);
                self.icache.store(self.pc, ins);

                // Fetch up to 4 words (from current PC up to next 16-byte
                // alignment). TODO: this might be 2 words (but unlikely to
                // ever be used).
                let mut next = self.pc.wrapping_add(4);
                while next & 0xf != 0 {
                    let ins = self.load::<4, B>(bus, next);
                    self.icache.store(next, ins);

                    next = next.wrapping_add(4);
                }

                ins
            }
        }
    }

    pub fn run<B: PsxBus>(&mut self, bus: &mut B) {
        loop {
            self.cycle(bus);
        }
    }

    pub fn run_until<B: PsxBus>(&mut self, bus: &mut B, desired_pc: u32) {
        loop {
            self.cycle(bus);

            if self.pc == desired_pc {
                break;
            }
        }
    }

    pub fn cycle<B: PsxBus>(&mut self, bus: &mut B) {
        if let Ok(command) = self.command_rx.try_recv() {
            match command {
                CpuCommand::Break => {
                    println!();
                    // debug::Debugger::enter(self);
                }
                CpuCommand::Irq(n) => {
                    self.request_interrupt(n);
                }
            }
        }

        self.step(bus);

        if self.cop0.should_interrupt() {
            self.interrupt();
        }

        bus.update_cycles(1);
    }

    #[inline(always)]
    pub fn pc(&self) -> u32 {
        if let Some((pc, _)) = self.branch_delay_slot {
            pc
        } else {
            self.pc
        }
    }

    #[inline(always)]
    pub fn step<B: PsxBus>(&mut self, bus: &mut B) {
        if let Some((_pc, ins)) = self.branch_delay_slot {
            self.in_delay = true;
            self.current_instruction.0 = ins;
            self.branch_delay_slot = None;
        } else {
            self.in_delay = false;

            if self.pc % 4 != 0 {
                self.exception(Exception::AddressErrorLoad);
                return;
            }

            self.current_instruction.0 = self.fetch_at_pc(bus);
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
                0x08 => self.ins_jr(bus),
                0x09 => self.ins_jalr(bus),
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
                    warn!(
                        self.logger,
                        "Unhandled instruction {:08x} at {:08x}",
                        self.current_instruction.0,
                        self.pc
                    );
                    self.exception(Exception::ReservedInstruction);
                }
            },
            0x01 => self.ins_bcondz(bus),
            0x02 => self.ins_j(bus),
            0x03 => self.ins_jal(bus),
            0x04 => self.ins_beq(bus),
            0x05 => self.ins_bne(bus),
            0x06 => self.ins_blez(bus),
            0x07 => self.ins_bgtz(bus),
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
            0x20 => self.ins_lb(bus),
            0x21 => self.ins_lh(bus),
            0x22 => self.ins_lwl(bus),
            0x23 => self.ins_lw(bus),
            0x24 => self.ins_lbu(bus),
            0x25 => self.ins_lhu(bus),
            0x26 => self.ins_lwr(bus),
            0x28 => self.ins_sb(bus),
            0x29 => self.ins_sh(bus),
            0x2A => self.ins_swl(bus),
            0x2B => self.ins_sw(bus),
            0x2E => self.ins_swr(bus),
            0x30 => self.ins_lwc0(),
            0x31 => self.ins_lwc1(),
            0x32 => self.ins_lwc2(bus),
            0x33 => self.ins_lwc3(),
            0x38 => self.ins_swc0(),
            0x39 => self.ins_swc1(),
            0x3A => self.ins_swc2(bus),
            0x3B => self.ins_swc3(),
            _ => {
                warn!(
                    self.logger,
                    "Unhandled instruction {:08x} at {:08x}", self.current_instruction.0, self.pc
                );
                self.exception(Exception::ReservedInstruction);
            }
        }

        self.in_delay = false;
        self.load_delays();
    }

    #[inline(always)]
    fn load_delays(&mut self) {
        self.regs[self.load_delay_slot[0].register as usize] = self.load_delay_slot[0].value;

        self.load_delay_slot[0] = self.load_delay_slot[1];
        self.load_delay_slot[1].register = 32;
    }

    pub fn request_interrupt(&mut self, irq_number: u32) {
        self.i_stat |= 1 << irq_number;
        self.check_interrupts();
    }

    #[inline(always)]
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

        if self.load_delay_slot[0].register == reg {
            self.load_delay_slot[0].register = 32;
        }
    }
}
