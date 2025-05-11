mod hw;

use hw::bus::Bus;
use crustationcpu::CpuCommand;
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let bus_rc = Rc::new(RefCell::new(Bus::new()));
    let bus = bus_rc.borrow();

    let cpu_tx = bus.cpu_tx.clone();

    ctrlc::set_handler(move || {
        cpu_tx.send(CpuCommand::Break).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    bus.load_rom("bios/SCPH1001.BIN");
    bus.link(bus_rc.clone());

    drop(bus);

    let mut bus = bus_rc.borrow_mut();
    // let executable = std::env::args().nth(1);
    // if let Some(exe) = executable {
        // bus.run_until(0x8003_0000);
        // bus.load_exe(&exe);
        // bus.run();
    // } else {
        bus.run();
    // }
}
