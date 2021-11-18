use crate::hw::vec::ByteSerialized;

use std::fs::File;
use std::sync::mpsc;

use crate::hw::cpu::{Cpu, PsxBus};
use crate::hw::dma::{ChannelLink, Direction, SyncMode};
use crate::hw::{Bios, Cdrom, Dma, Gpu, JoypadMemorycard, Ram, Scratchpad, Spu, Timers};

use std::cell::RefCell;
use std::rc::Rc;

use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub trait R3000Type {}
impl R3000Type for u8 {}
impl R3000Type for u16 {}
impl R3000Type for u32 {}

pub trait BusDevice {
    fn read<T: R3000Type>(&mut self, addr: u32) -> u32;
    fn write<T: R3000Type>(&mut self, addr: u32, value: u32);
}

pub struct Bus {
    pub cpu: RefCell<Cpu<Bus>>,
    pub debug_tx: Option<mpsc::Sender<bool>>,
    pub irq_tx: Option<mpsc::Sender<u32>>,

    pub total_cycles: RefCell<u64>,

    ram: RefCell<Ram>,
    bios: RefCell<Bios>,
    scratchpad: RefCell<Scratchpad>,
    io: RefCell<Vec<u8>>,
    cdrom: RefCell<Cdrom>,
    dma: RefCell<Dma>,
    spu: RefCell<Spu>,
    gpu: RefCell<Gpu>,
    timers: RefCell<Timers>,
    joy_mc: RefCell<JoypadMemorycard>,

    events: RefCell<BinaryHeap<PsxEvent>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd)]
pub enum PsxEventType {
    DeliverCDRomResponse,
    VBlank,
}

#[derive(Debug, Eq, PartialEq)]
struct PsxEvent {
    kind: PsxEventType,
    cycles_target: u64,
    repeat: u64,
}

impl Ord for PsxEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cycles_target.cmp(&self.cycles_target)
    }
}

impl PartialOrd for PsxEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Bus {
    pub fn new() -> Bus {
        let cpu = RefCell::new(Cpu::new());
        let debug_tx = Some(cpu.borrow_mut().new_channel());
        let irq_tx = Some(cpu.borrow_mut().new_irq_channel());

        Bus {
            total_cycles: RefCell::new(0),

            ram: RefCell::new(Ram::new()),
            bios: RefCell::new(Bios::new()),
            scratchpad: RefCell::new(Scratchpad::new()),
            io: RefCell::new(vec![0; 0x1000 + 8 * 1024]),

            cdrom: RefCell::new(Cdrom::new()),
            dma: RefCell::new(Dma::new()),
            spu: RefCell::new(Spu::new()),
            gpu: RefCell::new(Gpu::new()),
            timers: RefCell::new(Timers::new()),
            joy_mc: RefCell::new(JoypadMemorycard::new()),

            cpu,
            irq_tx,
            debug_tx,

            events: RefCell::new(BinaryHeap::new()),
        }
    }

    pub fn run(&self) {
        self.cpu.borrow_mut().run();
    }

    pub fn run_until(&self, target_pc: u32) {
        self.cpu.borrow_mut().run_until(target_pc);
    }

    pub fn run_for(&self, cycles: u64) {
        let target = *self.total_cycles.borrow() + cycles;
        while *self.total_cycles.borrow() < target {
            self.cpu.borrow_mut().cycle();
        }
    }

    /// Installs weak references of self into the devices to allow
    /// omnidirectional communication
    pub fn link(&self, self_ref: Rc<RefCell<Self>>) {
        self.cpu.borrow_mut().link(&self);
        self.timers.borrow_mut().link(Rc::downgrade(&self_ref));
        self.gpu.borrow_mut().link(Rc::downgrade(&self_ref));
        self.gpu.borrow_mut().load_renderer();
        self.cdrom.borrow_mut().link(Rc::downgrade(&self_ref));
    }

    pub fn load_rom(&self, path: &str) {
        let mut file = File::open(path).unwrap();
        self.bios.borrow_mut().load(&mut file);
    }

