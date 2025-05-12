use std::fs::File;

use crate::{executable::PsxExeHeader, system::System};

pub struct Emulator {
    pub cpu: crustationcpu::Cpu,
    pub system: System,
}

impl Emulator {
    pub fn new() -> Emulator {
        Emulator {
            cpu: crustationcpu::Cpu::new(),
            system: System::new(),
        }
    }

    pub fn run(&mut self) {
        self.cpu.run(&mut self.system);
    }

    pub fn run_until(&mut self, target: u32) {
        self.cpu.run_until(&mut self.system, target);
    }

    pub fn load_rom(&mut self, path: &str) {
        let mut file = File::open(path).unwrap();
        self.system.bios.load(&mut file);
    }

    pub fn load_exe(&mut self, path: &str) {
        use std::io::BufReader;
        use std::io::Read;
        use std::io::Seek;
        use std::mem;

        let mut header = PsxExeHeader::default();
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);

        unsafe {
            let buffer: &mut [u8] = std::slice::from_raw_parts_mut(
                &mut header as *mut _ as *mut u8,
                mem::size_of::<PsxExeHeader>(),
            );

            reader.read_exact(buffer).unwrap();
        }

        reader.seek(std::io::SeekFrom::Start(0x800)).unwrap();
        let mut code = vec![0_u8; header.size as usize];
        reader.read_exact(&mut code).unwrap();

        let mut addr = header.destination & 0x1f_fffc;

        for b in code.iter() {
            self.system.ram.write::<1>(addr, *b as u32);
            addr = (addr + 1) & 0x3f_ffff;
        }

        self.cpu.pc = header.pc;
        self.cpu.regs[28] = header.r28;
        self.cpu.regs[29] = header.r29_base + header.r29_offset;

        if self.cpu.regs[29] == 0 {
            self.cpu.regs[29] = 0x801f_fff0;
        }
    }
}