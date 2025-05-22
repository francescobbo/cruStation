use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub struct Spu {
    io_space: Vec<u8>,
    memory: Vec<u8>,

    spucnt: u16,

    manual_destination: usize,
}

impl Spu {
    pub fn new() -> Spu {
        Spu {
            io_space: vec![0; 1024],
            memory: vec![0; 512 * 1024], // 512Kb of SPU memory

            spucnt: 0,

            manual_destination: 0,
        }
    }

    pub fn write<const S: u32>(&mut self, addr: u32, value: u32) {
        let addr = addr as usize;
        let mut bytes = &mut self.io_space[addr..addr + 4];
        bytes.write_u32::<LittleEndian>(value).unwrap();

        match addr + 0x1f80_1c00 {
            0x1f80_1da6 => {
                self.manual_destination = (value & 0xFFFF) as usize * 8;
            }
            0x1f80_1da8 => {
                let mut bytes =
                    &mut self.memory[self.manual_destination..self.manual_destination + 2];
                bytes.write_u16::<LittleEndian>(value as u16).unwrap();

                // println!("[SPU] Memwrite: {:#06x} -> {:#04x}", self.manual_destination, value
                // as u16);

                self.manual_destination += 2;
                self.manual_destination %= self.memory.len();
            }
            0x1f80_1daa => {
                // SPUCNT
                // println!("[SPUCNT] Write: {:#08x}", value);
                self.spucnt = (value & 0xFFFF) as u16;
            }
            _ => {} //println!("[SPU] Write: {:#08x} -> {:#08x}", addr + 0x1f80_1c00, value)
        }
    }

    pub fn read<const S: u32>(&self, addr: u32) -> u32 {
        let addr = addr as usize;
        let mut bytes = &self.io_space[addr..addr + 4];
        let mut val = bytes.read_u32::<LittleEndian>().unwrap();

        val = match addr + 0x1f80_1c00 {
            0x1f80_1daa => {
                // SPUCNT
                self.spucnt as u32
            }
            0x1f80_1dae => {
                // SPUSTAT
                (self.spucnt as u32) & 0x3f
            }
            _ => val,
        };

        // println!("[SPU] Read: {:#08x} -> {:#08x}", addr + 0x1F801C00, val);
        val
    }
}
