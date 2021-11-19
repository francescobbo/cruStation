mod algebra;

// use std::num::Wrapping;

use crate::hw::gte::algebra::Axis::{X, Y, Z};
use crate::hw::gte::algebra::*;

/**
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

// enum Registers {
//     V0xy = 0, V0z = 1,
//     V1xy = 2, V1z = 3,
//     V2xy = 4, V2z = 5,
//     RGBC = 6,
//     OTZ = 7,
//     IR0 = 8,
//     IR1 = 9, IR2 = 10, IR3 = 11,
//     SXY0 = 12, SXY1 = 13, SXY2 = 14, SXYP = 15,
//     SZ0 = 16, SZ1 = 17, SZ2 = 18, SZ3 = 19,
//     RGB0 = 20, RGB1 = 21, RGB2 = 22,
//     RES1 = 23,
//     MAC0 = 24,
//     MAC1 = 25, MAC2 = 26, MAC3 = 27,
//     IRGB = 28, ORGB = 29,
//     LZCS = 30, LZCR = 31,
//     RT11_12 = 32, RT13_21 = 33, RT22_23 = 34, RT31_32 = 35, RT33 = 36,
//     TRX = 37, TRY = 38, TRZ = 39,
//     L11_12 = 40, L13_21 = 41, L22_23 = 42, L31_32 = 43, L33 = 44,
//     RBK = 45, GBK = 46, BBK = 47,
//     LR11_12 = 48, LR13_21 = 49, LR22_23 = 50, LR31_32 = 51, LR33 = 52,
//     RFC = 53, GFC = 54, BFC = 55,
//     OFX = 56, OFY = 57,
//     H = 58,
//     DQA = 59, DQB = 60,
//     ZSF3 = 61, ZSF4 = 62,
//     FLAG = 63
// }

trait Clamp {
    fn clamp(&self, min: Self, max: Self) -> Self;
}

impl Clamp for i32 {
    fn clamp(&self, min: Self, max: Self) -> Self {
        if *self < min {
            min
        } else if *self > max {
            max
        } else {
            *self
        }
    }
}

pub struct Gte {
    instruction: u32,

    v0: Vector3,
    v1: Vector3,
    v2: Vector3,

    ir0: i16,
    ir: Vector3,

    // mac0: i32,
    mac: Vector3,

    rotation: Matrix3,
    translation: Vector3,
    light: Matrix3,
    background_color: Vector3,
    light_color: Matrix3,
    far_color: Vector3,

    regs: [u32; 64],

    xyz_fifo: Vec<u32>,
}

impl Gte {
    pub fn new() -> Gte {
        Gte {
            instruction: 0,

            v0: Vector3::new(),
            v1: Vector3::new(),
            v2: Vector3::new(),

            ir0: 0,
            ir: Vector3::new(),

            // mac0: 0,
            mac: Vector3::new(),

            regs: [0; 64],

            rotation: Matrix3::new(),
            translation: Vector3::new(),
            light: Matrix3::new(),
            background_color: Vector3::new(),
            light_color: Matrix3::new(),
            far_color: Vector3::new(),

            xyz_fifo: vec![0; 3],
        }
    }

    pub fn read_reg(&mut self, index: u32) -> u32 {
        match index {
            0 => self.v0.x_u32() | (self.v0.y_u32() << 16),
            1 => self.v0.z_u32s(),
            2 => self.v1.x_u32() | (self.v1.y_u32() << 16),
            3 => self.v1.z_u32s(),
            4 => self.v2.x_u32() | (self.v2.y_u32() << 16),
            5 => self.v2.z_u32s(),
            8 => self.ir0 as u32,
            9 => self.ir.x_u32s(),
            10 => self.ir.y_u32s(),
            11 => self.ir.z_u32s(),
            12 => self.xyz_fifo[0],
            13 => self.xyz_fifo[1],
            14 | 15 => self.xyz_fifo[2],
            25 => self.mac.0 as u32,
            26 => self.mac.1 as u32,
            27 => self.mac.2 as u32,
            28 | 29 => {
                let r = (self.ir[X] / 0x80).clamp(0, 0x1f);
                let g = (self.ir[Y] / 0x80).clamp(0, 0x1f);
                let b = (self.ir[Z] / 0x80).clamp(0, 0x1f);
                (r | (g << 5) | (b << 10)) as u32
            }
            31 => {
                if self.regs[30] as i32 >= 0 {
                    self.regs[30].leading_zeros()
                } else {
                    self.regs[30].leading_ones()
                }
            }
            32 => {
                let rt11 = self.rotation[0].x_u32();
                let rt12 = self.rotation[0].y_u32();
                rt11 | (rt12 << 16)
            }
            33 => {
                let rt13 = self.rotation[0].z_u32();
                let rt21 = self.rotation[1].x_u32();
                rt13 | (rt21 << 16)
            }
            34 => {
                let rt22 = self.rotation[1].y_u32();
                let rt23 = self.rotation[1].z_u32();
                rt22 | (rt23 << 16)
            }
            35 => {
                let rt31 = self.rotation[2].x_u32();
                let rt32 = self.rotation[2].y_u32();
                rt31 | (rt32 << 16)
            }
            36 => self.rotation[2].z_u32s(),
            40 => {
                let lt11 = self.light[0].x_u32();
                let lt12 = self.light[0].y_u32();
                lt11 | (lt12 << 16)
            }
            41 => {
                let lt13 = self.light[0].z_u32();
                let lt21 = self.light[1].x_u32();
                lt13 | (lt21 << 16)
            }
            42 => {
                let lt22 = self.light[1].y_u32();
                let lt23 = self.light[1].z_u32();
                lt22 | (lt23 << 16)
            }
            43 => {
                let lt31 = self.light[2].x_u32();
                let lt32 = self.light[2].y_u32();
                lt31 | (lt32 << 16)
            }
            44 => self.light[2].z_u32s(),
            45 => self.background_color.0 as u32,
            46 => self.background_color.1 as u32,
            47 => self.background_color.2 as u32,
            48 => {
                let lc11 = self.light_color[0].x_u32();
                let lc12 = self.light_color[0].y_u32();
                lc11 | (lc12 << 16)
            }
            49 => {
                let lc13 = self.light_color[0].z_u32();
                let lc21 = self.light_color[1].x_u32();
                lc13 | (lc21 << 16)
            }
            50 => {
                let lc22 = self.light_color[1].y_u32();
                let lc23 = self.light_color[1].z_u32();
                lc22 | (lc23 << 16)
            }
            51 => {
                let lc31 = self.light_color[2].x_u32();
                let lc32 = self.light_color[2].y_u32();
                lc31 | (lc32 << 16)
            }
            52 => self.light_color[2].z_u32s(),
            53 => self.far_color.0 as u32,
            54 => self.far_color.1 as u32,
            55 => self.far_color.2 as u32,
            _ => self.regs[index as usize],
        }
    }

    pub fn write_reg(&mut self, index: u32, value: u32) {
        let index = index as usize;

        match index {
            0 => {
                self.v0[X] = value as i16 as i32;
                self.v0[Y] = (value >> 16) as i16 as i32;
            }
            1 => {
                self.v0[Z] = value as i16 as i32;
            }
            2 => {
                self.v1[X] = value as i16 as i32;
                self.v1[Y] = (value >> 16) as i16 as i32;
            }
            3 => {
                self.v1[Z] = value as i16 as i32;
            }
            4 => {
                self.v2[X] = value as i16 as i32;
                self.v2[Y] = (value >> 16) as i16 as i32;
            }
            5 => {
                self.v2[Z] = value as i16 as i32;
            }
            7 => {
                self.regs[index] = value & 0xffff;
            }
            8 => {
                self.ir0 = value as i16;
            }
            9 => {
                self.ir[X] = value as i16 as i32;
            }
            10 => {
                self.ir[Y] = value as i16 as i32;
            }
            11 => {
                self.ir[Z] = value as i16 as i32;
            }
            12 => {
                self.xyz_fifo[0] = value;
            }
            13 => {
                self.xyz_fifo[1] = value;
            }
            14 => {
                self.xyz_fifo[2] = value;
            }
            15 => {
                self.xyz_fifo.remove(0);
                self.xyz_fifo.push(value);
            }
            16 | 17 | 18 | 19 => {
                self.regs[index] = value & 0xffff;
            }
            25 => {
                self.mac.0 = value as i32;
            }
            26 => {
                self.mac.1 = value as i32;
            }
            27 => {
                self.mac.2 = value as i32;
            }
            28 => {
                let red = value & 0x1f;
                let green = (value >> 5) & 0x1f;
                let blue = (value >> 10) & 0x1f;

                self.ir[X] = (red * 0x80) as i16 as i32;
                self.ir[Y] = (green * 0x80) as i16 as i32;
                self.ir[Z] = (blue * 0x80) as i16 as i32;

                self.regs[index] = value & 0x7fff;
                self.regs[29] = value & 0x7fff;
            }
            29 | 31 => { /* read only */ }
            /* Rotation matrix */
            32 => {
                let rt11 = value & 0xffff;
                let rt12 = value >> 16;
                self.rotation[0][X] = rt11 as i16 as i32;
                self.rotation[0][Y] = rt12 as i16 as i32;
            }
            33 => {
                let rt13 = value & 0xffff;
                let rt21 = value >> 16;
                self.rotation[0][Z] = rt13 as i16 as i32;
                self.rotation[1][X] = rt21 as i16 as i32;
            }
            34 => {
                let rt22 = value & 0xffff;
                let rt23 = value >> 16;
                self.rotation[1][Y] = rt22 as i16 as i32;
                self.rotation[1][Z] = rt23 as i16 as i32;
            }
            35 => {
                let rt31 = value & 0xffff;
                let rt32 = value >> 16;
                self.rotation[2][X] = rt31 as i16 as i32;
                self.rotation[2][Y] = rt32 as i16 as i32;
            }
            36 => {
                let rt33 = value & 0xffff;
                self.rotation[2][Z] = rt33 as i16 as i32;
            }
            /* Light matrix */
            40 => {
                let lt11 = value & 0xffff;
                let lt12 = value >> 16;
                self.light[0][X] = lt11 as i16 as i32;
                self.light[0][Y] = lt12 as i16 as i32;
            }
            41 => {
                let lt13 = value & 0xffff;
                let lt21 = value >> 16;
                self.light[0][Z] = lt13 as i16 as i32;
                self.light[1][X] = lt21 as i16 as i32;
            }
            42 => {
                let lt22 = value & 0xffff;
                let lt23 = value >> 16;
                self.light[1][Y] = lt22 as i16 as i32;
                self.light[1][Z] = lt23 as i16 as i32;
            }
            43 => {
                let lt31 = value & 0xffff;
                let lt32 = value >> 16;
                self.light[2][X] = lt31 as i16 as i32;
                self.light[2][Y] = lt32 as i16 as i32;
            }
            44 => {
                let lt33 = value & 0xffff;
                self.light[2][Z] = lt33 as i16 as i32;
            }            
            45 => {
                self.background_color.0 = value as i32;
            }
            46 => {
                self.background_color.1 = value as i32;
            }
            47 => {
                self.background_color.2 = value as i32;
            }
            /* Light color matrix */
            48 => {
                let lc11 = value & 0xffff;
                let lc12 = value >> 16;
                self.light_color[0][X] = lc11 as i16 as i32;
                self.light_color[0][Y] = lc12 as i16 as i32;
            }
            49 => {
                let lc13 = value & 0xffff;
                let lc21 = value >> 16;
                self.light_color[0][Z] = lc13 as i16 as i32;
                self.light_color[1][X] = lc21 as i16 as i32;
            }
            50 => {
                let lc22 = value & 0xffff;
                let lc23 = value >> 16;
                self.light_color[1][Y] = lc22 as i16 as i32;
                self.light_color[1][Z] = lc23 as i16 as i32;
            }
            51 => {
                let lc31 = value & 0xffff;
                let lc32 = value >> 16;
                self.light_color[2][X] = lc31 as i16 as i32;
                self.light_color[2][Y] = lc32 as i16 as i32;
            }
            52 => {
                let lc33 = value & 0xffff;
                self.light_color[2][Z] = lc33 as i16 as i32;
            }    
            53 => {
                self.far_color.0 = value as i32;
            }
            54 => {
                self.far_color.1 = value as i32;
            }
            55 => {
                self.far_color.2 = value as i32;
            }
            44 | 52 | 58 | 59 | 61 | 62 => {
                self.regs[index] = (value & 0xffff) as i16 as i32 as u32;
            }
            63 => {
                self.regs[index] = value & !0x8000_0fff;
                let errors = value & 0x7f87_e000 != 0;
                if errors {
                    self.regs[index] |= 1 << 31;
                }
            }
            _ => {
                self.regs[index] = value;
            }
        }
    }

    pub fn execute(&mut self, op: u32) {
        self.instruction = op;

        match op & 0x1f {
            0x01 => self.rtps(),
            0x06 => self.nclip(),
            0x0c => self.op(),
            0x10 => self.dpcs(),
            0x13 => self.ncds(),
            _ => {
                println!("[GTE] Unhandled {:08x}", op);
            }
        }
    }

    pub fn op_lm(&self) -> u32 {
        self.instruction & (1 << 10)
    }

    pub fn op_shift(&self) -> u32 {
        self.instruction & (1 << 19)
    }

    pub fn rtps(&mut self) {
        println!("RTPS");

        let trx = self.regs[37] as u64;
        let _try = self.regs[38] as u64;
        let trz = self.regs[39] as u64;

        let rt11 = (self.regs[32] & 0xffff) as u64;
        let rt12 = (self.regs[32] >> 16) as u64;
        let rt13 = (self.regs[33] & 0xffff) as u64;
        let rt21 = (self.regs[33] >> 16) as u64;
        let rt22 = (self.regs[34] & 0xffff) as u64;
        let rt23 = (self.regs[34] >> 16) as u64;
        let rt31 = (self.regs[35] & 0xffff) as u64;
        let rt32 = (self.regs[35] >> 16) as u64;
        let rt33 = (self.regs[36] & 0xffff) as u64;

        let vx0 = (self.regs[0] & 0xffff) as u64;
        let vy0 = (self.regs[0] >> 16) as u64;
        let vz0 = (self.regs[1] & 0xfff) as u64;

        let r1 = (trx * 0x1000)
            .wrapping_add(rt11 * vx0)
            .wrapping_add(rt12 * vy0)
            .wrapping_add(rt13 * vz0); // SAR sf*12
        let r2 = (_try * 0x1000)
            .wrapping_add(rt21 * vx0)
            .wrapping_add(rt22 * vy0)
            .wrapping_add(rt23 * vz0); // SAR sf*12
        let r3 = (trz * 0x1000)
            .wrapping_add(rt31 * vx0)
            .wrapping_add(rt32 * vy0)
            .wrapping_add(rt33 * vz0); // SAR sf*12

        self.ir[X] = r1 as i16 as i32;
        self.ir[Y] = r2 as i16 as i32;
        self.ir[Z] = r3 as i16 as i32;

        self.mac = self.ir;

        // let sz3 = (((self.regs[27] & 0xffff) as i16) >> 12) as i64 as u64;
        let mac3 = r3 as u32;
        self.regs[19] = mac3;

        // let h = self.regs[58] as u64;

        // let ir1 = self.ir[X] as u64;
        // let ir2 = self.ir[Y] as u64;
        // let ofx = self.regs[56] as u64;
        // let ofy = self.regs[57] as u64;
        // let dqa = self.regs[59] as u64;
        // let dqb = self.regs[60] as u64;

        // let mac0 = (((h * 0x20000 / sz3) + 1) / 2) * ir1 + ofx;
        // let sx2 = mac0 / 0x10000; // ScrX FIFO -400h..+3FFh
        // let mac0 = (((h * 0x20000 / sz3) + 1) / 2) * ir2 + ofy;
        // let sy2 = mac0 / 0x10000; // ;ScrY FIFO -400h..+3FFh
        // let mac0 = (((h * 0x20000 / sz3) + 1) / 2) * dqa + dqb;
        // let ir0 = mac0 / 0x1000; //  ;Depth cueing 0..+1000h

        // self.regs[8] = ir0 as u32;
        // self.regs[24] = mac0 as u32;
        // sxy2
    }

    pub fn nclip(&mut self) {
        println!("NCLIP");

        let sx0 = (self.xyz_fifo[0] & 0xffff) as i16 as i64;
        let sy0 = (self.xyz_fifo[0] >> 16) as i16 as i64;

        let sx1 = (self.xyz_fifo[1] & 0xffff) as i16 as i64;
        let sy1 = (self.xyz_fifo[1] >> 16) as i16 as i64;

        let sx2 = (self.xyz_fifo[2] & 0xffff) as i16 as i64;
        let sy2 = (self.xyz_fifo[2] >> 16) as i16 as i64;

        let mac0 = sx0 * sy1 + sx1 * sy2 + sx2 * sy0 - sx0 * sy2 - sx1 * sy0 - sx2 * sy1;
        if mac0 > (2_i64.pow(31)) {
            self.regs[63] = 0x80010000;
        } else if mac0 < -(2_i64.pow(31)) {
            self.regs[63] = 0x80008000;
        } else {
            self.regs[63] = 0;
        }

        self.regs[24] = mac0 as u32;
    }

    pub fn op(&mut self) {
        println!("OP");

        self.mac = self.ir.cross(&self.rotation.diagonal());
        if self.op_shift() != 0 {
            self.mac = self.mac.shift_fraction();
        }

        self.ir = self.mac;
        self.saturate_ir(self.op_lm() != 0);
    }

    pub fn dpcs(&mut self) {
        println!("DPCS");

        let r = (self.regs[6] & 0xff) as i32;
        let g = ((self.regs[6] >> 8) & 0xff) as i32;
        let b = ((self.regs[6] >> 16) & 0xff) as i32;
        // let code = (self.regs[6] >> 24) as i32;

        self.mac = Vector3(r << 16, g << 16, b << 16);
        self.mac = self.mac + (self.far_color - self.mac) * (self.ir0 as i32);
        if self.op_shift() != 0 {
            self.mac = self.mac.shift_fraction();
        }
        // Color FIFO = [MAC1/16,MAC2/16,MAC3/16,CODE],

        self.ir = self.mac;
        self.saturate_ir(self.op_lm() != 0);
    }

    pub fn ncds(&mut self) {
        println!("NCDS({})", self.op_shift() != 0);

        self.mac = self.light * self.v0;
        if self.op_shift() != 0 {
            self.mac = self.mac.shift_fraction();
        }
        self.ir = self.mac;

        self.mac = (self.background_color * 0x1000) + (self.light_color * self.ir);
        if self.op_shift() != 0 {
            self.mac = self.mac.shift_fraction();
        }
        self.ir = self.mac;

        self.mac = Vector3(
            self.ir.0 * ((self.regs[6] & 0xff) as i32) << 4,
            self.ir.1 * (((self.regs[6] >> 8) & 0xff) as i32) << 4,
            self.ir.2 * (((self.regs[6] >> 16) & 0xff) as i32) << 4
        );

        self.mac = self.mac + (self.far_color - self.mac) * self.ir0.into();
        if self.op_shift() != 0 {
            self.mac = self.mac.shift_fraction();
        }

        self.ir = self.mac;

        // self.color_fifo.push(Color(
        //     self.mac.0 / 16,
        //     self.mac.1 / 16,
        //     self.mac.2 / 16,
        //     self.regs[6] >> 24
        // ))
    }

    fn saturate_ir(&mut self, lm: bool) {
        let min = if lm { 0 } else { -0x8000 };

        let f0 = self.ir.0.clamp(min, 0x7fff);
        let f1 = self.ir.1.clamp(min, 0x7fff);
        let f2 = self.ir.2.clamp(min, 0x7fff);

        self.regs[63] = 0;

        if f0 != self.ir.0 {
            self.regs[63] |= 1 << 24
        }
        if f1 != self.ir.1 {
            self.regs[63] |= 1 << 23
        }
        if f2 != self.ir.2 {
            self.regs[63] |= 1 << 22
        }
        if (self.regs[63] & !(1 << 22)) != 0 {
            self.regs[63] |= 1 << 31
        }

        self.ir = Vector3(f0, f1, f2);
    }
}
