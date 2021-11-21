mod algebra;

// use std::num::Wrapping;

mod division;

use algebra::Axis::{X, Y, Z};
use algebra::*;

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

#[derive(Copy, Clone, Debug)]
struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub code: u8,
}

impl Color {
    fn new() -> Color {
        Color {
            r: 0,
            g: 0,
            b: 0,
            code: 0,
        }
    }

    fn from(value: u32) -> Color {
        Color {
            r: value as u8,
            g: (value >> 8) as u8,
            b: (value >> 16) as u8,
            code: (value >> 24) as u8,
        }
    }

    fn as_u32(&self) -> u32 {
        self.r as u32
            | ((self.g as u32) << 8)
            | ((self.b as u32) << 16)
            | ((self.code as u32) << 24)
    }

    fn as_vec(&self) -> Vector3 {
        Vector3(self.r as i64, self.g as i64, self.b as i64)
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> u32 {
        color.as_u32()
    }
}

pub struct Gte {
    instruction: u32,

    // r0-1
    v0: Vector3,
    // r2-3
    v1: Vector3,
    // r4-5
    v2: Vector3,
    // r6
    color: Color,
    // r7
    otz: u16,
    // r8
    ir0: i16,
    // r9-11
    ir: Vector3,
    // r12-15
    xy_fifo: Vec<Vector3>,
    // r16-19
    z_fifo: Vec<u16>,
    // r20-22
    color_fifo: Vec<Color>,
    // r23 unused?
    r23: u32,
    // r24,
    mac0: i64,
    // r25-27
    mac: Vector3,
    // r28-29 are on the fly
    // r30
    lzcs: u32,
    // r31 is on the fly
    // r32-36
    rotation: Matrix3,
    // r37-39
    translation: Vector3,
    // r40-44
    light: Matrix3,
    // r45-47
    background_color: Vector3,
    // r48-52
    light_color: Matrix3,
    // r53-55
    far_color: Vector3,
    // r56
    ofx: u32,
    // r57
    ofy: u32,
    // r58
    h: u16,
    // r59
    dqa: i16,
    // r60
    dqb: u32,
    // r61
    zsf3: i16,
    // r62
    zsf4: i16,
    // r63
    flags: u32,
}

impl Gte {
    pub fn new() -> Gte {
        Gte {
            instruction: 0,

            v0: Vector3::new(),
            v1: Vector3::new(),
            v2: Vector3::new(),
            color: Color::new(),
            otz: 0,
            ir0: 0,
            ir: Vector3::new(),
            xy_fifo: vec![Vector3::new(); 3],
            z_fifo: vec![0; 4],
            color_fifo: vec![Color::new(); 3],
            r23: 0,
            mac0: 0,
            mac: Vector3::new(),
            lzcs: 0,
            rotation: Matrix3::new(),
            translation: Vector3::new(),
            light: Matrix3::new(),
            background_color: Vector3::new(),
            light_color: Matrix3::new(),
            far_color: Vector3::new(),
            ofx: 0,
            ofy: 0,
            h: 0,
            dqa: 0,
            dqb: 0,
            zsf3: 0,
            zsf4: 0,
            flags: 0,
        }
    }

