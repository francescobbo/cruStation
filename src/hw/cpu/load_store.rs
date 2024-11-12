use super::{Cpu, Exception, LoadDelaySlot};

impl Cpu {
    #[inline(always)]
    pub fn ls_address(&self) -> u32 {
        let imm = self.current_instruction.simm16() as u32;
        self.r_rs().wrapping_add(imm)
    }

    #[inline(always)]
    pub fn ins_lb(&mut self) {
        let value = self.load::<1>(self.ls_address()) as i8 as u32;

        self.delayed_load(self.current_instruction.rt(), value);
    }

    #[inline(always)]
    pub fn ins_lh(&mut self) {
        let address = self.ls_address();

        if address % 2 == 0 {
            let value = self.load::<2>(address) as i16 as u32;
            self.delayed_load(self.current_instruction.rt(), value);
        } else {
            self.exception(Exception::AddressErrorLoad);
        }
    }

    pub fn ins_lwl(&mut self) {
        let addr = self.ls_address();
        let cur_v = if self.load_delay_slot[0].register == self.current_instruction.rt() {
            self.load_delay_slot[0].value
        } else {
            self.r_rt()
        };

        let aligned_word = self.load::<4>(addr & !3);
        let v = match addr & 3 {
            0 => (cur_v & 0x00ffffff) | (aligned_word << 24),
            1 => (cur_v & 0x0000ffff) | (aligned_word << 16),
            2 => (cur_v & 0x000000ff) | (aligned_word << 8),
            3 => aligned_word,
            _ => unreachable!(),
        };

        self.delayed_load(self.current_instruction.rt(), v);
    }

    #[inline(always)]
    pub fn ins_lw(&mut self) {
        let address = self.ls_address();
        if address % 4 == 0 {
            let value = self.load::<4>(address);
            self.delayed_load(self.current_instruction.rt(), value);
        } else {
            self.exception(Exception::AddressErrorLoad);
        }
    }

    #[inline(always)]
    pub fn ins_lbu(&mut self) {
        let address = self.ls_address();
        let value = self.load::<1>(address) as u32;

        self.delayed_load(self.current_instruction.rt(), value);
    }

    #[inline(always)]
    pub fn ins_lhu(&mut self) {
        let address = self.ls_address();
        if address % 2 == 0 {
            let value = self.load::<2>(address) as u32;
            self.delayed_load(self.current_instruction.rt(), value);
        } else {
            self.exception(Exception::AddressErrorLoad);
        }
    }

    pub fn ins_lwr(&mut self) {
        let addr = self.ls_address();
        let cur_v = if self.load_delay_slot[0].register == self.current_instruction.rt() {
            self.load_delay_slot[0].value
        } else {
            self.r_rt()
        };

        let aligned_word = self.load::<4>(addr & !3);
        let v = match addr & 3 {
            0 => aligned_word,
            1 => (cur_v & 0xff000000) | (aligned_word >> 8),
            2 => (cur_v & 0xffff0000) | (aligned_word >> 16),
            3 => (cur_v & 0xffffff00) | (aligned_word >> 24),
            _ => unreachable!(),
        };

        self.delayed_load(self.current_instruction.rt(), v);
    }

    #[inline(always)]
    pub fn ins_sb(&mut self) {
        self.store::<1>(self.ls_address(), self.r_rt() & 0xff);
    }

    #[inline(always)]
    pub fn ins_sh(&mut self) {
        let address = self.ls_address();
        if address % 2 == 0 {
            self.store::<2>(address, self.r_rt() & 0xffff);
        } else {
            self.exception(Exception::AddressErrorStore);
        }
    }

    pub fn ins_swl(&mut self) {
        let addr = self.ls_address();
        let v = self.r_rt();
        let aligned_addr = addr & !3;
        let cur_v = self.load::<4>(aligned_addr);

        let v = match addr & 3 {
            0 => (cur_v & 0xffffff00) | (v >> 24),
            1 => (cur_v & 0xffff0000) | (v >> 16),
            2 => (cur_v & 0xff000000) | (v >> 8),
            3 => v,
            _ => unreachable!(),
        };

        self.store::<4>(aligned_addr, v);
    }

