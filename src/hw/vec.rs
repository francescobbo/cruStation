use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::mem::size_of;

use crate::hw::bus::R3000Type;

pub trait ByteSerialized {
    fn read<T: R3000Type>(&self, addr: u32) -> u32;
    fn write<T: R3000Type>(&mut self, addr: u32, value: u32);
}

impl ByteSerialized for Vec<u8> {
    fn read<T: R3000Type>(&self, addr: u32) -> u32 {
        let addr = addr as usize;

        match size_of::<T>() {
            1 => self[addr] as u32,
            2 => {
                let mut bytes = &self[addr..addr + 2];
                bytes.read_u16::<LittleEndian>().unwrap() as u32
            }
            4 => {
                let mut bytes = &self[addr..addr + 4];
                bytes.read_u32::<LittleEndian>().unwrap()
            }
            _ => {
                unreachable!()
            }
        }
    }

    fn write<T: R3000Type>(&mut self, addr: u32, value: u32) {
        let addr = addr as usize;

        match size_of::<T>() {
            1 => {
                self[addr] = value as u8;
            }
            2 => {
                let mut bytes = &mut self[addr..addr + 2];
                bytes.write_u16::<LittleEndian>(value as u16).unwrap();
            }
            4 => {
                let mut bytes = &mut self[addr..addr + 4];
                bytes.write_u32::<LittleEndian>(value).unwrap();
            }
            _ => {
                unreachable!()
            }
        }
    }
}