    pub fn write_io<T: R3000Type>(&self, addr: u32, value: u32) {
        self.io.borrow_mut().write::<T>(addr as u32, value);

        match addr {
            0x1000 => {
                println!("Set Expansion 1 base address to {:x}", value);
            }
            0x1004 => {
                println!("Set Expansion 2 base address to {:x}", value);
            }
            0x1008 => {
                println!("Set Expansion 1 delay/size to {:x}", value);
            }
            0x100c => {
                println!("Set Expansion 3 delay/size to {:x}", value);
            }
            0x1010 => {
                println!("Set BIOS ROM Delay/Size to {:x}", value);
            }
            0x1014 => {
                println!("Set SPU Delay to {:x}", value);
            }
            0x1018 => {
                println!("Set CDROM Delay to {:x}", value);
            }
            0x101c => {
                println!("Set Expansion 2 delay/size to {:x}", value);
            }
            0x1020 => {
                println!("Set COM_DELAY to {:x}", value);
            }
            0x1060 => {
                println!("Set RAM_SIZE to {:x}", value);
            }
            0x2041 => {
                println!("Set POST 7-segments to {:x}", value);
            }
            _ => {
                panic!(
                    "Write to unknown I/O Port: {:x} (value {:x})",
                    0x1f80_0000 + addr,
                    value
                )
            }
        }
    }

    pub fn strip_region(addr: u32) -> u32 {
        const MASK: [u32; 4] = [0x1fff_ffff, 0x1fff_ffff, 0x1fff_ffff, 0xffff_ffff];

        addr & MASK[(addr >> 30) as usize]
    }

    pub fn process_events(&self) {
        let total = self.total_cycles.borrow();
        let mut events = self.events.borrow_mut();

        while let Some(ev) = events.peek() {
            if ev.cycles_target < *total {
                let ev = events.pop().unwrap();

                self.process_event(ev.kind);

                if ev.repeat > 0 {
                    events.push(PsxEvent {
                        kind: ev.kind,
                        repeat: ev.repeat,
                        cycles_target: *total + ev.repeat,
                    });
                }
            } else {
                // The item at the head of the heap isn't ready to be processed
                // yet. So non of them are.
                break;
            }
        }
    }

    pub fn add_event(&self, kind: PsxEventType, mut first_target: u64, repeat_after: u64) {
        let mut events = self.events.borrow_mut();

        // If an event of the same type exists, remove it
        // TODO: retain is unstable API. Alternatives?
        events.retain(|ev| ev.kind != kind);

        if first_target == 0 && repeat_after != 0 {
            first_target = *self.total_cycles.borrow() + repeat_after;
        } else if first_target == 0 {
            panic!("Invalid event");
        }

        // New event
        events.push(PsxEvent {
            kind,
            cycles_target: first_target,
            repeat: repeat_after,
        });
    }

    pub fn remove_event(&self, kind: PsxEventType) {
        let mut events = self.events.borrow_mut();

        // If an event of the same type exists, remove it
        // TODO: retain is unstable API. Alternatives?
        events.retain(|ev| ev.kind != kind);
    }

    pub fn process_event(&self, kind: PsxEventType) {
        match kind {
            PsxEventType::DeliverCDRomResponse => {
                self.cdrom.borrow_mut().next_response();
            }
            PsxEventType::VBlank => {
                self.gpu.borrow_mut().vblank();
            }
        }
    }

    pub fn send_irq(&self, irq_num: u32) {
        if irq_num > 10 {
            panic!("[BUS] Invalid IRQ number");
        }

        if let Some(tx) = &self.irq_tx {
            tx.send(irq_num).unwrap();
        }
    }

    #[inline(always)]
    fn add_cycles(&self, count: u64) {
        (*self.total_cycles.borrow_mut()) += count;
    }
}

impl PsxBus for Bus {
    fn update_cycles(&self, cycles: u64) {
        let mut total = self.total_cycles.borrow_mut();
        (*total) += cycles;

        drop(total);
        self.process_events();
    }

