#[derive(Copy, Clone)]
struct Entry {
    tag: u32,
    data: u32,
    valid: bool,
}

impl Entry {
    pub fn new() -> Entry {
        Entry {
            tag: 0,
            data: 0,
            valid: false,
        }
    }
}

pub struct InstructionCache {
    entries: Vec<Entry>,
}

impl InstructionCache {
    pub fn new() -> InstructionCache {
        InstructionCache {
            entries: vec![Entry::new(); 1024],
        }
    }

    pub fn load(&self, pc: u32) -> Option<u32> {
        // The most significant bit is ignored
        let pc = pc & !(1 << 31);

        let entry_number = ((pc >> 2) & 0x3ff) as usize;
        let entry = &self.entries[entry_number as usize];

        let tag = pc >> 12;

        if entry.tag == tag && entry.valid {
            Some(entry.data)
        } else {
            None
        }
    }

    pub fn store(&mut self, pc: u32, value: u32) {
        // The most significant bit is ignored
        let pc = pc & !(1 << 31);

        let entry_number = ((pc >> 2) & 0x3ff) as usize;
        let entry = &mut self.entries[entry_number as usize];

        let tag = pc >> 12;

        entry.tag = tag;
        entry.valid = true;
        entry.data = value;
    }

    pub fn flush(&mut self) {
        for entry in &mut self.entries {
            entry.valid = false;
        }
    }
}
