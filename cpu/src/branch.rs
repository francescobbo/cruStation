use crate::{Cpu, PsxBus};

impl<T: PsxBus> Cpu<T> {
    #[inline(always)]
    pub fn ins_j(&mut self) {
        let target = self.pc & 0xf000_0000;
        let target = target + (self.current_instruction.imm26() << 2);
        self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));

        self.pc = target;
    }

    #[inline(always)]
    pub fn ins_jal(&mut self) {
        let target = self.pc & 0xf000_0000;
        let target = target + (self.current_instruction.imm26() << 2);
        self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));

        self.write_reg(31, self.pc.wrapping_add(4));
        self.pc = target;
    }

    #[inline(always)]
    pub fn ins_jr(&mut self) {
        self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));
        self.pc = self.r_rs();
    }

    #[inline(always)]
    pub fn ins_jalr(&mut self) {
        // must happen in this order to handle jalr r31, r31
        let ret = self.pc.wrapping_add(4);

        self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));

        self.pc = self.r_rs();
        self.write_reg(self.current_instruction.rd(), ret);
    }

    #[inline(always)]
    pub fn ins_bcondz(&mut self) {
        let is_bgez = (self.current_instruction.0 >> 16) & 1;
        let and_link = (self.current_instruction.0 >> 17) & 0xf == 8;

        let test = ((self.r_rs() as i32) < 0) as u32;
        let test = test ^ is_bgez;

        if and_link {
            self.regs[31] = self.pc.wrapping_add(4);
        }

        if test != 0 {
            self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));
            let delta = (self.current_instruction.simm16() as i32) << 2;

            self.pc = self.pc.wrapping_add(delta as u32);
        }
    }

    #[inline(always)]
    pub fn ins_beq(&mut self) {
        let value = self
            .pc
            .wrapping_add((self.current_instruction.simm16() << 2) as u32);

        if self.r_rs() == self.r_rt() {
            self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));
            self.pc = value;
        }
    }

    #[inline(always)]
    pub fn ins_bne(&mut self) {
        let value = self
            .pc
            .wrapping_add((self.current_instruction.simm16() << 2) as u32);

        if self.r_rs() != self.r_rt() {
            self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));
            self.pc = value;
        }
    }

    #[inline(always)]
    pub fn ins_blez(&mut self) {
        let value = self
            .pc
            .wrapping_add((self.current_instruction.simm16() << 2) as u32);

        if self.r_rs() as i32 <= 0 {
            self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));
            self.pc = value;
        }
    }

    #[inline(always)]
    pub fn ins_bgtz(&mut self) {
        let value = self
            .pc
            .wrapping_add((self.current_instruction.simm16() << 2) as u32);

        if self.r_rs() as i32 > 0 {
            self.branch_delay_slot = Some((self.pc, self.fetch_at_pc()));
            self.pc = value;
        }
    }
}