    fn read<T: R3000Type>(&self, addr: u32) -> u32 {
        let addr = Bus::strip_region(addr);

        match addr {
            0x0000_0000..=0x001f_ffff => {
                self.add_cycles(4);
                self.ram.borrow_mut().read::<T>(addr)
            }
            0x1f00_0000..=0x1f7f_ffff => {
                self.add_cycles(6 * std::mem::size_of::<T>() as u64);
                0xffffffff
            }
            0x1f80_1040..=0x1f80_104f => {
                self.add_cycles(2);
                self.joy_mc.borrow_mut().read::<T>(addr - 0x1f80_1040)
            }
            0x1f80_1050..=0x1f80_105f => {
                // SIO
                self.add_cycles(2);
                0
            }
            0x1f80_1060 => {
                // RAM SIZE
                self.add_cycles(2);
                0
            }
            0x1f80_1080..=0x1f80_10f4 => {
                self.add_cycles(2);
                self.dma.borrow_mut().read::<T>(addr - 0x1f80_1080)
            }
            0x1f80_1100..=0x1f80_112f => {
                self.add_cycles(2);
                self.timers.borrow_mut().read::<T>(addr - 0x1f80_1100)
            }
            0x1f80_1800..=0x1f80_1803 => {
                self.add_cycles(6 * std::mem::size_of::<T>() as u64 + 1);
                self.cdrom.borrow_mut().read::<T>(addr - 0x1f80_1800)
            }
            0x1f80_1810..=0x1f80_1814 => {
                self.add_cycles(2);
                self.gpu.borrow_mut().read::<T>(addr - 0x1f80_1810)
            }
            0x1f80_1820..=0x1f80_1824 => {
                // MDEC
                self.add_cycles(2);
                0
            }
            0x1f80_1c00..=0x1f80_1fff => {
                self.add_cycles(17);
                self.spu.borrow_mut().read::<T>(addr - 0x1f80_1c00)
            }
            0x1f80_2000..=0x1f80_2080 => {
                // EXP2 has some weeeeeird timings
                // 10 cycles for 1 byte
                // 25 for 2 bytes
                // 55 for 4 bytes
                self.add_cycles((15 * std::mem::size_of::<T>() - 5) as u64);
                0xffffffff
            }
            0x1fa0_0000 => {
                // EXP3 is not sane either
                // 5 cycles for 1/2 bytes
                // 9 cycles for 4 bytes
                if std::mem::size_of::<T>() == 4 {
                    self.add_cycles(9);
                } else {
                    self.add_cycles(5);
                }

                0xffffffff
            }
            0x1fc0_0000..=0x1fc8_0000 => {
                (*self.total_cycles.borrow_mut()) += 6 * std::mem::size_of::<T>() as u64;
                self.bios.borrow_mut().read::<T>(addr & 0xf_ffff)
            }
            _ => {
                panic!("Read in memory hole at {:08x}", addr);
            }
        }
    }

    fn write<T: R3000Type>(&self, addr: u32, value: u32) {
        match addr {
            0x0000_0000..=0x0020_0000 => {
                self.ram.borrow_mut().write::<T>(addr, value);
            }
            0x1f80_0000..=0x1f80_0400 => {
                self.scratchpad
                    .borrow_mut()
                    .write::<T>(addr - 0x1f80_0000, value);
            }
            0x1f80_1040..=0x1f80_104f => {
                self.joy_mc
                    .borrow_mut()
                    .write::<T>(addr - 0x1f80_1040, value);
            }
            0x1f80_1050..=0x1f80_105f => {
                // SIO: TODO
            }
            0x1f80_1080..=0x1f80_10f4 => {
                self.dma.borrow_mut().write::<T>(addr - 0x1f80_1080, value);
                self.handle_dma_write();
            }
            0x1f80_1100..=0x1f80_112f => {
                self.timers
                    .borrow_mut()
                    .write::<T>(addr - 0x1f80_1100, value);
            }
            0x1f80_1800..=0x1f80_1803 => {
                self.cdrom
                    .borrow_mut()
                    .write::<T>(addr - 0x1f80_1800, value);
            }
            0x1f80_1810..=0x1f80_1814 => {
                self.gpu.borrow_mut().write::<T>(addr - 0x1f80_1810, value);
            }
            0x1f80_1820..=0x1f80_1824 => {
                // MDEC: TODO
            }
            0x1f80_1c00..=0x1f80_1fff => {
                self.spu.borrow_mut().write::<T>(addr - 0x1f80_1c00, value);
            }
            0x1f80_2000..=0x1f80_207f => {
                // EXP2: ignore
                // However at 2041, there's the POST 7seg display
            }
            0x1f80_1000..=0x1f80_1020 | 0x1f80_1060 => {
                self.write_io::<T>(addr & 0xffff, value);
            }
            0x1fa0_0000 => {
                // EXP3: ignore
            }
            0x1fc0_0000..=0x1fc8_0000 => {
                // Ignore writes to the ROM
            }
            _ => {
                panic!("Cannot write value {:x} at {:x}", value, addr);
            }
        }
    }
}

