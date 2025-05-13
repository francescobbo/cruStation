use crate::hw::bus::{Bus, BusDevice, PsxEventType};
use crate::Cpu;
use bitfield::bitfield;
use ringbuffer::{AllocRingBuffer, RingBuffer, RingBufferExt, RingBufferRead, RingBufferWrite};

use std::cell::RefCell;
use std::rc::Weak;

use super::bus::CpuCommand;

bitfield! {
    struct ControllerStatus(u8);
    impl Debug;

    /// Index into the other registers
    pub index, set_index: 1, 0;

    pub adp_fifo_nonempty, _: 2;
    pub parameter_fifo_empty, set_parameter_fifo_empty: 3;
    pub parameter_fifo_writeable, set_parameter_fifo_writeable: 4;
    pub response_ready, set_response_ready: 5;
    pub data_fifo_notempty, _: 6;
    pub busy, _: 7;
}

bitfield! {
    struct Stat(u8);
    impl Debug;

    /// Invalid Command / parameters (followed by error)
    pub error, _: 0;
    /// 0 = Motor off, or in spin-up phase, 1 = Motor on
    pub motor, _: 1;
    /// Seek error, followed by error
    pub seek_error, _: 2;
    /// GetID failed
    pub id_error, _: 3;
    /// Shell is open or _was open_ (is true the first time it's read, then false if the shell got closed)
    pub shel_open, _: 4;

    /// Only one of reading, seeking and playing can be 1 at any point in time
    pub reading, _: 5;
    pub seeking, _: 6;
    pub playing, _: 7;
}

struct Interrupt {
    number: u32,
    data: Vec<u8>,
    acknowledged: bool,
}

pub struct Cdrom {
    controller_status: ControllerStatus,
    stat: Stat,

    parameters: AllocRingBuffer<u8>,
    pending_irqs: AllocRingBuffer<Interrupt>,
    interrupt_enable: u8,
}

// When reading from the CDROM controller, reads of sizes larger than 1 byte are
// copied to the remaining bytes
fn grow_to<const S: u32>(value: u8) -> u32 {
    let value = value as u32;

    match S {
        1 => value,
        2 => value | (value << 8),
        4 => value | (value << 8) | (value << 16) | (value << 24),
        _ => unreachable!(),
    }
}

impl Cdrom {
    pub fn new() -> Cdrom {
        Cdrom {
            controller_status: ControllerStatus(0),
            stat: Stat(0),

            parameters: AllocRingBuffer::with_capacity(16),
            pending_irqs: AllocRingBuffer::with_capacity(16),
            interrupt_enable: 0,
        }
    }

    pub fn read<const S: u32>(&mut self, addr: u32) -> (u32, CpuCommand) {
        print!("[CDR] Read {:04x}: ", addr);

        let mut command = CpuCommand::Noop;

        let val = match addr {
            0 => {
                self.controller_status
                    .set_parameter_fifo_empty(self.parameters.is_empty());
                self.controller_status
                    .set_parameter_fifo_writeable(self.parameters.is_full());
                self.controller_status.set_response_ready(
                    !self.pending_irqs.is_full(), /* && self.pending_irqs[0].response.len() > 0; */
                );

                self.controller_status.0
            }
            1 => {
                if let Some(irq) = self.pending_irqs.front_mut() {
                    if !irq.data.is_empty() {
                        let value = irq.data.remove(0);
                        if irq.data.is_empty() && irq.acknowledged {
                            self.pending_irqs.dequeue();
                            if !self.pending_irqs.is_empty() {
                                command = CpuCommand::EnqueueEvent(
                                    PsxEventType::DeliverCDRomResponse,
                                    50000,
                                    0,
                                );
                            }
                        }

                        value
                    } else {
                        0
                    }
                } else {
                    println!("[CDR] Tried to read response when none was available");
                    0
                }
                // TODO: When reading further bytes: The buffer is padded with
                // 00h's to the end of the 16-bytes, and does then restart at
                // the first response byte (that, without receiving a new
                // response, so it'll always return the same 16 bytes, until a
                // new command/response has been sent/received).
            }
            2 => {
                // println!("[CDR] Trying to read cd data");
                0
            }
            3 => {
                match self.controller_status.index() & 1 {
                    0 => {
                        println!("[CDR] Read Int enable");
                        /* fixed bits | writeable bits */
                        0xe0 | 0x1f
                    }
                    1 => {
                        println!("[CDR] Read Int flag");
                        if let Some(int) = self.pending_irqs.front() {
                            // TODO only set this after the IRQ has actually been delivered
                            0xe0 | ((int.number as u8) & 7)
                        } else {
                            0xe0
                        }
                    }
                    _ => unreachable!(),
                }
            }
            _ => panic!("[CDR] Invalid addr"),
        };

        // println!("{:02x}: ", val);
        (grow_to::<S>(val), command)
    }

