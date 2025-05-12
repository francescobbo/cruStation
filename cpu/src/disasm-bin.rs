mod disasm;

use disasm::Disasm;
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} <file> <base address>", args[0]);
        return;
    }

    let file_path = &args[1];
    let base_address: u32 = match u32::from_str_radix(&args[2], 16) {
        Ok(addr) => addr,
        Err(_) => {
            println!("Invalid base address: {}", args[2]);
            return;
        }
    };

    // read the binary file
    let mut file = std::fs::File::open(file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    
    // convert the buffer to a slice of u32
    let mut data: Vec<u32> = Vec::new();
    for chunk in buffer.chunks_exact(4) {
        let value = u32::from_le_bytes(chunk.try_into().expect("Invalid chunk size"));
        data.push(value);
    }

    // disassemble the binary
    for (i, &value) in data.iter().enumerate() {
        let address = base_address + (i as u32 * 4);
        let disasm = Disasm::disasm(value, address);
        println!("{:08x}: {}", address, disasm);
    }
}