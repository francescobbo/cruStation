use crustationgui::GpuCommand;

use crate::hw::vec::ByteSerialized;

use std::fs::File;

use crate::hw::dma::{ChannelLink, Direction, SyncMode};
use crate::hw::{Bios, Cdrom, Dma, Gpu, JoypadMemorycard, Ram, Spu, Timers};

use std::cell::RefCell;

use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub trait BusDevice {
    fn read<const S: u32>(&mut self, addr: u32) -> u32;
    fn write<const S: u32>(&mut self, addr: u32, value: u32);
}

pub struct Bus {
    pub total_cycles: u64,

    pub ram: Ram,
    bios: Bios,
    io: Vec<u8>,
    cdrom: Cdrom,
    dma: Dma,
    spu: Spu,
    gpu: Gpu,
    timers: Timers,
    joy_mc: JoypadMemorycard,

    events: BinaryHeap<PsxEvent>,

}

pub enum CpuCommand {
    Noop,
    Irq(u32),
    EnqueueEvent(PsxEventType, u64, u64),
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
    pub fn new(renderer_tx: crossbeam_channel::Sender<GpuCommand>) -> Bus {
        let mut bus = Bus {
            total_cycles: 0,

            ram: Ram::new(),
            bios: Bios::new(),
            io: vec![0; 0x1000 + 8 * 1024],

            cdrom: Cdrom::new(),
            dma: Dma::new(),
            spu: Spu::new(),
            gpu: Gpu::new(renderer_tx),
            timers: Timers::new(),
            joy_mc: JoypadMemorycard::new(),

            events: BinaryHeap::new(),

        };

        let cpu_freq = 33868800;
        let vblank_freq = 60;
        let vblank_cycles = cpu_freq / vblank_freq;
        bus.add_event(PsxEventType::VBlank, 0, vblank_cycles);

        #[cfg(not(test))]
        bus.gpu.load_renderer();

        bus
    }

    // pub fn run_for(&self, cycles: u64) {
    //     let target = *self.total_cycles.borrow() + cycles;
    //     while *self.total_cycles.borrow() < target {
    //         self.cpu.borrow_mut().cycle();
    //     }
    // }

    pub fn load_rom(&mut self, path: &str) {
        let mut file = File::open(path).unwrap();
        self.bios.load(&mut file);
    }