    pub fn write<const S: u32>(&mut self, addr: u32, value: u32) -> CpuCommand {
        println!(
            "[CDR] Write to reg {:04x} {:08x} of size {}",
            addr, value, S
        );

        if S != 1 {
            // println!("[CDR] Invalid write");
        }

        let value = value as u8;

        match addr {
            0 => {
                self.controller_status.set_index(value & 3);
                CpuCommand::Noop
            }
            1 => {
                match self.controller_status.index() {
                    0 => self.handle_command(value),
                    1 => {
                        // sound map data out
                        println!("[CDR] Wrote sound map data {:02x}", value);
                        CpuCommand::Noop
                    }
                    2 => {
                        // sound map coding info
                        println!("[CDR] Wrote sound coding {:02x}", value);
                        CpuCommand::Noop
                    }
                    3 => {
                        // Audio Volume for Right-CD-Out to Right-SPU-Input
                        println!("[CDR] Wrote audio vol r-to-r {:02x}", value);
                        CpuCommand::Noop
                    }
                    _ => unreachable!(),
                }
            }
            2 => {
                match self.controller_status.index() {
                    0 => {
                        self.parameters.push(value);
                    }
                    1 => {
                        // Interrupt Enable Register
                        println!("[CDR] Wrote int enable {:02x}", value);
                        self.interrupt_enable = value;
                    }
                    2 => {
                        // Audio Volume for Left-CD-Out to Left-SPU-Input
                        println!("[CDR] Wrote audio vol l-to-l {:02x}", value);
                    }
                    3 => {
                        // Audio Volume for Right-CD-Out to Left-SPU-Input
                        println!("[CDR] Wrote audio vol r-to-l {:02x}", value);
                    }
                    _ => unreachable!(),
                }

                CpuCommand::Noop
            }
            3 => {
                match self.controller_status.index() {
                    0 => {
                        // Request Register
                        println!("[CDR] Wrote request {:02x}", value);
                    }
                    1 => {
                        // Interrupt Flag Register
                        println!("[CDR] Wrote interrupt flag {:02x}", value);

                        if value & 0x40 != 0 {
                            self.parameters.clear();
                        }

                        if let Some(irq) = self.pending_irqs.front_mut() {
                            irq.acknowledged = true;

                            if irq.data.is_empty() {
                                self.pending_irqs.dequeue();
                                if !self.pending_irqs.is_empty() {
                                    return CpuCommand::EnqueueEvent(
                                        PsxEventType::DeliverCDRomResponse,
                                        50000,
                                        0,
                                    );
                                }
                            }
                        }
                    }
                    2 => {
                        // Audio Volume for Left-CD-Out to Right-SPU-Input
                        println!("[CDR] Wrote audio vol l-to-r {:02x}", value);
                    }
                    3 => {
                        // Interrupt Flag Register (mirror)
                        println!("[CDR] Wrote ifr mirror {:02x}", value);
                    }
                    _ => unreachable!(),
                }

                CpuCommand::Noop
            }
            _ => panic!("[CDR] Invalid addr"),
        }
    }
}

impl Cdrom {
    fn handle_command(&mut self, command: u8) -> CpuCommand {
        match command {
            0x01 => {
                println!("Started CDROM stat");
                self.enqueue_interrupt(3, &[self.stat.0])
            }
            0x02 => {
                println!("ReadN");
                self.enqueue_interrupt(3, &[self.stat.0])
            }
            0x06 => {
                println!("ReadN");
                self.enqueue_interrupt(3, &[0x20]);
                self.enqueue_interrupt(1, &[]);
                self.enqueue_interrupt(1, &[]);
                self.enqueue_interrupt(1, &[])
            }
            0x09 => {
                println!("Pause");
                self.enqueue_interrupt(3, &[self.stat.0]);
                self.enqueue_interrupt(2, &[self.stat.0])
            }
            0x0e => {
                println!("Set mode {:02x}", self.parameters.get(0).unwrap());
                self.enqueue_interrupt(3, &[self.stat.0])
            }
            0x15 => {
                self.enqueue_interrupt(3, &[self.stat.0]);
                self.enqueue_interrupt(2, &[self.stat.0])
            }
            0x19 => {
                self.command_test();
                CpuCommand::Noop
            }
            0x1a => {
                self.enqueue_interrupt(3, &[self.stat.0]);
                self.enqueue_interrupt(5, &[2, 0, 0x20, 0, b'S', b'C', b'E', b'A'])
            }
            _ => {
                panic!("[CDR] Cannot do {:02x}", command);
            }
        }
    }

    fn command_test(&mut self) {
        let subcommand = self.parameters.get(0).unwrap();

        match subcommand {
            0x20 => {
                println!("Started CDROM identify");
                self.enqueue_interrupt(3, &[0x94, 0x09, 0x19, 0xc0]);
            }
            _ => unimplemented!(),
        }
    }

    fn enqueue_interrupt(&mut self, irq: u32, response: &[u8]) -> CpuCommand {
        self.pending_irqs.push(Interrupt {
            number: irq,
            data: response.to_vec(),
            acknowledged: false,
        });

        CpuCommand::EnqueueEvent(PsxEventType::DeliverCDRomResponse, 50000, 0)
    }

    pub fn next_response(&mut self) -> CpuCommand {
        // let response = self.pending_irqs.get(0).unwrap();

        println!("Deliver CDROM response");
        CpuCommand::Irq(2)
    }
}
