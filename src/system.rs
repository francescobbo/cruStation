use std::{cmp::Ordering, collections::BinaryHeap};

use crustationcpu::PsxBus;

use crate::hw::{bios::Bios, cdrom::Cdrom, dma::{ChannelLink, Direction, Dma, SyncMode}, gpu::Gpu, joy_mc::JoypadMemorycard, ram::Ram, spu::Spu, timers::Timers};

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

pub struct System {
    pub total_cycles: u64,

    pub ram: Ram,
    pub bios: Bios,
    pub io: Vec<u8>,
    pub cdrom: Cdrom,
    pub dma: Dma,
    pub spu: Spu,
    pub gpu: Gpu,
    pub timers: Timers,
    pub joy_mc: JoypadMemorycard,

    events: BinaryHeap<PsxEvent>,

    pub irq: u32,
}

impl System {
    pub fn new() -> System {
        System {
            total_cycles: 0,
            irq: 0,

            ram: Ram::new(),
            bios: Bios::new(),
            io: vec![0; 0x10000],
            cdrom: Cdrom::new(),
            dma: Dma::new(),
            spu: Spu::new(),
            gpu: Gpu::new(),
            timers: Timers::new(),
            joy_mc: JoypadMemorycard::new(),

            events: BinaryHeap::new(),
        }
    }

    pub fn strip_region(addr: u32) -> u32 {
        const MASK: [u32; 4] = [0x1fff_ffff, 0x1fff_ffff, 0x1fff_ffff, 0xffff_ffff];

        addr & MASK[(addr >> 30) as usize]
    }

    pub fn process_events(&mut self) {
        let total = self.total_cycles;

        while let Some(ev) = self.events.peek() {
            if ev.cycles_target < total {
                let ev = self.events.pop().unwrap();

                self.process_event(ev.kind);

                if ev.repeat > 0 {
                    self.events.push(PsxEvent {
                        kind: ev.kind,
                        repeat: ev.repeat,
                        cycles_target: total + ev.repeat,
                    });
                }
            } else {
                // The item at the head of the heap isn't ready to be processed
                // yet. So none of them are.
                break;
            }
        }
    }

    pub fn add_event(&mut self, kind: PsxEventType, mut first_target: u64, repeat_after: u64) {
        // If an event of the same type exists, remove it
        // TODO: retain is unstable API. Alternatives?
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
    //     // TODO: retain is unstable API. Alternatives?
    //     events.retain(|ev| ev.kind != kind);
    // }

    pub fn process_event(&mut self, kind: PsxEventType) {
        match kind {
            PsxEventType::DeliverCDRomResponse => {
                self.cdrom.next_response();
            }
            PsxEventType::VBlank => {
                self.gpu.vblank();
            }
        }
    }

    pub fn send_irq(&mut self, irq_num: u32) {
        if irq_num > 10 {
            panic!("[BUS] Invalid IRQ number");
        }

        self.irq |= 1 << irq_num;
    }

    pub fn write_io<const S: u32>(&self, addr: u32, value: u32) {
        // self.io.write::<S>(addr as u32, value);

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

    fn handle_dma_write(&mut self) {
        if let Some(active_channel) = self.dma.active_channel() {
            let step = active_channel.step();
            let mut addr = active_channel.base();

            let (blocks, block_size) = active_channel.transfer_size();

            match active_channel.sync_mode() {
                SyncMode::Immediate => match active_channel.link() {
                    ChannelLink::Otc => {
                        let mut remaining_words = block_size;
                        // println!("[DMA6] OTC -> RAM @ 0x{:08x}, block, count: 0x{:04x}\n", addr, remaining_words);
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
                        let cdrom = &mut self.cdrom;
                        while remaining_words > 0 {
                            match active_channel.direction() {
                                Direction::ToRam => {
                                    let value = cdrom.read::<1>(2) | cdrom.read::<1>(2) << 8 | cdrom.read::<1>(2) << 16 | cdrom.read::<1>(2) << 24;
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
                                        //     println!("[DMA2] GPU <- RAM @ 0x{:08x}, count: {}, nextAddr: 0x{:08x}",
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

impl PsxBus for System {
    fn update_cycles(&mut self, cycles: u64) {
        self.total_cycles += cycles;
        self.process_events();
    }

    fn read<const S: u32>(&mut self, addr: u32) -> u32 {
        let addr = System::strip_region(addr);

        match addr {
            0x0000_0000..=0x001f_ffff => {
                self.ram.read::<S>(addr)
            }
            0x1f00_0000..=0x1f7f_ffff => {
                0xffffffff
            }
            0x1f80_1040..=0x1f80_104f => {
                self.joy_mc.read::<S>(addr - 0x1f80_1040)
            }
            0x1f80_1050..=0x1f80_105f => {
                // SIO
                0
            }
            0x1f80_1060 => {
                // RAM SIZE
                0
            }
            0x1f80_1080..=0x1f80_10f4 => {
                self.dma.read::<S>(addr - 0x1f80_1080)
            }
            0x1f80_1100..=0x1f80_112f => {
                self.timers.read::<S>(self.total_cycles, addr - 0x1f80_1100)
            }
            0x1f80_1800..=0x1f80_1803 => {
                self.cdrom.read::<S>(addr - 0x1f80_1800)
            }
            0x1f80_1810..=0x1f80_1814 => {
                self.gpu.read::<S>(addr - 0x1f80_1810)
            }
            0x1f80_1820..=0x1f80_1824 => {
                // MDEC
                0
            }
            0x1f80_1c00..=0x1f80_1fff => {
                self.spu.read::<S>(addr - 0x1f80_1c00)
            }
            0x1f80_2000..=0x1f80_2080 => {
                // EXP2 has some weeeeeird timings
                // 10 cycles for 1 byte
                // 25 for 2 bytes
                // 55 for 4 bytes
                0xffffffff
            }
            0x1fa0_0000 => {
                // EXP3 is not sane either
                // 5 cycles for 1/2 bytes
                // 9 cycles for 4 bytes
                if S == 4 {
                } else {
                }

                0xffffffff
            }
            0x1fc0_0000..=0x1fc8_0000 => {
                self.bios.read::<S>(addr & 0xf_ffff)
            }
            _ => {
                panic!("Read in memory hole at {:08x}", addr);
            }
        }
    }

    fn write<const S: u32>(&mut self, addr: u32, value: u32) {
        match addr {
            0x0000_0000..=0x0020_0000 => {
                self.ram.write::<S>(addr, value);
            }
            0x1f80_1040..=0x1f80_104f => {
                self.joy_mc
                    .write::<S>(addr - 0x1f80_1040, value);
            }
            0x1f80_1050..=0x1f80_105f => {
                // SIO: TODO
            }
            0x1f80_1080..=0x1f80_10f4 => {
                self.dma.write::<S>(addr - 0x1f80_1080, value);
                // self.handle_dma_write();
            }
            0x1f80_1100..=0x1f80_112f => {
                self.timers.write::<S>(self.total_cycles, addr - 0x1f80_1100, value);
            }
            0x1f80_1800..=0x1f80_1803 => {
                self.cdrom
                    .write::<S>(addr - 0x1f80_1800, value);
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
}