    pub fn read_reg(&mut self, index: u32) -> u32 {
        let value = match index {
            0 => self.v0.x_u32() | (self.v0.y_u32() << 16),
            1 => self.v0.z_u32s(),
            2 => self.v1.x_u32() | (self.v1.y_u32() << 16),
            3 => self.v1.z_u32s(),
            4 => self.v2.x_u32() | (self.v2.y_u32() << 16),
            5 => self.v2.z_u32s(),
            6 => self.color.into(),
            7 => self.otz as u32,
            8 => self.ir0 as u32,
            9 => self.ir.x_u32s(),
            10 => self.ir.y_u32s(),
            11 => self.ir.z_u32s(),
            12 => self.xy_fifo[0].x_u32() | (self.xy_fifo[0].y_u32() << 16),
            13 => self.xy_fifo[1].x_u32() | (self.xy_fifo[1].y_u32() << 16),
            14 | 15 => self.xy_fifo[2].x_u32() | (self.xy_fifo[2].y_u32() << 16),
            16 => self.z_fifo[0] as u32,
            17 => self.z_fifo[1] as u32,
            18 => self.z_fifo[2] as u32,
            19 => self.z_fifo[3] as u32,
            20 => self.color_fifo[0].into(),
            21 => self.color_fifo[1].into(),
            22 => self.color_fifo[2].into(),
            23 => self.r23,
            24 => self.mac0 as u32,
            25 => self.mac.0 as u32,
            26 => self.mac.1 as u32,
            27 => self.mac.2 as u32,
            28 | 29 => {
                let r = (self.ir[X] / 0x80).clamp(0, 0x1f);
                let g = (self.ir[Y] / 0x80).clamp(0, 0x1f);
                let b = (self.ir[Z] / 0x80).clamp(0, 0x1f);
                (r | (g << 5) | (b << 10)) as u32
            }
            30 => self.lzcs,
            31 => {
                if self.lzcs as i32 >= 0 {
                    self.lzcs.leading_zeros()
                } else {
                    self.lzcs.leading_ones()
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
            37 => self.translation.x_u32s(),
            38 => self.translation.y_u32s(),
            39 => self.translation.z_u32s(),
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
            56 => self.ofx as u32,
            57 => self.ofy as u32,
            58 => self.h as i16 as u32,
            59 => self.dqa as u32,
            60 => self.dqb as u32,
            61 => self.zsf3 as u32,
            62 => self.zsf4 as u32,
            63 => self.flags,
            _ => unreachable!("{}", index),
        };

        // println!("[GTE] Read {:08x} from r{}", value, index);
        value
    }

    pub fn write_reg(&mut self, index: u32, value: u32) {
        let index = index as usize;

        // println!("[GTE] Writing {:08x} to r{}", value, index);

        match index {
            0 => {
                self.v0[X] = value as i16 as i64;
                self.v0[Y] = (value >> 16) as i16 as i64;
            }
            1 => {
                self.v0[Z] = value as i16 as i64;
            }
            2 => {
                self.v1[X] = value as i16 as i64;
                self.v1[Y] = (value >> 16) as i16 as i64;
            }
            3 => {
                self.v1[Z] = value as i16 as i64;
            }
            4 => {
                self.v2[X] = value as i16 as i64;
                self.v2[Y] = (value >> 16) as i16 as i64;
            }
            5 => {
                self.v2[Z] = value as i16 as i64;
            }
            6 => {
                self.color = Color::from(value);
            }
            7 => {
                self.otz = value as u16;
            }
            8 => {
                self.ir0 = value as i16;
            }
            9 => {
                self.ir[X] = value as i16 as i64;
            }
            10 => {
                self.ir[Y] = value as i16 as i64;
            }
            11 => {
                self.ir[Z] = value as i16 as i64;
            }
            12 => {
                self.xy_fifo[0][X] = value as i16 as i64;
                self.xy_fifo[0][Y] = (value >> 16) as i16 as i64;
            }
            13 => {
                self.xy_fifo[1][X] = value as i16 as i64;
                self.xy_fifo[1][Y] = (value >> 16) as i16 as i64;
            }
            14 => {
                self.xy_fifo[2][X] = value as i16 as i64;
                self.xy_fifo[2][Y] = (value >> 16) as i16 as i64;
            }
            15 => {
                self.xy_fifo.remove(0);
                self.xy_fifo
                    .push(Vector3(value as i16 as i64, (value >> 16) as i16 as i64, 0));
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
                self.color_fifo[0] = Color::from(value);
            }
            21 => {
                self.color_fifo[1] = Color::from(value);
            }
            22 => {
                self.color_fifo[2] = Color::from(value);
            }
            23 => {
                self.r23 = value;
            }
            24 => {
                self.mac0 = value as i64;
            }
            25 => {
                self.mac.0 = value as i64;
            }
            26 => {
                self.mac.1 = value as i64;
            }
            27 => {
                self.mac.2 = value as i64;
            }
            28 => {
                let red = value & 0x1f;
                let green = (value >> 5) & 0x1f;
                let blue = (value >> 10) & 0x1f;

                self.ir[X] = (red * 0x80) as i16 as i64;
                self.ir[Y] = (green * 0x80) as i16 as i64;
                self.ir[Z] = (blue * 0x80) as i16 as i64;
            }
            30 => {
                self.lzcs = value;
            }
            29 | 31 => { /* read only */ }
            /* Rotation matrix */
            32 => {
                let rt11 = value & 0xffff;
                let rt12 = value >> 16;
                self.rotation[0][X] = rt11 as i16 as i64;
                self.rotation[0][Y] = rt12 as i16 as i64;
            }
            33 => {
                let rt13 = value & 0xffff;
                let rt21 = value >> 16;
                self.rotation[0][Z] = rt13 as i16 as i64;
                self.rotation[1][X] = rt21 as i16 as i64;
            }
            34 => {
                let rt22 = value & 0xffff;
                let rt23 = value >> 16;
                self.rotation[1][Y] = rt22 as i16 as i64;
                self.rotation[1][Z] = rt23 as i16 as i64;
            }
            35 => {
                let rt31 = value & 0xffff;
                let rt32 = value >> 16;
                self.rotation[2][X] = rt31 as i16 as i64;
                self.rotation[2][Y] = rt32 as i16 as i64;
            }
            36 => {
                let rt33 = value & 0xffff;
                self.rotation[2][Z] = rt33 as i16 as i64;
            }
            37 => {
                self.translation[X] = value as i32 as i64;
            }
            38 => {
                self.translation[Y] = value as i32 as i64;
            }
            39 => {
                self.translation[Z] = value as i32 as i64;
            }
            /* Light matrix */
            40 => {
                let lt11 = value & 0xffff;
                let lt12 = value >> 16;
                self.light[0][X] = lt11 as i16 as i64;
                self.light[0][Y] = lt12 as i16 as i64;
            }
            41 => {
                let lt13 = value & 0xffff;
                let lt21 = value >> 16;
                self.light[0][Z] = lt13 as i16 as i64;
                self.light[1][X] = lt21 as i16 as i64;
            }
            42 => {
                let lt22 = value & 0xffff;
                let lt23 = value >> 16;
                self.light[1][Y] = lt22 as i16 as i64;
                self.light[1][Z] = lt23 as i16 as i64;
            }
            43 => {
                let lt31 = value & 0xffff;
                let lt32 = value >> 16;
                self.light[2][X] = lt31 as i16 as i64;
                self.light[2][Y] = lt32 as i16 as i64;
            }
            44 => {
                let lt33 = value & 0xffff;
                self.light[2][Z] = lt33 as i16 as i64;
            }
            45 => {
                self.background_color.0 = value as i64;
            }
            46 => {
                self.background_color.1 = value as i64;
            }
            47 => {
                self.background_color.2 = value as i64;
            }
            /* Light color matrix */
            48 => {
                let lc11 = value & 0xffff;
                let lc12 = value >> 16;
                self.light_color[0][X] = lc11 as i16 as i64;
                self.light_color[0][Y] = lc12 as i16 as i64;
            }
            49 => {
                let lc13 = value & 0xffff;
                let lc21 = value >> 16;
                self.light_color[0][Z] = lc13 as i16 as i64;
                self.light_color[1][X] = lc21 as i16 as i64;
            }
            50 => {
                let lc22 = value & 0xffff;
                let lc23 = value >> 16;
                self.light_color[1][Y] = lc22 as i16 as i64;
                self.light_color[1][Z] = lc23 as i16 as i64;
            }
            51 => {
                let lc31 = value & 0xffff;
                let lc32 = value >> 16;
                self.light_color[2][X] = lc31 as i16 as i64;
                self.light_color[2][Y] = lc32 as i16 as i64;
            }
            52 => {
                let lc33 = value & 0xffff;
                self.light_color[2][Z] = lc33 as i16 as i64;
            }
            53 => {
                self.far_color.0 = value as i64;
            }
            54 => {
                self.far_color.1 = value as i64;
            }
            55 => {
                self.far_color.2 = value as i64;
            }
            56 => {
                self.ofx = value;
            }
            57 => {
                self.ofy = value;
            }
            58 => {
                self.h = value as u16;
            }
            59 => {
                self.dqa = value as i16;
            }
            60 => {
                self.dqb = value;
            }
            61 => {
                self.zsf3 = value as i16;
            }
            62 => {
                self.zsf4 = value as i16;
            }
            63 => {
                self.flags = value & !0x8000_0fff;
                let errors = value & 0x7f87_e000 != 0;
                if errors {
                    self.flags |= 1 << 31;
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn execute(&mut self, op: u32) {
        self.instruction = op;

        match op & 0x3f {
            0x01 => self.rtps(0, true),
            0x06 => self.nclip(),
            0x0c => self.op(),
            0x10 => self.dpcs(),
            0x13 => self.ncds(),
            0x1b => self.nccs(),
            0x1e => self.ncs(),
            0x2d => self.avsz3(),
            0x2e => self.avsz4(),
            0x30 => self.rtpt(),
            0x3f => {
                println!("Unimplemented 0x3f GTE");
            }
            _ => {
                panic!("[GTE] Unhandled {:08x}", op);
            }
        }
    }

    pub fn op_lm(&self) -> u32 {
        self.instruction & (1 << 10)
    }

    pub fn op_shift(&self) -> u32 {
        self.instruction & (1 << 19)
    }

    pub fn rtps(&mut self, v_idx: usize, finalize: bool) {
        let vec = if v_idx == 0 {
            &self.v0
        } else if v_idx == 1 {
            &self.v1
        } else {
            &self.v2
        };

        let dots = Vector3(
            self.rotation[0].dot(vec),
            self.rotation[1].dot(vec),
            self.rotation[2].dot(vec),
        );

        let first = (self.translation << 12) + dots;
        self.set_mac_ir(first, self.op_lm() != 0);

        let new_z = ((first.2 >> 12) as i32).clamp(0, 0xffff) as u16;
        self.z_fifo.remove(0);
        self.z_fifo.push(new_z);

        let (h_over_s3z, _) = division::division(self.h, new_z as u16);
        let h_over_s3z = h_over_s3z as i32 as i64;
        let mut x = h_over_s3z * self.ir.0 + (self.ofx as i32 as i64);
        let mut y = h_over_s3z * self.ir.1 + (self.ofy as i32 as i64);

        self.mac0 = y;
        x >>= 16;
        y >>= 16;

        self.xy_fifo[0] = self.xy_fifo[1];
        self.xy_fifo[1] = self.xy_fifo[2];
        self.xy_fifo[2] = Vector3(
            x.clamp(-0x400, 0x3ff) as i32 as i64,
            y.clamp(-0x400, 0x3ff) as i32 as i64,
            0,
        );

        if finalize {
            self.mac0 = h_over_s3z * (self.dqa as i32 as i64) + (self.dqb as i32 as i64);
            self.ir0 = (self.mac0 >> 12).clamp(0, 0x1000) as i16;
        }
    }

    pub fn rtpt(&mut self) {
        self.rtps(0, false);
        self.rtps(1, false);
        self.rtps(2, true);
    }

    pub fn nclip(&mut self) {
        let sx0 = self.xy_fifo[0][X] as i64;
        let sy0 = self.xy_fifo[0][Y] as i64;

        let sx1 = self.xy_fifo[1][X] as i64;
        let sy1 = self.xy_fifo[1][Y] as i64;

        let sx2 = self.xy_fifo[2][X] as i64;
        let sy2 = self.xy_fifo[2][Y] as i64;

        let mac0 = sx0 * sy1 + sx1 * sy2 + sx2 * sy0 - sx0 * sy2 - sx1 * sy0 - sx2 * sy1;
        if mac0 > (2_i64.pow(31)) {
            self.flags = 0x80010000;
        } else if mac0 < -(2_i64.pow(31)) {
            self.flags = 0x80008000;
        } else {
            self.flags = 0;
        }

        self.mac0 = mac0 as i64;
    }

    pub fn op(&mut self) {
        self.mac = self.ir.cross(&self.rotation.diagonal());
        if self.op_shift() != 0 {
            self.mac = self.mac.shift_fraction();
        }

        self.ir = self.mac;
        self.saturate_ir(self.op_lm() != 0);
    }

    fn set_ir(&mut self, value: Vector3, lm_flag: bool) {
        let min = if lm_flag { 0 } else { -0x8000 };

        self.ir = Vector3(
            value.0.clamp(min, 0x7fff),
            value.1.clamp(min, 0x7fff),
            value.2.clamp(min, 0x7fff),
        )
    }

    fn set_mac_ir(&mut self, value: Vector3, lm_flag: bool) {
        self.mac = if self.op_shift() != 0 {
            value.shift_fraction()
        } else {
            value
        }
        .truncate();

        self.set_ir(self.mac, lm_flag);
    }

    /// Linearly interpolates the main color with the far color.
    /// The result is pushed to the color FIFO.
    ///
    /// Inputs:
    ///   - Color (8, 8, 8, )
    ///   - Far color (1, 27, 4)
    ///   - IR0
    ///
    /// Outputs:
    ///   - Pushes to Color FIFO
    ///
    /// Uses:
    ///   - MAC1/2/3
    ///   - IR1/2/3
    ///
    /// `MainColor + (FarColor - MainColor) * IR0`
    pub fn dpcs(&mut self) {
        // Align bits to Far color (lower 4 bits are "fraction")
        let rgb = self.color.as_vec() << 4;

        // ir = far - rgb
        self.set_mac_ir((self.far_color << 12) - (rgb << 12), false);

        // mac = rgb + (far - rgb) * ir0
        self.set_mac_ir((rgb << 12) + self.ir * self.ir0 as i64, self.op_lm() != 0);

        self.color_fifo.remove(0);
        self.color_fifo.push(Color {
            r: (self.mac.0 >> 4).clamp(0, 0xff) as u8,
            g: (self.mac.1 >> 4).clamp(0, 0xff) as u8,
            b: (self.mac.2 >> 4).clamp(0, 0xff) as u8,
            code: self.color.code,
        });
    }

    pub fn ncs(&mut self) {
        self.set_mac_ir(self.light * self.v0, self.op_lm() != 0);

        self.set_mac_ir(
            (self.background_color << 12) + (self.light_color * self.ir),
            self.op_lm() != 0,
        );

        self.color_fifo.remove(0);
        self.color_fifo.push(Color {
            r: (self.mac.0 >> 4).clamp(0, 0xff) as u8,
            g: (self.mac.1 >> 4).clamp(0, 0xff) as u8,
            b: (self.mac.2 >> 4).clamp(0, 0xff) as u8,
            code: self.color.code,
        });
    }

    pub fn nccs(&mut self) {
        self.set_mac_ir(self.light * self.v0, self.op_lm() != 0);

        self.set_mac_ir(
            (self.background_color << 12) + (self.light_color * self.ir),
            self.op_lm() != 0,
        );

        self.set_mac_ir((self.ir * self.color.as_vec()) << 4, self.op_lm() != 0);

        self.color_fifo.remove(0);
        self.color_fifo.push(Color {
            r: (self.mac.0 >> 4).clamp(0, 0xff) as u8,
            g: (self.mac.1 >> 4).clamp(0, 0xff) as u8,
            b: (self.mac.2 >> 4).clamp(0, 0xff) as u8,
            code: self.color.code,
        });
    }

    pub fn ncds(&mut self) {
        self.set_mac_ir(self.light * self.v0, self.op_lm() != 0);

        self.set_mac_ir(
            (self.background_color << 12) + (self.light_color * self.ir),
            self.op_lm() != 0,
        );

        let orig_ir = self.ir;

        self.set_mac_ir(
            (self.far_color << 12) - (self.color.as_vec() << 4) * self.ir,
            false,
        );

        self.set_mac_ir(
            (self.color.as_vec() << 4) * orig_ir + self.ir * (self.ir0 as i64),
            self.op_lm() != 0,
        );

        self.color_fifo.remove(0);
        self.color_fifo.push(Color {
            r: (self.mac.0 >> 4).clamp(0, 0xff) as u8,
            g: (self.mac.1 >> 4).clamp(0, 0xff) as u8,
            b: (self.mac.2 >> 4).clamp(0, 0xff) as u8,
            code: self.color.code,
        });
    }

    fn saturate_ir(&mut self, lm: bool) {
        let min = if lm { 0 } else { -0x8000 };

        let f0 = self.ir.0.clamp(min, 0x7fff);
        let f1 = self.ir.1.clamp(min, 0x7fff);
        let f2 = self.ir.2.clamp(min, 0x7fff);

        self.flags = 0;

        if f0 != self.ir.0 {
            self.flags |= 1 << 24
        }
        if f1 != self.ir.1 {
            self.flags |= 1 << 23
        }
        if f2 != self.ir.2 {
            self.flags |= 1 << 22
        }
        if (self.flags & !(1 << 22)) != 0 {
            self.flags |= 1 << 31
        }

        self.ir = Vector3(f0, f1, f2);
    }

    pub fn avsz3(&mut self) {
        let value = self.zsf3 as i64;
        let sum = (self.z_fifo[1] as i64)
            .wrapping_add(self.z_fifo[2] as i64)
            .wrapping_add(self.z_fifo[3] as i64);
        self.mac0 = value * sum;
        self.otz = (value >> 12).clamp(0, 0xffff) as u16;
    }

    pub fn avsz4(&mut self) {
        let value = self.zsf4 as i64;
        let sum = (self.z_fifo[0] as i64)
            .wrapping_add(self.z_fifo[1] as i64)
            .wrapping_add(self.z_fifo[2] as i64)
            .wrapping_add(self.z_fifo[3] as i64);
        self.mac0 = value * sum;
        self.otz = (value >> 12).clamp(0, 0xffff) as u16;
    }
}
