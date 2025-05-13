use bitfield::bitfield;
use crustationlogger::*;

mod division;
mod operations;

/*
    Registers  | Type  | Name             | Description
    -----------|-------|------------------|---
    cop2r0-1   | 3xS16 | VXY0,VZ0         | Vector 0 (X,Y,Z)
    cop2r2-3   | 3xS16 | VXY1,VZ1         | Vector 1 (X,Y,Z)
    cop2r4-5   | 3xS16 | VXY2,VZ2         | Vector 2 (X,Y,Z)
    cop2r6     | 4xU8  | RGBC             | Color/code value
    cop2r7     | 1xU16 | OTZ              | Average Z value (for Ordering Table)
    cop2r8     | 1xS16 | IR0              | 16bit Accumulator (Interpolate)
    cop2r9-11  | 3xS16 | IR[1-3]          | 16bit Accumulator (Vector)
    cop2r12-15 | 6xS16 | SXY[0-2,P]       | Screen XY-coordinate FIFO  (3 stages)
    cop2r16-19 | 4xU16 | SZ[0-3]          | Screen Z-coordinate FIFO   (4 stages)
    cop2r20-22 | 12xU8 | RGB[0-3]         | Color CRGB-code/color FIFO (3 stages)
    cop2r23    | 4xU8  | (RES1)           | Prohibited
    cop2r24    | 1xS32 | MAC0             | 32bit Maths Accumulators (Value)
    cop2r25-27 | 3xS32 | MAC[1-3]         | 32bit Maths Accumulators (Vector)
    cop2r28-29 | 1xU15 | IRGB,ORGB        | Convert RGB Color (48bit vs 15bit)
    cop2r30-31 | 2xS32 | LZCS,LZCR        | Count Leading-Zeroes/Ones (sign bits)
    cop2r32-36 | 9xS16 | RT11RT12,..,RT33 | Rotation matrix     (3x3)        ;cnt0-4
    cop2r37-39 | 3x 32 | TRX,TRY,TRZ      | Translation vector  (X,Y,Z)      ;cnt5-7
    cop2r40-44 | 9xS16 | L11L12,..,L33    | Light source matrix (3x3)        ;cnt8-12
    cop2r45-47 | 3x 32 | RBK,GBK,BBK      | Background color    (R,G,B)      ;cnt13-15
    cop2r48-52 | 9xS16 | LR1LR2,..,LB3    | Light color matrix source (3x3)  ;cnt16-20
    cop2r53-55 | 3x 32 | RFC,GFC,BFC      | Far color           (R,G,B)      ;cnt21-23
    cop2r56-57 | 2x 32 | OFX,OFY          | Screen offset       (X,Y)        ;cnt24-25
    cop2r58    | FU16  | H                | Projection plane distance.       ;cnt26
    cop2r59    | S16   | DQA              | Depth queing parameter A (coeff) ;cnt27
    cop2r60    | 32    | DQB              | Depth queing parameter B (offset);cnt28
    cop2r61-62 | 2xS16 | ZSF3,ZSF4        | Average Z scale factors          ;cnt29-30
    cop2r63    | U20   | FLAG             | Returns any calculation errors   ;cnt31
*/

bitfield! {
    pub struct Flags(u32);
    impl Debug;

    ir0_sat, set_ir0_sat: 12;
    ir1_sat, set_ir1_sat: 24;
    ir2_sat, set_ir2_sat: 23;
    ir3_sat, set_ir3_sat: 22;

    color_r_sat, set_color_r_sat: 21;
    color_g_sat, set_color_g_sat: 20;
    color_b_sat, set_color_b_sat: 19;

    mac0_of_pos, set_mac0_of_pos: 16;
    mac0_of_neg, set_mac0_of_neg: 15;
    mac1_of_pos, set_mac1_of_pos: 30;
    mac1_of_neg, set_mac1_of_neg: 27;
    mac2_of_pos, set_mac2_of_pos: 29;
    mac2_of_neg, set_mac2_of_neg: 26;
    mac3_of_pos, set_mac3_of_pos: 28;
    mac3_of_neg, set_mac3_of_neg: 25;

    sx2_sat, set_sx2_sat: 14;
    sy2_sat, set_sy2_sat: 13;

    sz3_otz_sat, set_sz3_otz_sat: 18;
    division_overflow, set_division_overflow: 17;

    error, set_error: 31;
}

type Matrix = [[i16; 3]; 3];

#[derive(Copy, Clone, Debug)]
struct RGB {
    r: u8,
    g: u8,
    b: u8,
    code: u8,
}

#[derive(Copy, Clone, Debug)]
struct XY {
    x: i16,
    y: i16,
}

pub struct Gte {
    logger: Logger,

    current_instruction: u32,

