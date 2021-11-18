use crate::hw::bus::{BusDevice, R3000Type};

#[derive(Copy, Clone, Debug)]
enum ControllerState {
    Initial,
    IdLow,
    IdHigh,
    ButtonsLow,
    ButtonsHigh,
    Analog0,
    Analog1,
    Analog2,
    Analog3,
}

pub struct JoypadMemorycard {
    state: ControllerState,
    joy_ctrl: u16,
    joy_stat: u32,

    tx_data: u8,
    rx_data: u8,

    txen: bool,
    // rxen: bool,
    current_joy: u16,
}

impl JoypadMemorycard {
    pub fn new() -> JoypadMemorycard {
        JoypadMemorycard {
            // cpu: Weak::new(),
            state: ControllerState::Initial,
            joy_ctrl: 0,
            joy_stat: 0,

            tx_data: 0,
            rx_data: 0,

            txen: false,
            // rxen: false,
            current_joy: 0,
        }
    }

    // pub fn install_cpu(&mut self, cpu: Weak<RefCell<Cpu>>) {
    //     self.cpu = cpu;
    // }
}

impl BusDevice for JoypadMemorycard {
    fn read<T: R3000Type>(&mut self, addr: u32) -> u32 {
        // println!("Read from reg {:04x}", addr);
        match addr {
            0x00 => self.rx_data as u32,
            0x0a => self.joy_ctrl as u32,
            _ => 0,
        }
    }

    fn write<T: R3000Type>(&mut self, addr: u32, value: u32) {
        // println!("Write to reg {:04x} {:08x}", addr, value);

        // Writes to JOY are truncated to 16 bits
        let value = value as u16;

        match addr {
            0x00 => {
                let data = value as u8;
                self.write_tx_data(data);
            }
            0x08 => {
                // TODO
            }
            0x0a => {
                self.write_joy_ctrl(value);
            }
            0x0c => {
                // TODO
            }
            _ => {
                unimplemented!("{}", addr);
            }
        }
    }
}

impl JoypadMemorycard {
    fn write_tx_data(&mut self, tx_data: u8) {
        self.tx_data = tx_data;
        if self.txen {
            self.process_tx_data();
        }
    }

    fn write_joy_ctrl(&mut self, value: u16) {
        // Clear forced-zero bits
        let value = value & !0xc080;

        if value & (1 << 6) != 0 {
            // panic!("JoyMc reset requested");
        }

        self.txen = value & 1 != 0;

        if value & (1 << 1) != 0 {
            self.current_joy = (value >> 13) & 1;

            self.state = ControllerState::IdLow;
        }

        if value & (1 << 4) != 0 {
            self.joy_stat &= !0x208;
            // println!("JoyMc ack");
        }

        // if value & (7 << 10) != 0 {
        //     panic!("JoyMc requested ack interrupt");
        // }

        self.joy_ctrl = value;

        if self.txen {
            self.process_tx_data();
        }
    }

    fn process_tx_data(&mut self) {
        match self.state {
            ControllerState::Initial => {
                match self.tx_data {
                    0 => {}
                    1 => {
                        // Started Joypad initialization
                        self.state = ControllerState::IdLow;
                    }
                    _ => {
                        panic!(
                            "Unhandled value {:02x} in state {:?}",
                            self.tx_data, self.state
                        )
                    }
                }
            }
            ControllerState::IdLow => {
                if self.tx_data == 0x42 {
                    self.rx_data = 0x41;
                    self.state = ControllerState::IdHigh;
                }
            }
            ControllerState::IdHigh => {
                self.rx_data = 0x5a;
                self.state = ControllerState::ButtonsLow;
            }
            ControllerState::ButtonsLow => {
                self.rx_data = 0xff;
                self.state = ControllerState::ButtonsHigh;
            }
            ControllerState::ButtonsHigh => {
                self.rx_data = 0xff;
                self.state = ControllerState::Analog0;
            }
            ControllerState::Analog0 => {
                self.rx_data = 0x80;
                self.state = ControllerState::Analog1;
            }
            ControllerState::Analog1 => {
                self.rx_data = 0x80;
                self.state = ControllerState::Analog2;
            }
            ControllerState::Analog2 => {
                self.rx_data = 0x80;
                self.state = ControllerState::Analog3;
            }
            ControllerState::Analog3 => {
                self.rx_data = 0x80;
                // self.state = ControllerState::Analog;
            }
        }
    }
}
