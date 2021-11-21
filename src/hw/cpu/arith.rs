use crate::hw::cpu::{Cpu, Exception, PsxBus};

impl<T: PsxBus> Cpu<T> {
    #[inline(always)]
    pub fn ins_sll(&mut self) {
        let value = self.current_instruction.imm5();
        self.write_reg(self.current_instruction.rd(), self.r_rt() << value);
    }

    #[inline(always)]
    pub fn ins_srl(&mut self) {
        let value = self.current_instruction.imm5();
        self.write_reg(self.current_instruction.rd(), self.r_rt() >> value);
    }

    #[inline(always)]
    pub fn ins_sra(&mut self) {
        let value = self.current_instruction.imm5();
        self.write_reg(
            self.current_instruction.rd(),
            ((self.r_rt() as i32) >> value) as u32,
        );
    }

    #[inline(always)]
    pub fn ins_sllv(&mut self) {
        self.write_reg(
            self.current_instruction.rd(),
            self.r_rt() << (self.r_rs() & 0x1f),
        );
    }

    #[inline(always)]
    pub fn ins_srlv(&mut self) {
        self.write_reg(
            self.current_instruction.rd(),
            self.r_rt() >> (self.r_rs() & 0x1f),
        );
    }

    #[inline(always)]
    pub fn ins_srav(&mut self) {
        self.write_reg(
            self.current_instruction.rd(),
            ((self.r_rt() as i32) >> (self.r_rs() & 0x1f)) as u32,
        );
    }

    #[inline(always)]
    pub fn ins_mult(&mut self) {
        let res = ((self.r_rs() as i32 as i64) * (self.r_rt() as i32 as i64)) as u64;
        self.hi = (res >> 32) as u32;
        self.lo = (res & 0xffff_ffff) as u32;
    }

    #[inline(always)]
    pub fn ins_multu(&mut self) {
        let res = (self.r_rs() as u64) * (self.r_rt() as u64);
        self.hi = (res >> 32) as u32;
        self.lo = (res & 0xffff_ffff) as u32;
    }

    #[inline(always)]
    pub fn ins_div(&mut self) {
        let op1 = self.r_rs() as i32;
        let op2 = self.r_rt() as i32;

        if op2 == 0 {
            self.hi = op1 as u32;
            if op1 >= 0 {
                self.lo = 0xffff_ffff;
            } else {
                self.lo = 1;
            }
        } else if op1 as u32 == 0x8000_0000 && op2 == -1 {
            self.hi = 0;
            self.lo = 0x8000_0000;
        } else {
            self.hi = (op1 % op2) as u32;
            self.lo = (op1 / op2) as u32;
        }
    }

    #[inline(always)]
    pub fn ins_divu(&mut self) {
        let op1 = self.r_rs();
        let op2 = self.r_rt();

        if op2 == 0 {
            self.hi = op1;
            self.lo = 0xffff_ffff;
        } else {
            self.hi = op1 % op2;
            self.lo = op1 / op2;
        }
    }

    #[inline(always)]
    pub fn ins_add(&mut self) {
        let op1 = self.r_rs() as i32;
        let op2 = self.r_rt() as i32;

        match op1.checked_add(op2) {
            Some(v) => self.write_reg(self.current_instruction.rd(), v as u32),
            None => self.exception(Exception::Overflow),
        }
    }

    #[inline(always)]
    pub fn ins_addu(&mut self) {
        self.write_reg(
            self.current_instruction.rd(),
            self.r_rs().wrapping_add(self.r_rt()),
        );
    }

    #[inline(always)]
    pub fn ins_sub(&mut self) {
        let op1 = self.r_rs() as i32;
        let op2 = self.r_rt() as i32;

        match op1.checked_sub(op2) {
            Some(v) => self.write_reg(self.current_instruction.rd(), v as u32),
            None => self.exception(Exception::Overflow),
        }
    }

    #[inline(always)]
    pub fn ins_subu(&mut self) {
        self.write_reg(
            self.current_instruction.rd(),
            self.r_rs().wrapping_sub(self.r_rt()),
        );
    }

    #[inline(always)]
    pub fn ins_and(&mut self) {
        self.write_reg(self.current_instruction.rd(), self.r_rs() & self.r_rt());
    }

