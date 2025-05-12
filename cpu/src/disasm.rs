#[derive(Copy, Clone)]
enum ArgumentFormats {
    Missing,
    None,
    RdRtRs,
    RdRsRt,
    LoadRtImmRs,
    RtRsSimm,
    RtRsImm,
    RdRtShamt,
    RtImm,
    RsOffset,
    RsRtOffset,
    RdRs,
    RsRt,
    Rs,
    CopRdRt,
    J,
}

#[derive(Copy, Clone)]
pub struct Opcode(u32);

impl Opcode {
    pub fn main_opcode(&self) -> u32 {
        self.0 >> 26
    }

    pub fn special_opcode(&self) -> u32 {
        self.0 & 0x3f
    }

    pub fn rs(&self) -> u32 {
        (self.0 >> 21) & 0x1f
    }

    pub fn rt(&self) -> u32 {
        (self.0 >> 16) & 0x1f
    }

    pub fn rd(&self) -> u32 {
        (self.0 >> 11) & 0x1f
    }

    pub fn imm16(&self) -> u32 {
        self.0 & 0xffff
    }

    pub fn simm16(&self) -> i32 {
        ((self.0 & 0xffff) as i16) as i32
    }

    pub fn cop_op(&self) -> u32 {
        (self.0 >> 21) & 0x1f
    }

    pub fn shamt(&self) -> u32 {
        (self.0 >> 6) & 0x1f
    }
}

pub struct Instruction {
    opcode: u32,
    special: u32,
    name: &'static str,
    format: ArgumentFormats,
}

pub struct Disasm;

