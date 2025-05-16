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

        let breakpoint = cpu.bus.debugger.triggered.clone();
        ctrlc::set_handler(move || {
            breakpoint.store(true, Ordering::Relaxed);
        })
        .expect("Error setting Ctrl-C handler");
    
        cpu.bus.load_rom("bios/SCPH1001.BIN");
    
        let executable = std::env::args().nth(1);
        if let Some(exe) = executable {
            cpu.run_until(0x8003_0000);
            cpu.load_exe(&exe);
            cpu.run();
        } else {
            cpu.run();
        }
    });

    gui.0.run(gui.1);
}