impl Bus {
    fn handle_dma_write(&self) {
        if let Some(active_channel) = self.dma.borrow_mut().active_channel() {
            let step = active_channel.step();
            let mut addr = active_channel.base();

            let (blocks, block_size) = active_channel.transfer_size();

            match active_channel.sync_mode() {
                SyncMode::Immediate => match active_channel.link() {
                    ChannelLink::Otc => {
                        let mut remaining_words = block_size;
                        while remaining_words > 0 {
                            match active_channel.direction() {
                                Direction::FromRam => {
                                    panic!("Cannot OTC from RAM");
                                }
                                Direction::ToRam => {
                                    let word = match remaining_words {
                                        1 => 0xff_ffff,
                                        _ => addr.wrapping_add(step as u32) & 0x1f_fffc,
                                    };
                                    self.ram.borrow_mut().write::<u32>(addr, word);
                                }
                            }
                            addr = addr.wrapping_add(step as u32) & 0x1f_fffc;
                            remaining_words -= 1;
                        }
                        active_channel.done();
                    }
                    ChannelLink::Cdrom => {
                        let mut remaining_words = block_size * blocks;
                        let mut cdrom = self.cdrom.borrow_mut();
                        while remaining_words > 0 {
                            match active_channel.direction() {
                                Direction::ToRam => {
                                    let value = cdrom.read::<u8>(2) | cdrom.read::<u8>(2) << 8 | cdrom.read::<u8>(2) << 16 | cdrom.read::<u8>(2) << 24;
                                    self.ram.borrow_mut().write::<u32>(addr, value);
                                    addr = addr.wrapping_add(4);
                                    remaining_words -= 1;
                                    println!("writing from CDROM to {:08x}", addr);
                                }
                                Direction::FromRam => {
                                    panic!("Writing to CDROM? Not happening");
                                }
                            }
                        }
                    }
                    _ => {
                        panic!("Cannot handle link {:?}", active_channel.link());
                    }
                },
                SyncMode::LinkedList => {
                    match active_channel.link() {
                        ChannelLink::Gpu => {
                            loop {
                                match active_channel.direction() {
                                    Direction::FromRam => {
                                        let header = self.ram.borrow_mut().read::<u32>(addr);
                                        let word_count = header >> 24;

                                        // println!("[DMA] GPU packet {} words {:08x}", word_count, header);

                                        for _ in 0..word_count {
                                            addr = addr.wrapping_add(step as u32);
                                            let cmd = self.ram.borrow_mut().read::<u32>(addr);
                                            self.gpu.borrow_mut().process_gp0(cmd);
                                        }

                                        addr = header & 0xffffff;
                                        if addr == 0xffffff {
                                            break;
                                        }
                                    }
                                    Direction::ToRam => {
                                        panic!("Cannot DMA2-GPU to ram");
                                    }
                                }
                            }
                            active_channel.done();
                        }
                        _ => {
                            panic!("Linked list is for gpu only");
                        }
                    }
                }
                SyncMode::Sync => match active_channel.link() {
                    ChannelLink::Gpu => {
                        for _ in 0..(blocks * block_size) as usize {
                            match active_channel.direction() {
                                Direction::FromRam => {
                                    let value = self.ram.borrow_mut().read::<u32>(addr);
                                    self.gpu.borrow_mut().process_gp0(value);
                                    addr = addr.wrapping_add(step as u32);
                                }
                                Direction::ToRam => {
                                    panic!("Cannot DMA2-GPU to ram");
                                }
                            }
                        }
                        active_channel.done();
                    }
                    _ => {
                        panic!("Linked list is for gpu only");
                    }
                },
                _ => {
                    println!("Unhandled sync mode {:?}", active_channel.sync_mode());
                    active_channel.done();
                }
            };
        }
    }

    pub fn load_exe(&self, path: &str) {
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

        let mut ram = self.ram.borrow_mut();

        for b in code.iter() {
            ram.write::<u8>(addr, *b as u32);
            addr = (addr + 1) & 0x3f_ffff;
        }

        let mut cpu = self.cpu.borrow_mut();
        cpu.pc = header.pc;
        cpu.regs[28] = header.r28;
        cpu.regs[29] = header.r29_base + header.r29_offset;

        if cpu.regs[29] == 0 {
            cpu.regs[29] = 0x801f_fff0;
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
