mod debug;
mod hw;

use std::sync::atomic::Ordering;
use std::thread;

pub use hw::Cpu;
pub use hw::Ram;
use log::info;

fn main() {
    env_logger::init();
    info!("Emulator starting...");

    let mut gui = pollster::block_on(crustationgui::EmuGui::new());

    thread::spawn(move || {
        let mut cpu = Cpu::new(gui.2);

        let breakpoint = cpu.debugger.triggered.clone();
        ctrlc::set_handler(move || {
            println!("Ctrl-C pressed, stopping execution...");
            breakpoint.store(true, Ordering::Relaxed);
        })
        .expect("Error setting Ctrl-C handler");

        cpu.bus.load_rom("bios/SCPH1001.BIN");

        cpu.debugger.stepping = true;

        let executable = std::env::args().nth(1);
        if let Some(exe) = executable {
            cpu.run_until(0x8003_0000);
            cpu.load_exe(&exe);
            println!("Loaded executable: {}", exe);
            cpu.debugger.stepping = true;
            cpu.run();
        } else {
            cpu.run();
        }
    });

    gui.0.run(gui.1);
}