const ALL_INSTRUCTIONS: [Instruction; 79] = [
    Instruction {
        opcode: 0x01,
        special: 0x00,
        name: "bcondz",
        format: ArgumentFormats::RsOffset,
    },
    Instruction {
        opcode: 0x02,
        special: 0x00,
        name: "j",
        format: ArgumentFormats::J,
    },
    Instruction {
        opcode: 0x03,
        special: 0x00,
        name: "jal",
        format: ArgumentFormats::J,
    },
    Instruction {
        opcode: 0x04,
        special: 0x00,
        name: "beq",
        format: ArgumentFormats::RsRtOffset,
    },
    Instruction {
        opcode: 0x05,
        special: 0x00,
        name: "bne",
        format: ArgumentFormats::RsRtOffset,
    },
    Instruction {
        opcode: 0x06,
        special: 0x00,
        name: "blez",
        format: ArgumentFormats::RsOffset,
    },
    Instruction {
        opcode: 0x07,
        special: 0x00,
        name: "bgtz",
        format: ArgumentFormats::RsOffset,
    },
    Instruction {
        opcode: 0x08,
        special: 0x00,
        name: "addi",
        format: ArgumentFormats::RtRsSimm,
    },
    Instruction {
        opcode: 0x09,
        special: 0x00,
        name: "addiu",
        format: ArgumentFormats::RtRsSimm,
    },
    Instruction {
        opcode: 0x0A,
        special: 0x00,
        name: "slti",
        format: ArgumentFormats::RtRsSimm,
    },
    Instruction {
        opcode: 0x0B,
        special: 0x00,
        name: "sltiu",
        format: ArgumentFormats::RtRsImm,
    },
    Instruction {
        opcode: 0x0C,
        special: 0x00,
        name: "andi",
        format: ArgumentFormats::RtRsImm,
    },
    Instruction {
        opcode: 0x0D,
        special: 0x00,
        name: "ori",
        format: ArgumentFormats::RtRsImm,
    },
    Instruction {
        opcode: 0x0E,
        special: 0x00,
        name: "xori",
        format: ArgumentFormats::RtRsImm,
    },
    Instruction {
        opcode: 0x0F,
        special: 0x00,
        name: "lui",
        format: ArgumentFormats::RtImm,
    },
    Instruction {
        opcode: 0x10,
        special: 0x00,
        name: "mfc0",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x10,
        special: 0x02,
        name: "cfc0",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x10,
        special: 0x04,
        name: "mtc0",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x10,
        special: 0x06,
        name: "ctc0",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x11,
        special: 0x00,
        name: "mfc1",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x11,
        special: 0x02,
        name: "cfc1",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x11,
        special: 0x04,
        name: "mtc1",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x11,
        special: 0x06,
        name: "ctc1",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x12,
        special: 0x00,
        name: "mfc2",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x12,
        special: 0x02,
        name: "cfc2",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x12,
        special: 0x04,
        name: "mtc2",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x12,
        special: 0x06,
        name: "ctc2",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x13,
        special: 0x00,
        name: "mfc3",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x13,
        special: 0x02,
        name: "cfc3",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x13,
        special: 0x04,
        name: "mtc3",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x13,
        special: 0x06,
        name: "ctc3",
        format: ArgumentFormats::CopRdRt,
    },
    Instruction {
        opcode: 0x20,
        special: 0x00,
        name: "lb",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x21,
        special: 0x00,
        name: "lh",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x22,
        special: 0x00,
        name: "lwl",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x23,
        special: 0x00,
        name: "lw",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x24,
        special: 0x00,
        name: "lbu",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x25,
        special: 0x00,
        name: "lhu",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x26,
        special: 0x00,
        name: "lwr",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x28,
        special: 0x00,
        name: "sb",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x29,
        special: 0x00,
        name: "sh",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x2A,
        special: 0x00,
        name: "swl",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x2B,
        special: 0x00,
        name: "sw",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x2E,
        special: 0x00,
        name: "swr",
        format: ArgumentFormats::LoadRtImmRs,
    },
    Instruction {
        opcode: 0x30,
        special: 0x00,
        name: "lwc0",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x31,
        special: 0x00,
        name: "lwc1",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x32,
        special: 0x00,
        name: "lwc2",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x33,
        special: 0x00,
        name: "lwc3",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x38,
        special: 0x00,
        name: "swc0",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x39,
        special: 0x00,
        name: "swc1",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x3A,
        special: 0x00,
        name: "swc2",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x3B,
        special: 0x00,
        name: "swc3",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x00,
        special: 0x00,
        name: "sll",
        format: ArgumentFormats::RdRtShamt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x02,
        name: "srl",
        format: ArgumentFormats::RdRtShamt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x03,
        name: "sra",
        format: ArgumentFormats::RdRtShamt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x04,
        name: "sllv",
        format: ArgumentFormats::RdRtRs,
    },
    Instruction {
        opcode: 0x00,
        special: 0x06,
        name: "srlv",
        format: ArgumentFormats::RdRtRs,
    },
    Instruction {
        opcode: 0x00,
        special: 0x07,
        name: "srav",
        format: ArgumentFormats::RdRtRs,
    },
    Instruction {
        opcode: 0x00,
        special: 0x08,
        name: "jr",
        format: ArgumentFormats::Rs,
    },
    Instruction {
        opcode: 0x00,
        special: 0x09,
        name: "jalr",
        format: ArgumentFormats::RdRs,
    },
    Instruction {
        opcode: 0x00,
        special: 0x0C,
        name: "syscall",
        format: ArgumentFormats::None,
    },
    Instruction {
        opcode: 0x00,
        special: 0x0D,
        name: "break",
        format: ArgumentFormats::None,
    },
    Instruction {
        opcode: 0x00,
        special: 0x10,
        name: "mfhi",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x00,
        special: 0x11,
        name: "mthi",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x00,
        special: 0x12,
        name: "mflo",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x00,
        special: 0x13,
        name: "mtlo",
        format: ArgumentFormats::Missing,
    },
    Instruction {
        opcode: 0x00,
        special: 0x18,
        name: "mult",
        format: ArgumentFormats::RsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x19,
        name: "multu",
        format: ArgumentFormats::RsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x1A,
        name: "div",
        format: ArgumentFormats::RsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x1B,
        name: "divu",
        format: ArgumentFormats::RsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x20,
        name: "add",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x21,
        name: "addu",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x22,
        name: "sub",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x23,
        name: "subu",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x24,
        name: "and",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x25,
        name: "or",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x26,
        name: "xor",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x27,
        name: "nor",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x2A,
        name: "slt",
        format: ArgumentFormats::RdRsRt,
    },
    Instruction {
        opcode: 0x00,
        special: 0x2B,
        name: "sltu",
        format: ArgumentFormats::RdRsRt,
    },
];

impl Disasm {
    pub fn disasm(instruction: u32, pc: u32) -> String {
        let opcode = Opcode(instruction);

        let main_op = opcode.main_opcode();
        let special_op = opcode.special_opcode();
        let cop_op = opcode.cop_op();

        for ins in &ALL_INSTRUCTIONS {
            if ins.opcode == main_op {
                if main_op == 0 {
                    if ins.special == special_op {
                        return format!(
                            "{} {}",
                            ins.name,
                            Disasm::format_args(opcode, ins.format, pc)
                        );
                    } else {
                        continue;
                    }
                } else if (0x10..=0x13).contains(&main_op) {
                    if ins.special == cop_op {
                        return format!(
                            "{} {}",
                            ins.name,
                            Disasm::format_args(opcode, ins.format, pc)
                        );
                    } else {
                        continue;
                    }
                } else {
                    return format!(
                        "{} {}",
                        ins.name,
                        Disasm::format_args(opcode, ins.format, pc)
                    );
                }
            }
        }

        return format!("invalid ({:8x})", instruction);
    }

    pub fn is_function_call(instruction: u32) -> bool {
        let opcode = Opcode(instruction);

        if opcode.main_opcode() == 0x03
            || (opcode.main_opcode() == 0x00 && opcode.special_opcode() == 0x09)
        {
            // jal or jalr
            true
        } else if opcode.main_opcode() == 0x01 {
            // branch (and link)
            (opcode.0 >> 20) & 1 == 1
        } else {
            false
        }
    }

