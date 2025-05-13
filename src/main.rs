mod debug;
mod hw;

use std::sync::atomic::Ordering;

pub use hw::Cpu;
pub use hw::Ram;

fn main() {
    let mut cpu = Cpu::new();

    let breakpoint = cpu.bus.debugger.triggered.clone();
    ctrlc::set_handler(move || {
        breakpoint.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    cpu.bus.load_rom("bios/PSXONPSP660.BIN");

    // let executable = std::env::args().nth(1);
    // if let Some(exe) = executable {
        // bus.run_until(0x8003_0000);
        // bus.load_exe(&exe);
        // bus.run();
    // } else {
        cpu.run();
    // }
}
