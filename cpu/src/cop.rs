use crate::{Cpu, Exception, PsxBus};

use crustationlogger::*;

impl<T: PsxBus> Cpu<T> {
    pub fn interrupt(&mut self) {
        debug!(self.logger, "Interrupt fired at {:08x}", self.pc);

        self.cop0
            .enter_exception(Exception::Interrupt, self.pc, self.in_delay, 0);

        self.pc = self.cop0.exception_handler(Exception::Interrupt);
    }

    pub fn exception(&mut self, cause: Exception) {
        debug!(
            self.logger,
            "Entering exception {:?} at {:08x}",
            cause,
            self.pc.wrapping_sub(4)
        );

        self.cop0
            .enter_exception(cause, self.pc.wrapping_sub(4), self.in_delay, 0);

        self.pc = self.cop0.exception_handler(cause);
    }

    pub fn coprocessor_exception(&mut self, cop_number: u32) {
        err!(
            self.logger,
            "Coprocessor Unusable Exception for COP{}",
            cop_number
        );

        self.cop0.enter_exception(
            Exception::CoprocessorUnusable,
            self.pc.wrapping_sub(4),
            self.in_delay,
            cop_number,
        );

        self.pc = self.cop0.exception_handler(Exception::CoprocessorUnusable);
    }

    #[inline(always)]
    pub fn ins_syscall(&mut self) {
        self.exception(Exception::Syscall);
    }

    #[inline(always)]
    pub fn ins_break(&mut self) {
        self.exception(Exception::Breakpoint);
    }

    pub fn ins_cop0(&mut self) {
        if self.current_instruction.0 & 0x2000000 != 0 {
            match self.cop0.execute(self.current_instruction.0 & 0x1ff_ffff) {
                Ok(_) => {}
                Err(exception) => {
                    self.exception(exception);
                }
            }
        } else {
            match (self.current_instruction.0 >> 21) & 0xf {
                0x00 => {
                    // MFC
                    match self.cop0.read_reg(self.current_instruction.rd()) {
                        Some(value) => {
                            self.write_reg(self.current_instruction.rt(), value);
                        }
                        None => self.coprocessor_exception(0),
                    }
                }
                0x02 => {
                    // CFC
                    self.coprocessor_exception(0);
                }
                0x04 => {
                    // MTC
                    let value = self.r_rt();
                    match self.cop0.write_reg(self.current_instruction.rd(), value) {
                        Ok(_) => {
                            self.check_interrupts();
                        }
                        Err(_) => self.coprocessor_exception(0),
                    }

                    if self.cop0.isolate_cache {
                        // Not accurate, however...
                        // every time the bios isolates the cache is to clear it
                        // Instead of accurately emulating I-cache writes, just
                        // help it.
                        self.icache.flush();
                    }
                }
                0x06 => {
                    // CTC
                    self.coprocessor_exception(0);
                }
                _ => {
                    self.exception(Exception::ReservedInstruction);
                }
            }
        }
    }

    pub fn ins_lwc0(&mut self) {
        if !self.cop0.cop0_enabled {
            self.coprocessor_exception(0);
        } else {
            warn!(self.logger, "lwc0 was used ({:08x})", self.pc);
        }
    }

    pub fn ins_swc0(&mut self) {
        if !self.cop0.cop0_enabled {
            self.coprocessor_exception(0);
        } else {
            warn!(self.logger, "swc0 was used ({:08x})", self.pc);
        }
    }

    pub fn ins_cop1(&mut self) {
        if !self.cop0.cop1_enabled {
            self.coprocessor_exception(1);
        } else {
            warn!(self.logger, "cop1 was used ({:08x})", self.pc);
        }
    }

    pub fn ins_lwc1(&mut self) {
        if !self.cop0.cop1_enabled {
            self.coprocessor_exception(1);
        } else {
            warn!(self.logger, "lwc1 was used ({:08x})", self.pc);
        }
    }

    pub fn ins_swc1(&mut self) {
        if !self.cop0.cop1_enabled {
            self.coprocessor_exception(1);
        } else {
            warn!(self.logger, "swc1 was used ({:08x})", self.pc);
        }
    }

    pub fn ins_cop2(&mut self) {
        if !self.cop0.cop2_enabled {
            self.coprocessor_exception(2);
            return;
        }

        let is_op = self.current_instruction.0 & (1 << 25) != 0;
        if is_op {
            self.gte.execute(self.current_instruction.0 & 0x1ff_ffff);
        } else {
            match (self.current_instruction.0 >> 21) & 0xf {
                0x00 => {
                    // mfc
                    let value = self.gte.read_reg(self.current_instruction.rd());
                    self.write_reg(self.current_instruction.rt(), value);
                }
                0x02 => {
                    // cfc
                    let value = self.gte.read_reg(self.current_instruction.rd() + 32);
                    self.write_reg(self.current_instruction.rt(), value);
                }
                0x04 => {
                    // mtc
                    self.gte
                        .write_reg(self.current_instruction.rd(), self.r_rt());
                }
                0x06 => {
                    // ctc
                    self.gte
                        .write_reg(self.current_instruction.rd() + 32, self.r_rt());
                }
                _ => {
                    err!(
                        self.logger,
                        "Invalid GTE opcode {:08x}",
                        self.current_instruction.0
                    );
                }
            }
        }
    }

    pub fn ins_lwc2(&mut self) {
        if !self.cop0.cop2_enabled {
            self.coprocessor_exception(2);
        }

        let address = self.ls_address();
        let value = self.load::<4>(address);

        self.gte.write_reg(self.current_instruction.rt(), value);
    }

    pub fn ins_swc2(&mut self) {
        if !self.cop0.cop2_enabled {
            self.coprocessor_exception(2);
        }

        let address = self.ls_address();
        let value = self.gte.read_reg(self.current_instruction.rt());
        self.store::<4>(address, value);
    }

    pub fn ins_cop3(&mut self) {
        if !self.cop0.cop3_enabled {
            self.coprocessor_exception(3);
        } else {
            warn!(self.logger, "cop3 was used ({:08x})", self.pc);
        }
    }

    pub fn ins_lwc3(&mut self) {
        if !self.cop0.cop3_enabled {
            self.coprocessor_exception(3);
        } else {
            warn!(self.logger, "lwc3 was used ({:08x})", self.pc);
        }
    }

    pub fn ins_swc3(&mut self) {
        if !self.cop0.cop3_enabled {
            self.coprocessor_exception(3);
        } else {
            warn!(self.logger, "swc3 was used ({:08x})", self.pc);
        }
    }
}
