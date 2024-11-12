use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rustyline::Editor;
// use std::process;

// use crate::hw::cpu::{Cpu, PsxBus};
// use crate::hw::disasm::Disasm;

// fn die() {
//     println!("Quitting...");
//     process::exit(-1);
// }

pub struct Debugger {
    readline: Editor<()>,
    breakpoints: Vec<u32>,
    pub stepping: bool,
    next_breakpoint: u32,
    last_line: String,

    pub triggered: Arc<AtomicBool>,
}

fn effective_pc(cpu: &crate::Cpu) -> u32 {
    if let Some((branch_delay, _)) = cpu.branch_delay_slot {
        branch_delay
    } else {
        cpu.pc
    }
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {
            readline: Editor::<()>::new(),
            breakpoints: vec![],
            stepping: false,
            next_breakpoint: 0xffff_ffff,
            last_line: String::from(""),

            triggered: Arc::new(AtomicBool::new(false)),
        }
    }

//     pub fn enter<T: PsxBus>(cpu: &mut Cpu<T>) {
//         cpu.debugger.stepping = false;
//         cpu.debugger.next_breakpoint = 0xffff_ffff;

//         let pc = effective_pc(cpu);
//         let instruction = cpu.load::<u32>(pc);
//         let disasm = Disasm::disasm(instruction, pc);
//         println!(
//             "[{:08x}] {} ({:08x})",
//             effective_pc(cpu),
//             disasm,
//             instruction
//         );

//         loop {
//             match cpu.debugger.readline.readline("psx> ") {
//                 Ok(line) => {
//                     let mut line = line.trim().to_string();

//                     if line.is_empty() {
//                         line = cpu.debugger.last_line.clone();
//                     } else {
//                         cpu.debugger.last_line = line.clone();
//                         cpu.debugger.readline.add_history_entry(line.as_str());
//                     }

//                     if Debugger::run_debug_command(line, cpu) {
//                         break;
//                     }
//                 }
//                 Err(ReadlineError::Interrupted) => {
//                     die();
//                 }
//                 Err(err) => {
//                     println!("Error: {:?}", err);
//                 }
//             }
//         }
//     }

    pub fn should_break(&self) -> bool {
        // let pc = effective_pc(cpu);
        // cpu.debugger.stepping
            // || cpu.debugger.next_breakpoint == pc
            // || cpu.debugger.breakpoints.contains(&pc)

        self.triggered.load(Ordering::Relaxed)
    }

    pub fn trigger(&self) {
        self.triggered.store(true, Ordering::Relaxed);
    }

//     fn run_debug_command<T: PsxBus>(line: String, cpu: &mut Cpu<T>) -> bool {
//         let mut parsed = line.split_whitespace();
//         let command = parsed.next().unwrap_or("");

//         match command {
//             "q" | "quit" => {
//                 die();
//                 false
//             }
//             "c" | "continue" => true,
//             "s" | "step" => {
//                 cpu.debugger.stepping = true;
//                 true
//             }
//             "n" | "next" => {
//                 let pc = effective_pc(cpu);
//                 let is_call = Disasm::is_function_call(cpu.load::<u32>(pc));
//                 if is_call {
//                     cpu.debugger.next_breakpoint = pc.wrapping_add(8)
//                 } else {
//                     cpu.debugger.stepping = true
//                 }

//                 true
//             }
//             "r" | "regs" => {
//                 println!("PC:  {:08x}", effective_pc(cpu));
//                 for i in 0..8 {
//                     for j in 0..4 {
//                         let idx = j * 8 + i;
//                         print!(
//                             "{}: {:08x}  ",
//                             Disasm::reg_name(idx),
//                             cpu.regs[idx as usize]
//                         );
//                     }
//                     println!();
//                 }
//                 println!("HI:  {:08x}  LO:  {:08x}", cpu.hi, cpu.lo);

//                 false
//             }
//             "rm" | "read-mem" => {
//                 let address = parsed.next().unwrap_or("");
//                 if let Ok(address) = u32::from_str_radix(address.trim_start_matches("0x"), 16) {
//                     let value = cpu.load::<u32>(address);
//                     println!("Read at {:08x}: {:08x}", address, value);
//                 } else {
//                     println!("Usage: read-mem [address] (eg: read-mem bfc01234)")
//                 }

//                 false
//             }
//             "b" | "breakpoint" => {
//                 let address = parsed.next().unwrap_or("");
//                 if let Ok(address) = u32::from_str_radix(address.trim_start_matches("0x"), 16) {
//                     cpu.debugger.breakpoints.push(address);
//                 } else {
//                     println!("Usage: breakpoint [address] (eg: breakpoint bfc01234)")
//                 }

//                 false
//             }
//             "lb" | "list-breakpoints" => {
//                 if !cpu.debugger.breakpoints.is_empty() {
//                     for i in 0..cpu.debugger.breakpoints.len() {
//                         println!("{}: {:08x}", i, cpu.debugger.breakpoints[i]);
//                     }
//                 } else {
//                     println!("No breakpoint has been set.");
//                 }

//                 false
//             }
//             "db" | "delete-breakpoint" => {
//                 let index = parsed.next().unwrap_or("");
//                 if let Ok(index) = index.parse::<usize>() {
//                     if index < cpu.debugger.breakpoints.len() {
//                         cpu.debugger.breakpoints.remove(index);
//                     } else {
//                         println!("There's no breakpoint #{}", index);
//                     }
//                 } else {
//                     println!("Usage: delete-breakpoint [index] (eg: delete-breakpoint 0)");
//                 }

//                 false
//             }
//             "h" | "help" => {
//                 println!("  h, help                      Shows this message");
//                 println!("  c, continue                  Resumes emulation");
//                 println!("  s, step                      Run 1 instruction and break again");
//                 println!("  n, next                      Like step, but skipping function calls");
//                 println!("  r, regs                      Dumps Cpu registers");
//                 println!(" rm, read-mem                  Reads a bus address");
//                 println!("  b, breakpoint [address]      Sets a breakpoint");
//                 println!(" lb, list-breakpoints          Lists all breakpoints");
//                 println!(" db, delete-breakpoint [index] Deletes a breakpoints");
//                 println!("  q, quit                      Terminates the emulator");
//                 false
//             }
//             _ => {
//                 println!(
//                     "Unknown debugger command {}. Type <help> for help.",
//                     command
//                 );
//                 false
//             }
//         }
//     }
}
