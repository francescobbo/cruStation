mod emulator;
mod executable;
mod hw;
mod system;

use crustationcpu::DEBUG_BREAK_REQUESTED;

fn main() {
    let mut emulator = emulator::Emulator::new();
    emulator.load_rom("bios/SCPH1001.BIN");

    ctrlc::set_handler(move || {
        DEBUG_BREAK_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    emulator.run();
}
