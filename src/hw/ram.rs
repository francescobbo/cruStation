use crate::hw::bus::{BusDevice};
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
    fn read<const S: u32>(&mut self, addr: u32) -> u32 {
        self.memory.read::<S>(addr)
    }

    fn write<const S: u32>(&mut self, addr: u32, value: u32) {
        self.memory.write::<S>(addr, value);
    }
}
