use crate::hw::bus::{BusDevice, R3000Type};
use crate::hw::vec::ByteSerialized;

pub struct Ram {
    memory: Vec<u8>,
}

impl Ram {
    pub fn new() -> Ram {
        Ram {
            memory: vec![0; 2 * 1024 * 1024],
        }
    }
}

impl BusDevice for Ram {
    fn read<T: R3000Type>(&mut self, addr: u32) -> u32 {
        self.memory.read::<T>(addr)
    }

    fn write<T: R3000Type>(&mut self, addr: u32, value: u32) {
        if addr == 0x000589c0 {
            println!("[RAM] Writing {:08x} to {:08x}", value, addr);
        }
        self.memory.write::<T>(addr, value);
    }
}