    cr: [u32; 32],

    rotation: Matrix,
    light: Matrix,
    color: Matrix,

    t: [i32; 4],
    b: [i32; 4],
    fc: [i32; 4],
    null: [i32; 4],

    /// Screen offset
    /// 32 bit, signed 15.16 fixed point
    ofx: i32,
    ofy: i32,

    /// Projection plane distance
    /// 16 bit integer, unsigned.
    h: u16,

    dqa: i16,
    dqb: i32,

    zsf3: i16,
    zsf4: i16,

    vectors: [[i16; 4]; 3],
    rgb: RGB,
    otz: u16,

    /// Intermediary registers
    /// 16 bit integers, signed.
    ir: [i16; 4],

    xy_fifo: [XY; 4],

    /// Screen Z-coordinate FIFO
    /// 16 bit integer, unsigned.
    z_fifo: [u16; 4],

    rgb_fifo: [RGB; 3],

    /// Math accumulators.
    /// 32 bit integers, signed.
    mac: [i32; 4],

    lzcs: u32,
    lzcr: u32,

    r23: u32,

    // r63
    flags: Flags,
}

impl Gte {
    pub fn new() -> Gte {
        Gte {
            logger: Logger::new("GTE", Level::Debug),

            current_instruction: 0,

            cr: [0; 32],

            rotation: [[0; 3]; 3],
            light: [[0; 3]; 3],
            color: [[0; 3]; 3],

            t: [0; 4],
            b: [0; 4],
            fc: [0; 4],
            null: [0; 4],

            ofx: 0,
            ofy: 0,
            h: 0,
            dqa: 0,
            dqb: 0,

            zsf3: 0,
            zsf4: 0,

            vectors: [[0; 4]; 3],
            rgb: RGB {
                r: 0,
                g: 0,
                b: 0,
                code: 0,
            },
            otz: 0,

            ir: [0; 4],

            xy_fifo: [XY { x: 0, y: 0 }; 4],
            z_fifo: [0; 4],
            rgb_fifo: [RGB {
                r: 0,
                g: 0,
                b: 0,
                code: 0,
            }; 3],
            mac: [0; 4],
            lzcs: 0,
            lzcr: 0,
            r23: 0,
            flags: Flags(0),
        }
    }

    pub fn read_reg(&mut self, index: u32) -> u32 {
        let index = index as usize;
        if index >= 32 {
            self.read_cr(index - 32)
        } else {
            self.read_dr(index)
        }
    }

    fn read_dr(&self, index: usize) -> u32 {
        match index {
            0 => (self.vectors[0][0] as u16 as u32) | ((self.vectors[0][1] as u16 as u32) << 16),
            1 => self.vectors[0][2] as u32,
            2 => (self.vectors[1][0] as u16 as u32) | ((self.vectors[1][1] as u16 as u32) << 16),
            3 => self.vectors[1][2] as u32,
            4 => (self.vectors[2][0] as u16 as u32) | ((self.vectors[2][1] as u16 as u32) << 16),
            5 => self.vectors[2][2] as u32,
            6 => {
                self.rgb.r as u32
                    | ((self.rgb.g as u32) << 8)
                    | ((self.rgb.b as u32) << 16)
                    | ((self.rgb.code as u32) << 24)
            }
            7 => self.otz as u32,
            8 => self.ir[0] as u32,
            9 => self.ir[1] as u32,
            10 => self.ir[2] as u32,
            11 => self.ir[3] as u32,
            12 => (self.xy_fifo[0].x as u16 as u32) | ((self.xy_fifo[0].y as u16 as u32) << 16),
            13 => (self.xy_fifo[1].x as u16 as u32) | ((self.xy_fifo[1].y as u16 as u32) << 16),
            14 | 15 => {
                (self.xy_fifo[2].x as u16 as u32) | ((self.xy_fifo[2].y as u16 as u32) << 16)
            }
            16 => self.z_fifo[0] as u32,
            17 => self.z_fifo[1] as u32,
            18 => self.z_fifo[2] as u32,
            19 => self.z_fifo[3] as u32,
            20 => {
                (self.rgb_fifo[0].r as u32)
                    | ((self.rgb_fifo[0].g as u32) << 8)
                    | ((self.rgb_fifo[0].b as u32) << 16)
                    | ((self.rgb_fifo[0].code as u32) << 24)
            }
            21 => {
                (self.rgb_fifo[1].r as u32)
                    | ((self.rgb_fifo[1].g as u32) << 8)
                    | ((self.rgb_fifo[1].b as u32) << 16)
                    | ((self.rgb_fifo[1].code as u32) << 24)
            }
            22 => {
                (self.rgb_fifo[2].r as u32)
                    | ((self.rgb_fifo[2].g as u32) << 8)
                    | ((self.rgb_fifo[2].b as u32) << 16)
                    | ((self.rgb_fifo[2].code as u32) << 24)
            }
            23 => self.r23,
            24 => self.mac[0] as u32,
            25 => self.mac[1] as u32,
            26 => self.mac[2] as u32,
            27 => self.mac[3] as u32,
            28 | 29 => {
                Gte::sat5(self.ir[1] >> 7) as u32
                    | ((Gte::sat5(self.ir[2] >> 7) as u32) << 5)
                    | ((Gte::sat5(self.ir[3] >> 7) as u32) << 10)
            }
            30 => self.lzcs,
            31 => {
                if self.lzcs as i32 >= 0 {
                    self.lzcs.leading_zeros()
                } else {
                    self.lzcs.leading_ones()
                }
            }
            _ => unreachable!("{}", index),
        }
    }