    #[inline(always)]
    pub fn ins_sw(&mut self) {
        let address = self.ls_address();

        if address % 4 == 0 {
            self.store::<4>(address, self.r_rt());
        } else {
            self.exception(Exception::AddressErrorStore);
        }
    }

    pub fn ins_swr(&mut self) {
        let addr = self.ls_address();
        let v = self.r_rt();
        let aligned_addr = addr & !3;
        let cur_v = self.load::<4>(aligned_addr);

        let v = match addr & 3 {
            0 => v,
            1 => (cur_v & 0x000000ff) | (v << 8),
            2 => (cur_v & 0x0000ffff) | (v << 16),
            3 => (cur_v & 0x00ffffff) | (v << 24),
            _ => unreachable!(),
        };

        self.store::<4>(aligned_addr, v)
    }

    #[inline(always)]
    fn delayed_load(&mut self, reg: u32, value: u32) {
        if reg == 0 {
            return;
        }

        if self.load_delay_slot[0].register == reg {
            self.load_delay_slot[0].register = 32;
        }

        self.load_delay_slot[1] = LoadDelaySlot {
            register: reg,
            value,
        };
    }

    pub fn load<const T: u32>(&mut self, address: u32) -> u32 {
        if self.cop0.isolate_cache {
            // TODO: not sure what to do here.
        }

        match address {
            0xfffe_0130 => {
                if T != 1 {
                    self.extra_cycles += 1;
                }
                self.biu_cc.0
            }
            0xfffe_0000..=0xfffe_012f | 0xfffe_0134..=0xfffe_013f => {
                /*
                 * TODO: 0 is reasonable for most locations.
                 * This does not match the hardware behaviour, but hopefully
                 * no game relies on this.
                 */
                0
            }
            _ => {
                let address = address & 0x1fff_ffff;
                match address {
                    0x1f80_0000..=0x1f80_03ff => self.dcache.read::<T>(address & 0x3ff),
                    0x1f80_1070 => {
                        self.extra_cycles += 2;
                        self.i_stat
                    }
                    0x1f80_1074 => {
                        self.extra_cycles += 2;
                        self.i_mask
                    }
                    _ => unsafe { 
                        self.bus.read::<T>(address)
                    },
                }
            }
        }
    }

    pub fn store<const T: u32>(&mut self, address: u32, value: u32) {
        if self.cop0.isolate_cache {
            return;
        }

        match address {
            0xfffe_0130 => {
                /*
                 * Writes to 0131, 0132, 0133 are ignored.
                 * Writes to 0130 of size 8 and 16 are zero-extended
                 *
                 * Bits 6 and 10 are fixed to 0.
                 * Bits 3 and 7 enable the scratchpad when both set.
                 * Bit 11 enables the i-cache.
                 */

                self.biu_cc.0 = value & !0x440;

                // TODO: if bit 11 is set enable i-cache
                // TODO: if bits 3 and 7 are set enable scratchpad
            }
            0xfffe_0000..=0xfffe_012f | 0xfffe_0134..=0xfffe_013f => {
                // Ignore writes to garbage locations
            }
            _ => {
                let address = address & 0x1fff_ffff;
                match address {
                    0x1f80_0000..=0x1f80_03ff => {
                        self.dcache.write::<T>(address & 0x3ff, value);
                    }
                    0x1f80_1070 => {
                        self.i_stat &= value;
                        self.check_interrupts();
                    }
                    0x1f80_1074 => {
                        self.i_mask = value & !0xf800;
                        self.check_interrupts();
                    }
                    _ => unsafe {
                        self.bus.write::<T>(address, value);
                    },
                }
            }
        }
    }
}
