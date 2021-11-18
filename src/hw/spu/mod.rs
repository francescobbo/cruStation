use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::hw::bus::R3000Type;

pub struct Spu {
    io_space: Vec<u8>,
}

impl Spu {
    pub fn new() -> Spu {
        Spu {
            io_space: vec![0; 1024],
        }
    }

    pub fn write<T: R3000Type>(&mut self, addr: u32, value: u32) {
        let addr = addr as usize;
        let mut bytes = &mut self.io_space[addr..addr + 4];
        bytes.write_u32::<LittleEndian>(value).unwrap();
    }

    pub fn read<T: R3000Type>(&self, addr: u32) -> u32 {
        let addr = addr as usize;
        let mut bytes = &self.io_space[addr..addr + 4];
        bytes.read_u32::<LittleEndian>().unwrap()
    }
}