    fn read_cr(&self, index: usize) -> u32 {
        match index {
            24 => self.ofx as u32,
            25 => self.ofy as u32,
            26 => self.h as i16 as u32,
            27 => self.dqa as i16 as u32,
            28 => self.dqb as u32,
            29 => self.zsf3 as i16 as u32,
            30 => self.zsf4 as i16 as u32,
            31 => self.cr[31],
            4 | 12 | 20 => self.cr[index] as i16 as u32,
            _ => self.cr[index],
        }
    }

    pub fn write_reg(&mut self, index: u32, value: u32) {
        let index = index as usize;
        // println!("[GTE] Writing {:08x} to r{}", value, index);

        if index >= 32 {
            self.write_cr(index - 32, value);
        } else {
            self.write_dr(index, value);
        }
    }

    fn write_cr(&mut self, index: usize, value: u32) {
        const MASK_TABLE: [u32; 32] = [
            /* 0x00 */
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            /* 0x08 */
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            /* 0x10 */
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            /* 0x18 */
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0x0000_ffff,
            0xffff_ffff,
        ];

        let value = value & MASK_TABLE[index];
        self.cr[index] = value | (self.cr[index] & !MASK_TABLE[index]);

        if index < 24 {
            let we = index >> 3;
            let index = index & 7;

            if index >= 5 {
                let vector = match we {
                    0 => &mut self.t,
                    1 => &mut self.b,
                    2 => &mut self.fc,
                    _ => unreachable!(),
                };

                vector[index - 5] = value as i32;
            } else {
                let matrix = match we {
                    0 => &mut self.rotation,
                    1 => &mut self.light,
                    2 => &mut self.color,
                    _ => unreachable!(),
                };

                match index {
                    0 => {
                        matrix[0][0] = value as i16;
                        matrix[0][1] = (value >> 16) as i16;
                    }
                    1 => {
                        matrix[0][2] = value as i16;
                        matrix[1][0] = (value >> 16) as i16;
                    }
                    2 => {
                        matrix[1][1] = value as i16;
                        matrix[1][2] = (value >> 16) as i16;
                    }
                    3 => {
                        matrix[2][0] = value as i16;
                        matrix[2][1] = (value >> 16) as i16;
                    }
                    4 => {
                        matrix[2][2] = value as i16;
                    }
                    _ => unreachable!(),
                }
            }

            return;
        }

        match index {
            24 => {
                self.ofx = value as i32;
            }
            25 => {
                self.ofy = value as i32;
            }
            26 => {
                self.h = value as u16;
            }
            27 => {
                self.dqa = value as i16;
            }
            28 => {
                self.dqb = value as i32;
            }
            29 => {
                self.zsf3 = value as i16;
            }
            30 => {
                self.zsf4 = value as i16;
            }
            31 => {
                self.cr[31] = value & 0x7fff_f000;
                if value & 0x7f87e000 != 0 {
                    self.cr[31] |= 1 << 31;
                }
            }
            _ => unreachable!(),
        }
    }