    pub fn reg_name(n: u32) -> &'static str {
        match n {
            0 => "zero",
            1 => "at",
            2 => "v0",
            3 => "v1",
            4 => "a0",
            5 => "a1",
            6 => "a2",
            7 => "a3",
            8 => "t0",
            9 => "t1",
            10 => "t2",
            11 => "t3",
            12 => "t4",
            13 => "t5",
            14 => "t6",
            15 => "t7",
            16 => "s0",
            17 => "s1",
            18 => "s2",
            19 => "s3",
            20 => "s4",
            21 => "s5",
            22 => "s6",
            23 => "s7",
            24 => "t8",
            25 => "t9",
            26 => "k0",
            27 => "k1",
            28 => "gp",
            29 => "sp",
            30 => "fp",
            31 => "ra",
            _ => panic!("Invalid register number"),
        }
    }

    fn format_args(opcode: Opcode, format: ArgumentFormats, pc: u32) -> String {
        match format {
            ArgumentFormats::Missing => String::from("??"),
            ArgumentFormats::RdRtRs => {
                format!(
                    "{}, {}, {}",
                    Disasm::reg_name(opcode.rd()),
                    Disasm::reg_name(opcode.rt()),
                    Disasm::reg_name(opcode.rs())
                )
            }
            ArgumentFormats::RdRsRt => {
                format!(
                    "{}, {}, {}",
                    Disasm::reg_name(opcode.rd()),
                    Disasm::reg_name(opcode.rs()),
                    Disasm::reg_name(opcode.rt())
                )
            }
            ArgumentFormats::LoadRtImmRs => {
                let imm = opcode.simm16();
                match imm {
                    0 => {
                        format!(
                            "{}, ({})",
                            Disasm::reg_name(opcode.rt()),
                            Disasm::reg_name(opcode.rs())
                        )
                    }
                    n if n > 0 => {
                        format!(
                            "{}, ({} + {:x})",
                            Disasm::reg_name(opcode.rt()),
                            Disasm::reg_name(opcode.rs()),
                            imm
                        )
                    }
                    _ => {
                        format!(
                            "{}, ({} - {:x})",
                            Disasm::reg_name(opcode.rt()),
                            Disasm::reg_name(opcode.rs()),
                            -imm
                        )
                    }
                }
            }
            ArgumentFormats::RtRsSimm => {
                let imm = opcode.simm16();
                if imm < 0 {
                    format!(
                        "{}, {}, -{:x}",
                        Disasm::reg_name(opcode.rt()),
                        Disasm::reg_name(opcode.rs()),
                        -imm
                    )
                } else {
                    format!(
                        "{}, {}, {:x}",
                        Disasm::reg_name(opcode.rt()),
                        Disasm::reg_name(opcode.rs()),
                        imm
                    )
                }
            }
            ArgumentFormats::RtRsImm => {
                let imm = opcode.imm16();
                format!(
                    "{}, {}, 0x{:04x}",
                    Disasm::reg_name(opcode.rt()),
                    Disasm::reg_name(opcode.rs()),
                    imm
                )
            }
            ArgumentFormats::RdRtShamt => {
                let imm = opcode.shamt();
                format!(
                    "{}, {}, {}",
                    Disasm::reg_name(opcode.rd()),
                    Disasm::reg_name(opcode.rt()),
                    imm
                )
            }
            ArgumentFormats::J => {
                let imm = (opcode.0 & 0x3ff_ffff) << 2;
                let high = pc & 0xf000_0000;
                format!("{:08x}", high | imm)
            }
            ArgumentFormats::RdRs => {
                format!(
                    "{}, {}",
                    Disasm::reg_name(opcode.rd()),
                    Disasm::reg_name(opcode.rs())
                )
            }
            ArgumentFormats::RsRt => {
                format!(
                    "{}, {}",
                    Disasm::reg_name(opcode.rs()),
                    Disasm::reg_name(opcode.rt())
                )
            }
            ArgumentFormats::CopRdRt => {
                format!("cop{}, {}", opcode.rd(), Disasm::reg_name(opcode.rt()))
            }
            ArgumentFormats::Rs => Disasm::reg_name(opcode.rs()).to_string(),
            ArgumentFormats::RtImm => {
                format!("{}, {:04x}", Disasm::reg_name(opcode.rt()), opcode.imm16())
            }
            ArgumentFormats::RsOffset => {
                let imm = (opcode.simm16() as i32) << 2;
                let target = pc.wrapping_add((imm + 4) as u32);
                format!("{}, {:08x}", Disasm::reg_name(opcode.rs()), target)
            }
            ArgumentFormats::RsRtOffset => {
                let imm = (opcode.simm16() as i32) << 2;
                let target = pc.wrapping_add((imm + 4) as u32);
                format!(
                    "{}, {}, {:08x}",
                    Disasm::reg_name(opcode.rs()),
                    Disasm::reg_name(opcode.rt()),
                    target
                )
            }
            ArgumentFormats::None => String::from(""),
        }
    }
}
