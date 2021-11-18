use crate::hw::bus::{BusDevice, R3000Type};
use crate::hw::vec::ByteSerialized;

pub struct Scratchpad {
    memory: Vec<u8>,
}

impl Scratchpad {
    pub fn new() -> Scratchpad {
        Scratchpad {
            memory: vec![0; 1024],
        }
    }
}

impl BusDevice for Scratchpad {
    fn read<T: R3000Type>(&mut self, addr: u32) -> u32 {
        self.memory.read::<T>(addr)
    }

    fn write<T: R3000Type>(&mut self, addr: u32, value: u32) {
        self.memory.write::<T>(addr, value);
    }
}