    #[inline(always)]
    pub fn ins_or(&mut self) {
        self.write_reg(self.current_instruction.rd(), self.r_rs() | self.r_rt());
    }

    #[inline(always)]
    pub fn ins_xor(&mut self) {
        self.write_reg(self.current_instruction.rd(), self.r_rs() ^ self.r_rt());
    }

    #[inline(always)]
    pub fn ins_nor(&mut self) {
        self.write_reg(self.current_instruction.rd(), !(self.r_rs() | self.r_rt()));
    }

    #[inline(always)]
    pub fn ins_slt(&mut self) {
        self.write_reg(
            self.current_instruction.rd(),
            if (self.r_rs() as i32) < (self.r_rt() as i32) {
                1
            } else {
                0
            },
        );
    }

    #[inline(always)]
    pub fn ins_sltu(&mut self) {
        self.write_reg(
            self.current_instruction.rd(),
            if self.r_rs() < self.r_rt() { 1 } else { 0 },
        );
    }

    #[inline(always)]
    pub fn ins_addi(&mut self) {
        let op1 = self.r_rs() as i32;
        let op2 = self.current_instruction.simm16() as i32;

        match op1.checked_add(op2) {
            Some(v) => self.write_reg(self.current_instruction.rt(), v as u32),
            None => self.exception(Exception::Overflow),
        }
    }

    #[inline(always)]
    pub fn ins_addiu(&mut self) {
        let value = self.current_instruction.simm16() as i32;
        self.write_reg(
            self.current_instruction.rt(),
            self.r_rs().wrapping_add(value as u32),
        );
    }

    #[inline(always)]
    pub fn ins_slti(&mut self) {
        let value = self.current_instruction.simm16() as i32;
        self.write_reg(
            self.current_instruction.rt(),
            if (self.r_rs() as i32) < value { 1 } else { 0 },
        )
    }

    #[inline(always)]
    pub fn ins_sltiu(&mut self) {
        let value = self.current_instruction.simm16() as u32;
        self.write_reg(
            self.current_instruction.rt(),
            if self.r_rs() < value { 1 } else { 0 },
        )
    }

    #[inline(always)]
    pub fn ins_andi(&mut self) {
        let value = self.current_instruction.imm16() as u32;
        self.write_reg(self.current_instruction.rt(), self.r_rs() & value);
    }

    #[inline(always)]
    pub fn ins_ori(&mut self) {
        let value = self.current_instruction.imm16() as u32;
        self.write_reg(self.current_instruction.rt(), self.r_rs() | value);
    }

    #[inline(always)]
    pub fn ins_xori(&mut self) {
        let value = self.current_instruction.imm16() as u32;
        self.write_reg(self.current_instruction.rt(), self.r_rs() ^ value);
    }

    #[inline(always)]
    pub fn ins_lui(&mut self) {
        let value = (self.current_instruction.imm16() as u32) << 16;
        self.write_reg(self.current_instruction.rt(), value);
    }

    #[inline(always)]
    pub fn ins_mfhi(&mut self) {
        self.write_reg(self.current_instruction.rd(), self.hi);
    }

    #[inline(always)]
    pub fn ins_mthi(&mut self) {
        self.hi = self.r_rs();
    }

    #[inline(always)]
    pub fn ins_mflo(&mut self) {
        self.write_reg(self.current_instruction.rd(), self.lo);
    }

    #[inline(always)]
    pub fn ins_mtlo(&mut self) {
        self.lo = self.r_rs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::cpu::R3000Type;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct NullBus {}

    impl PsxBus for NullBus {
        fn read<T: R3000Type>(&self, _: u32) -> u32 {
            0
        }
        fn write<T: R3000Type>(&self, _: u32, _: u32) {}
        fn update_cycles(&self, _: u64) {}
    }

    impl<T: PsxBus> Cpu<T> {
        pub fn trash_registers(&mut self) {
            for i in 1..31 {
                self.regs[i] = if i % 2 == 0 { 0x1337_c0d3 } else { 0xf00d_beef };
            }
        }
    }

    fn make_cpu() -> Cpu<NullBus> {
        Cpu::new()
    }

    #[test]
    fn test_sll_nop() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SLL r0, r0, 0 => NOP
        cpu.current_instruction.0 = 0x00000000;
        cpu.ins_sll();

        assert_eq!(cpu.regs[0], 0);
    }

