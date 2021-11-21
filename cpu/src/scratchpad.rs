pub(crate) struct Scratchpad {
    data: Vec<u8>,
}

impl Scratchpad {
    pub fn new() -> Scratchpad {
        Scratchpad {
            data: vec![0; 0x400],
        }
    }

    pub fn read<const S: u32>(&self, addr: u32) -> u32 {
        let addr = addr as usize;

        match S {
            1 => self.data[addr] as u32,
            2 => (self.data[addr] as u32) | (self.data[addr + 1] as u32) << 8,
            4 => {
                (self.data[addr] as u32)
                    | (self.data[addr + 1] as u32) << 8
                    | (self.data[addr + 2] as u32) << 16
                    | (self.data[addr + 3] as u32) << 24
            }
            _ => unreachable!(),
        }
    }

    pub fn write<const S: u32>(&mut self, addr: u32, value: u32) {
        let addr = addr as usize;

        match S {
            1 => {
                self.data[addr] = value as u8;
            }
            2 => {
                self.data[addr] = value as u8;
                self.data[addr + 1] = (value >> 8) as u8;
            }
            4 => {
                self.data[addr] = value as u8;
                self.data[addr + 1] = (value >> 8) as u8;
                self.data[addr + 2] = (value >> 16) as u8;
                self.data[addr + 3] = (value >> 24) as u8;
            }
            _ => unreachable!(),
        }
    }
}
