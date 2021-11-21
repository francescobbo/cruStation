use crate::hw::bus::{Bus, BusDevice};
use std::cell::RefCell;
use std::rc::Weak;

use bitfield::bitfield;

bitfield! {
    struct CounterStatus(u32);
    impl Debug;

    pub synchronization_enable, _: 0;
    pub synchronization_mode, _: 2, 1;
    pub reset_at_target, _: 3;
    pub irq_at_target, _: 4;
    pub irq_at_wrap, _: 5;
    pub repeat_mode, _: 6;
    pub pulse_mode, _: 7;
    pub clock_source, _: 9, 8;
    pub irq_pulse, _: 10;
    pub reached_target, set_reached_target: 11;
    pub reached_wrap, set_reached_wrap: 12;
}

struct Timer {
    n: u32,
    current: u16,
    target: u16,
    status: CounterStatus,
    last_update_cycles: u64,

    bus: Weak<RefCell<Bus>>,
}

impl Timer {
    pub fn new(n: u32) -> Timer {
        Timer {
            n,
            current: 0,
            target: 0,
            status: CounterStatus(0x400),
            last_update_cycles: 0,

            bus: Weak::new(),
        }
    }

    pub fn write_current_value(&mut self, value: u16) {
        self.current = value;

        self.refresh_cycles();

        //println!("Wrote {:08x} value to tmr{}", value, self.n);
    }

    pub fn write_status(&mut self, mut value: u32) {
        // Can only set bits 0-9
        value &= 0x3ff;

        // Bit 10 is always set on writing
        value |= 1 << 10;

        self.status.0 = (self.status.0 & !0x3ff) | value;

        // Reset current value on status writes
        self.current = 0;
        self.refresh_cycles();
        //println!("Wrote {:08x} mode to tmr{} ({:?})", self.status.0, self.n, self.status);
    }

    pub fn write_target(&mut self, value: u16) {
        self.target = value;
        //println!("Wrote {:08x} target to tmr{}", value, self.n);
    }

    pub fn get_current_value(&mut self) -> u16 {
        let previous_cycles = self.refresh_cycles();

        // Thank you modular arithmetic
        let delta = (self.last_update_cycles - previous_cycles) as u16;

        let divider = match self.n {
            0 => 1.0,
            1 => match self.status.clock_source() {
                0 | 2 => 1.0,
                1 | 3 => 1.0 / 2200.0, // 15840Hz average of PAL and NTSC
                _ => unreachable!(),
            },
            2 => 1.0,
            _ => unreachable!(),
        };

        let delta = ((delta as f32) * divider) as u16;

        let (new_value, overflown) = self.current.overflowing_add(delta);
        self.current = new_value;

        if overflown && !self.status.reached_wrap() {
            self.status.set_reached_wrap(true);
        }

        self.current
    }

    fn refresh_cycles(&mut self) -> u64 {
        let old = self.last_update_cycles;
        self.last_update_cycles = *self.bus.upgrade().unwrap().borrow().total_cycles.borrow();

        old
    }
}

pub struct Timers {
    timers: [Timer; 3],
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            timers: [Timer::new(0), Timer::new(1), Timer::new(2)],
        }
    }

    pub fn link(&mut self, bus: Weak<RefCell<Bus>>) {
        self.timers[0].bus = bus.clone();
        self.timers[1].bus = bus.clone();
        self.timers[2].bus = bus;
    }
}

impl BusDevice for Timers {
    fn read<const S: u32>(&mut self, addr: u32) -> u32 {
        let n = (addr >> 4) as usize;

        if n > 2 {
            //println!("[TMR] Invalid read at non-existing timer {}", n);
            return 0;
        }

        let timer = &mut self.timers[n];
        let val = match addr & 0xf {
            0x0 => timer.get_current_value() as u32,
            0x4 => {
                let value = timer.status.0;

                // Bits 11/12 are cleared upon read
                timer.status.set_reached_target(false);
                timer.status.set_reached_wrap(false);

                value
            }
            0x8 => timer.target as u32,
            _ => {
                //println!("[TMR] Invalid access to register {:x} on timer {}", addr & 0xf, n);
                0
            }
        };

        //// println!("[TMR] Timer {} field {:x} read: {:x}", n, addr & 0xf, val);
        val
    }

    fn write<const S: u32>(&mut self, addr: u32, value: u32) {
        let n = (addr >> 4) as usize;

        if n > 2 {
            //println!("[TMR] Invalid write at non-existing timer {}", n);
            return;
        }

        let timer = &mut self.timers[n];
        match addr & 0xf {
            0x0 => {
                timer.write_current_value(value as u16);
            }
            0x4 => {
                timer.write_status(value);
            }
            0x8 => {
                timer.write_target(value as u16);
            }
            _ => {
                //println!("Invalid access to register {:x} on timer {}", addr & 0xf, n);
            }
        }
    }
}
