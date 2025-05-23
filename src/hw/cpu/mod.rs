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

use std::{collections::VecDeque, fs::File};

use crustationgui::GpuCommand;
use crustationlogger::*;

use biu::BIUCacheControl;
use cop0::{Cop0, Exception};
use gte::Gte;
use icache::InstructionCache;
use instruction::Instruction;
use scratchpad::Scratchpad;

use crate::{debug, hw::bus::BusDevice};

use super::bus::{Bus, CpuCommand};

#[derive(Copy, Clone, Eq, PartialEq)]
struct LoadDelaySlot {
    register: u32,
    value: u32,
}

pub struct Cpu {
    logger: Logger,

    pub bus: Bus,

    pub pc: u32,
    pub regs: [u32; 33], // The extra 33rd register is a placeholder for unused load delay slots
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
    pub branch_delay_slot: Option<(u32, u32)>,
    load_delay_slot: Option<LoadDelaySlot>,
    last_reg_write: u32,
    in_delay: bool,

    extra_cycles: u64,

    pub tasks: VecDeque<CpuCommand>,

    pub debugger: debug::Debugger,
}

impl Cpu {
    pub fn new(renderer_tx: crossbeam_channel::Sender<GpuCommand>) -> Cpu {
        Cpu {
            logger: Logger::new("CPU", Level::Info),
            bus: Bus::new(renderer_tx),

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
            load_delay_slot: None,
            in_delay: false,
            last_reg_write: 0,
            // ips: 0,
            // ips_start: SystemTime::now()
            //     .duration_since(UNIX_EPOCH)
            //     .unwrap()
            //     .as_millis(),
            extra_cycles: 0,
            tasks: VecDeque::new(),

            debugger: debug::Debugger::new(),
        }
    }

    #[inline(always)]
    pub fn fetch_at_pc(&mut self) -> u32 {
        // Uncomment for hardware-faithful implementation
        // if !self.biu_cc.is1() {
        //     return self.load::<u32>(self.pc);
        // }

        if self.pc >= 0xa000_0000 {
            return self.load::<4>(self.pc);
        }

        match self.icache.load(self.pc) {
            Some(ins) => ins,
            None => {
                // Fetch and store the current instruction
                let ins: u32;
                ins = self.load::<4>(self.pc);
                self.icache.store(self.pc, ins);

                // Fetch up to 4 words (from current PC up to next 16-byte
                // alignment). TODO: this might be 2 words (but unlikely to
                // ever be used).
                let mut next = self.pc.wrapping_add(4);
                while next & 0xf != 0 {
                    let ins = self.load::<4>(next);
                    self.icache.store(next, ins);

                    next = next.wrapping_add(4);
                }

                ins
            }
        }
    }

    // pub fn run(&mut self) {
    //     loop {
    //         self.step();
    //     }
    // }

    pub fn run(&mut self) {
        loop {
            if let Some(command) = self.tasks.pop_front() {
                match command {
                    CpuCommand::Irq(n) => {
                        self.request_interrupt(n);
                    }
                    _ => {}
                }
            }

            let cycles = self.step();
            self.tasks.extend(self.bus.add_cycles(cycles));
        }
    }

    pub fn run_until(&mut self, desired_pc: u32) {
        loop {
            if let Some(command) = self.tasks.pop_front() {
                match command {
                    CpuCommand::Irq(n) => {
                        self.request_interrupt(n);
                    }
                    _ => {}
                }
            }

            let cycles = self.step();
            self.tasks.extend(self.bus.add_cycles(cycles));

            if self.pc == desired_pc {
                break;
            }
        }
    }

    pub fn send_irq(&mut self, irq_num: u32) {
        if irq_num > 10 {
            panic!("[BUS] Invalid IRQ number");
        }

        self.tasks.push_back(CpuCommand::Irq(irq_num));
    }

    pub fn step(&mut self) -> u64 {
        self.extra_cycles = 0;

        if debug::Debugger::should_break(self) {
            debug::Debugger::enter(self);
        }

        self.step_inner();

        // match self.pc() {
        //     0xa0 => Bios::call_a(self),
        //     0xb0 => Bios::call_b(self),
        //     0xc0 => Bios::call_c(self),
        //     _ => {}
        // }

        if self.cop0.should_interrupt() {
            self.interrupt();
        }

        return 1 + self.extra_cycles;
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
    pub fn step_inner(&mut self) {
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

            self.current_instruction.0 = self.fetch_at_pc();
            self.pc = self.pc.wrapping_add(4);
        }

        // println!("[PC] {:08x} {:08x}", self.pc, self.current_instruction.0);
        let delay = self.load_delay_slot.take();
        self.last_reg_write = 0;

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
                    warn!(
                        self.logger,
                        "Unhandled instruction {:08x} at {:08x}",
                        self.current_instruction.0,
                        self.pc
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
            0x22 => self.ins_lwl(delay),
            0x23 => self.ins_lw(),
            0x24 => self.ins_lbu(),
            0x25 => self.ins_lhu(),
            0x26 => self.ins_lwr(delay),
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
                warn!(
                    self.logger,
                    "Unhandled instruction {:08x} at {:08x}", self.current_instruction.0, self.pc
                );
                self.exception(Exception::ReservedInstruction);
            }
        }

        self.in_delay = false;
        self.load_delays(delay);
    }

    #[inline(always)]
    fn load_delays(&mut self, delay: Option<LoadDelaySlot>) {
        if let Some(LoadDelaySlot {
            register, value, ..
        }) = delay
        {
            if register == 0 || register == self.last_reg_write {
                return;
            }

            self.regs[register as usize] = value;
        }
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
        self.last_reg_write = reg;
    }

    pub fn load_exe(&mut self, path: &str) {
        use std::io::BufReader;
        use std::io::Read;
        use std::io::Seek;
        use std::mem;

        let mut header = PsxExeHeader::default();
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);

        unsafe {
            let buffer: &mut [u8] = std::slice::from_raw_parts_mut(
                &mut header as *mut _ as *mut u8,
                mem::size_of::<PsxExeHeader>(),
            );

            reader.read_exact(buffer).unwrap();
        }

        reader.seek(std::io::SeekFrom::Start(0x800)).unwrap();
        let mut code = vec![0_u8; header.size as usize];
        reader.read_exact(&mut code).unwrap();

        let mut addr = header.destination & 0x1f_fffc;

        let ram = &mut self.bus.ram;

        for b in code.iter() {
            ram.write::<1>(addr, *b as u32);
            addr = (addr + 1) & 0x3f_ffff;
        }

        self.pc = header.pc;
        self.regs[28] = header.r28;
        self.regs[29] = header.r29_base + header.r29_offset;

        if self.regs[29] == 0 {
            self.regs[29] = 0x801f_fff0;
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct PsxExeHeader {
    signature: [u8; 8],
    zero1: [u8; 8],
    pc: u32,
    r28: u32,
    destination: u32,
    size: u32,
    zero2: [u32; 2],
    memfill_address: u32,
    memfill_size: u32,
    r29_base: u32,
    r29_offset: u32,
}