    pub fn write_io<const S: u32>(&mut self, addr: u32, value: u32) {
        self.io.write::<S>(addr as u32, value);

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

    pub fn process_events(&mut self) -> Vec<CpuCommand> {
        let total = self.total_cycles;
        let mut commands = Vec::new();

        while let Some(ev) = self.events.peek() {
            if ev.cycles_target < total {
                let ev = self.events.pop().unwrap();

                let enqueue = self.process_event(ev.kind);
                commands.extend(enqueue);

                if ev.repeat > 0 {
                    self.events.push(PsxEvent {
                        kind: ev.kind,
                        repeat: ev.repeat,
                        cycles_target: total + ev.repeat,
                    });
                }
            } else {
                // The item at the head of the heap isn't ready to be processed
                // yet. So non of them are.
                break;
            }
        }

        commands
    }

    pub fn add_event(&mut self, kind: PsxEventType, mut first_target: u64, repeat_after: u64) {
        // If an event of the same type exists, remove it
        self.events.retain(|ev| ev.kind != kind);

        if first_target == 0 && repeat_after != 0 {
            first_target = self.total_cycles + repeat_after;
        } else if first_target == 0 {
            panic!("Invalid event");
        }

        // New event
        self.events.push(PsxEvent {
            kind,
            cycles_target: first_target,
            repeat: repeat_after,
        });
    }

    // pub fn remove_event(&self, kind: PsxEventType) {
    //     let mut events = self.events;

    //     // If an event of the same type exists, remove it
    //     events.retain(|ev| ev.kind != kind);
    // }

    pub fn process_event(&mut self, kind: PsxEventType) -> Vec<CpuCommand> {
        match kind {
            PsxEventType::DeliverCDRomResponse => {
                self.cdrom.next_response();
                vec![CpuCommand::Irq(2)]
            }
            PsxEventType::VBlank => {
                self.gpu.vblank();
                vec![CpuCommand::Irq(0)]
            }
        }
    }

    #[inline(always)]
    pub fn add_cycles(&mut self, count: u64) -> Vec<CpuCommand> {
        self.total_cycles += count;
        self.process_events()
    }

    pub fn read<const S: u32>(&mut self, addr: u32) -> u32 {
        let addr = Bus::strip_region(addr);

        match addr {
            0x0000_0000..=0x001f_ffff => {
                self.add_cycles(4);
                self.ram.read::<S>(addr)
            }
            0x1f00_0000..=0x1f7f_ffff => {
                self.add_cycles(6 * S as u64);
                0xffffffff
            }
            0x1f80_1040..=0x1f80_104f => {
                self.add_cycles(2);
                self.joy_mc.read::<S>(addr - 0x1f80_1040)
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
                self.dma.read::<S>(addr - 0x1f80_1080)
            }
            0x1f80_1100..=0x1f80_112f => {
                self.add_cycles(2);
                self.timers.read::<S>(addr - 0x1f80_1100, self.total_cycles)
            }
            0x1f80_1800..=0x1f80_1803 => {
                self.add_cycles(6 * S as u64 + 1);
                let (val, cmd) = self.cdrom.read::<S>(addr - 0x1f80_1800);

                match cmd {
                    CpuCommand::EnqueueEvent(evt, ft, ra) => {
                        self.add_event(evt, ft, ra);
                    }
                    _ => {}
                }

                val
            }
            0x1f80_1810..=0x1f80_1814 => {
                self.add_cycles(2);
                self.gpu.read::<S>(addr - 0x1f80_1810)
            }
            0x1f80_1820..=0x1f80_1824 => {
                // MDEC
                self.add_cycles(2);
                0
            }
            0x1f80_1c00..=0x1f80_1fff => {
                self.add_cycles(17);
                self.spu.read::<S>(addr - 0x1f80_1c00)
            }
            0x1f80_2000..=0x1f80_2080 => {
                // EXP2 has some weeeeeird timings
                // 10 cycles for 1 byte
                // 25 for 2 bytes
                // 55 for 4 bytes
                self.add_cycles((15 * S - 5) as u64);
                0xffffffff
            }
            0x1fa0_0000 => {
                // EXP3 is not sane either
                // 5 cycles for 1/2 bytes
                // 9 cycles for 4 bytes
                if S == 4 {
                    self.add_cycles(9);
                } else {
                    self.add_cycles(5);
                }

                0xffffffff
            }
            0x1fc0_0000..=0x1fc8_0000 => {
                self.total_cycles += 6 * S as u64;
                self.bios.read::<S>(addr & 0xf_ffff)
            }
            _ => {
                panic!("Read in memory hole at {:08x}", addr);
            }
        }
    }

    pub fn write<const S: u32>(&mut self, addr: u32, value: u32) {
        match addr {
            0x0000_0000..=0x0020_0000 => {
                self.ram.write::<S>(addr, value);
            }
            0x1f80_1040..=0x1f80_104f => {
                self.joy_mc.write::<S>(addr - 0x1f80_1040, value);
            }
            0x1f80_1050..=0x1f80_105f => {
                // SIO: TODO
            }
            0x1f80_1080..=0x1f80_10f4 => {
                self.dma.write::<S>(addr - 0x1f80_1080, value);
                self.handle_dma_write();
            }
            0x1f80_1100..=0x1f80_112f => {
                self.timers
                    .write::<S>(addr - 0x1f80_1100, value, self.total_cycles);
            }
            0x1f80_1800..=0x1f80_1803 => {
                let cmd = self.cdrom.write::<S>(addr - 0x1f80_1800, value);

                match cmd {
                    CpuCommand::EnqueueEvent(evt, ft, ra) => {
                        self.add_event(evt, ft, ra);
                    }
                    _ => {}
                }
            }
            0x1f80_1810..=0x1f80_1814 => {
                self.gpu.write::<S>(addr - 0x1f80_1810, value);
            }
            0x1f80_1820..=0x1f80_1824 => {
                // MDEC: TODO
            }
            0x1f80_1c00..=0x1f80_1fff => {
                self.spu.write::<S>(addr - 0x1f80_1c00, value);
            }
            0x1f80_2000..=0x1f80_207f => {
                // EXP2: ignore
                // However at 2041, there's the POST 7seg display
            }
            0x1f80_1000..=0x1f80_1020 | 0x1f80_1060 => {
                self.write_io::<S>(addr & 0xffff, value);
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

    fn handle_dma_write(&mut self) {
        if let Some(active_channel) = self.dma.active_channel() {
            let step = active_channel.step();
            let mut addr = active_channel.base();

            let (blocks, block_size) = active_channel.transfer_size();

            match active_channel.sync_mode() {
                SyncMode::Immediate => match active_channel.link() {
                    ChannelLink::Otc => {
                        let mut remaining_words = block_size;
                        // println!("[DMA6] OTC -> RAM @ 0x{:08x}, block, count: 0x{:04x}\n", addr,
                        // remaining_words);
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
                                    self.ram.write::<4>(addr, word);
                                }
                            }
                            addr = addr.wrapping_add(step as u32) & 0x1f_fffc;
                            remaining_words -= 1;
                        }
                        active_channel.done();
                        // if let Some(d) = &self.debug_tx {
                        //     d.send(true);
                        // }
                    }
                    ChannelLink::Cdrom => {
                        let mut remaining_words = block_size * blocks;
                        while remaining_words > 0 {
                            match active_channel.direction() {
                                Direction::ToRam => {
                                    let value = self.cdrom.read::<1>(2).0
                                        | self.cdrom.read::<1>(2).0 << 8
                                        | self.cdrom.read::<1>(2).0 << 16
                                        | self.cdrom.read::<1>(2).0 << 24;
                                    self.ram.write::<4>(addr, value);
                                    addr = addr.wrapping_add(4);
                                    remaining_words -= 1;
                                }
                                Direction::FromRam => {
                                    panic!("Writing to CDROM? Not happening");
                                }
                            }
                        }
                        active_channel.done();
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
                                        let header = self.ram.read::<4>(addr);
                                        let word_count = header >> 24;

                                        // if word_count > 0 {
                                        //     println!("[DMA2] GPU <- RAM @ 0x{:08x}, count: {},
                                        // nextAddr: 0x{:08x}",
                                        //     addr, word_count, header);
                                        // }

                                        for _ in 0..word_count {
                                            addr = addr.wrapping_add(step as u32);
                                            let cmd = self.ram.read::<4>(addr);
                                            self.gpu.process_gp0(cmd);
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
                                    let value = self.ram.read::<4>(addr);
                                    self.gpu.process_gp0(value);
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
}