    fn write_dr(&mut self, index: usize, value: u32) {
        match index {
            0 => {
                self.vectors[0][0] = value as i16;
                self.vectors[0][1] = (value >> 16) as i16;
            }
            1 => {
                self.vectors[0][2] = value as i16;
            }
            2 => {
                self.vectors[1][0] = value as i16;
                self.vectors[1][1] = (value >> 16) as i16;
            }
            3 => {
                self.vectors[1][2] = value as i16;
            }
            4 => {
                self.vectors[2][0] = value as i16;
                self.vectors[2][1] = (value >> 16) as i16;
            }
            5 => {
                self.vectors[2][2] = value as i16;
            }
            6 => {
                self.rgb.r = value as u8;
                self.rgb.g = (value >> 8) as u8;
                self.rgb.b = (value >> 16) as u8;
                self.rgb.code = (value >> 24) as u8;
            }
            7 => {
                self.otz = value as u16;
            }
            8 => {
                self.ir[0] = value as i16;
            }
            9 => {
                self.ir[1] = value as i16;
            }
            10 => {
                self.ir[2] = value as i16;
            }
            11 => {
                self.ir[3] = value as i16;
            }
            12 => {
                self.xy_fifo[0].x = value as i16;
                self.xy_fifo[0].y = (value >> 16) as i16;
            }
            13 => {
                self.xy_fifo[1].x = value as i16;
                self.xy_fifo[1].y = (value >> 16) as i16;
            }
            14 => {
                self.xy_fifo[2].x = value as i16;
                self.xy_fifo[2].y = (value >> 16) as i16;
                self.xy_fifo[3].x = value as i16;
                self.xy_fifo[3].y = (value >> 16) as i16;
            }
            15 => {
                self.xy_fifo[3].x = value as i16;
                self.xy_fifo[3].y = (value >> 16) as i16;

                self.xy_fifo[0] = self.xy_fifo[1];
                self.xy_fifo[1] = self.xy_fifo[2];
                self.xy_fifo[2] = self.xy_fifo[3];
            }
            16 => {
                self.z_fifo[0] = value as u16;
            }
            17 => {
                self.z_fifo[1] = value as u16;
            }
            18 => {
                self.z_fifo[2] = value as u16;
            }
            19 => {
                self.z_fifo[3] = value as u16;
            }
            20 => {
                self.rgb_fifo[0].r = value as u8;
                self.rgb_fifo[0].g = (value >> 8) as u8;
                self.rgb_fifo[0].b = (value >> 16) as u8;
                self.rgb_fifo[0].code = (value >> 24) as u8;
            }
            21 => {
                self.rgb_fifo[1].r = value as u8;
                self.rgb_fifo[1].g = (value >> 8) as u8;
                self.rgb_fifo[1].b = (value >> 16) as u8;
                self.rgb_fifo[1].code = (value >> 24) as u8;
            }
            22 => {
                self.rgb_fifo[2].r = value as u8;
                self.rgb_fifo[2].g = (value >> 8) as u8;
                self.rgb_fifo[2].b = (value >> 16) as u8;
                self.rgb_fifo[2].code = (value >> 24) as u8;
            }
            23 => {
                self.r23 = value;
            }
            24 => {
                self.mac[0] = value as i32;
            }
            25 => {
                self.mac[1] = value as i32;
            }
            26 => {
                self.mac[2] = value as i32;
            }
            27 => {
                self.mac[3] = value as i32;
            }
            28 => {
                self.ir[1] = ((value & 0x1f) << 7) as i16;
                self.ir[2] = (((value >> 5) & 0x1f) << 7) as i16;
                self.ir[3] = (((value >> 10) & 0x1f) << 7) as i16;
            }
            29 => {}
            30 => {
                self.lzcs = value;
                self.lzcr = if self.lzcs as i32 >= 0 {
                    self.lzcs.leading_zeros()
                } else {
                    self.lzcs.leading_ones()
                }
            }
            31 => {}
            _ => unreachable!(),
        }
    }

    pub fn execute(&mut self, instruction: u32) {
        self.flags.0 = 0;
        self.current_instruction = instruction;

        match instruction & 0x3f {
            0x01 => self.rtps(),
            0x06 => self.nclip(),
            0x0c => self.op(),
            0x10 => self.dpcs(),
            0x11 => self.intpl(),
            0x12 => self.mvmva(),
            0x13 => self.ncds(),
            0x14 => self.cdp(),
            0x16 => self.ncdt(),
            0x1b => self.nccs(),
            0x1c => self.cc(),
            0x1e => self.ncs(),
            0x20 => self.nct(),
            0x28 => self.sqr(),
            0x29 => self.dcpl(),
            0x2a => self.dpct(),
            0x2d => self.avsz3(),
            0x2e => self.avsz4(),
            0x30 => self.rtpt(),
            0x3d => self.gpf(),
            0x3e => self.gpl(),
            0x3f => self.ncct(),
            _ => err!(self.logger, "Unknown function {}", instruction & 0x3f),
        }

        if self.flags.0 & 0x7f87_e000 != 0 {
            self.flags.set_error(true);
        }

        self.cr[31] = self.flags.0;
    }

    fn sat5(cc: i16) -> u8 {
        if cc < 0 {
            0
        } else if cc > 0x1f {
            0x1f
        } else {
            cc as u8
        }
    }

    pub fn op_lm(&self) -> bool {
        self.current_instruction & (1 << 10) != 0
    }

    pub fn op_shift(&self) -> bool {
        self.current_instruction & (1 << 19) != 0
    }
}
