mod hw;
mod debug;

use std::sync::atomic::Ordering;

pub use hw::Ram;
pub use hw::Cpu;

fn main() {
    let mut cpu = Cpu::new();

    let breakpoint = cpu.bus.debugger.triggered.clone();
    ctrlc::set_handler(move || {
        breakpoint.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    cpu.bus.load_rom("bios/PSXONPSP660.BIN");

    bus.load_rom("bios/SCPH1001.BIN");
    bus.link(bus_rc.clone());

    drop(bus);

    let bus = bus_rc.borrow();
    // let executable = std::env::args().nth(1);
    // if let Some(exe) = executable {
        // bus.run_until(0x8003_0000);
        // bus.load_exe(&exe);
        // bus.run();
    // } else {
        bus.run();
    // }
}