    #[test]
    fn test_sll_zero_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SLL r1, r1, 0
        cpu.current_instruction.0 = 0x0001_0800;
        cpu.ins_sll();

        assert_eq!(cpu.regs[1], 0xf00d_beef);
    }

    #[test]
    fn test_sll_one_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SLL r1, r1, 1
        cpu.current_instruction.0 = 0x0001_0840;
        cpu.ins_sll();

        assert_eq!(cpu.regs[1], 0xe01b_7dde);
    }

    #[test]
    fn test_sll_30_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SLL r1, r1, 30
        cpu.current_instruction.0 = 0x0001_0f80;
        cpu.ins_sll();

        assert_eq!(cpu.regs[1], 0xc000_0000);
    }

    #[test]
    fn test_sll_31_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SLL r1, r1, 31
        cpu.current_instruction.0 = 0x0001_0fc0;
        cpu.ins_sll();

        assert_eq!(cpu.regs[1], 0x8000_0000);
    }

    #[test]
    fn test_sll_different_regs_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SLL r1, r2, 6
        cpu.current_instruction.0 = 0x0002_0980;
        cpu.ins_sll();

        assert_eq!(cpu.regs[1], 0xcdf0_34c0);
        assert_eq!(cpu.regs[2], 0x1337_c0d3);
    }

    #[test]
    fn test_srl_zero_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRL r1, r1, 0
        cpu.current_instruction.0 = 0x0001_0802;
        cpu.ins_srl();

        assert_eq!(cpu.regs[1], 0xf00d_beef);
    }

    #[test]
    fn test_srl_one_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRL r1, r1, 1
        cpu.current_instruction.0 = 0x0001_0842;
        cpu.ins_srl();

        assert_eq!(cpu.regs[1], 0x7806_df77);
    }

    #[test]
    fn test_srl_30_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRL r1, r1, 30
        cpu.current_instruction.0 = 0x0001_0f82;
        cpu.ins_srl();

        assert_eq!(cpu.regs[1], 0x0000_0003);
    }

    #[test]
    fn test_srl_31_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRL r1, r1, 31
        cpu.current_instruction.0 = 0x0001_0fc2;
        cpu.ins_srl();

        assert_eq!(cpu.regs[1], 0x0000_0001);
    }

    #[test]
    fn test_srl_different_regs_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRL r1, r2, 6
        cpu.current_instruction.0 = 0x0002_0982;
        cpu.ins_srl();

        assert_eq!(cpu.regs[1], 0x004c_df03);
        assert_eq!(cpu.regs[2], 0x1337_c0d3);
    }

    #[test]
    fn test_sra_zero_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRA r1, r1, 0
        cpu.current_instruction.0 = 0x0001_0803;
        cpu.ins_sra();

        assert_eq!(cpu.regs[1], 0xf00d_beef);
    }

    #[test]
    fn test_sra_one_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRA r1, r1, 1
        cpu.current_instruction.0 = 0x0001_0843;
        cpu.ins_sra();

        assert_eq!(cpu.regs[1], 0xf806_df77);
    }

    #[test]
    fn test_sra_30_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRA r1, r1, 30
        cpu.current_instruction.0 = 0x0001_0f83;
        cpu.ins_sra();

        assert_eq!(cpu.regs[1], 0xffff_ffff);
    }

    #[test]
    fn test_sra_31_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRA r1, r1, 31
        cpu.current_instruction.0 = 0x0001_0fc3;
        cpu.ins_sra();

        assert_eq!(cpu.regs[1], 0xffff_ffff);
    }

    #[test]
    fn test_sra_different_regs_sa() {
        let mut cpu = make_cpu();
        cpu.trash_registers();

        // SRA r1, r2, 6
        cpu.current_instruction.0 = 0x0002_0983;
        cpu.ins_sra();

        assert_eq!(cpu.regs[1], 0x004c_df03);
        assert_eq!(cpu.regs[2], 0x1337_c0d3);
    }
}
