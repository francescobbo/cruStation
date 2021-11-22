use crustationlogger::*;

use super::algebra::Axis::{X, Y};
use super::algebra::*;
use super::color::Color;
use super::division;
use super::Gte;

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

impl Gte {
    pub fn execute(&mut self, op: u32) {
        self.instruction = op;

        debug!(self.logger, "OP {:02x}", op & 0x3f);

        self.flags.0 = 0;

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
                err!(self.logger, "Unimplemented operation {:02x}", op & 0x3f);
            }
            _ => {
                err!(self.logger, "Invalid opcode {:08x}", op);
            }
        }
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
        self.set_mac_ir(first, self.op_lm());

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

        let value = sx0 * sy1 + sx1 * sy2 + sx2 * sy0 - sx0 * sy2 - sx1 * sy0 - sx2 * sy1;
        self.set_mac0(value);
    }

    pub fn op(&mut self) {
        self.set_mac_ir(self.ir.cross(&self.rotation.diagonal()), self.op_lm());
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
        self.set_mac_ir((rgb << 12) + self.ir * self.ir0 as i64, self.op_lm());

        self.push_color(self.mac);
    }

    pub fn ncs(&mut self) {
        self.set_mac_ir(self.light * self.v0, self.op_lm());

        self.set_mac_ir(
            (self.background_color << 12) + (self.light_color * self.ir),
            self.op_lm(),
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
        self.set_mac_ir(self.light * self.v0, self.op_lm());

        self.set_mac_ir(
            (self.background_color << 12) + (self.light_color * self.ir),
            self.op_lm(),
        );

        self.set_mac_ir((self.ir * self.color.as_vec()) << 4, self.op_lm());

        self.color_fifo.remove(0);
        self.color_fifo.push(Color {
            r: (self.mac.0 >> 4).clamp(0, 0xff) as u8,
            g: (self.mac.1 >> 4).clamp(0, 0xff) as u8,
            b: (self.mac.2 >> 4).clamp(0, 0xff) as u8,
            code: self.color.code,
        });
    }

    pub fn ncds(&mut self) {
        self.set_mac_ir(self.light * self.v0, self.op_lm());

        self.set_mac_ir(
            (self.background_color << 12) + (self.light_color * self.ir),
            self.op_lm(),
        );

        let orig_ir = self.ir;

        self.set_mac_ir(
            (self.far_color << 12) - (self.color.as_vec() << 4) * self.ir,
            false,
        );

        self.set_mac_ir(
            (self.color.as_vec() << 4) * orig_ir + self.ir * (self.ir0 as i64),
            self.op_lm(),
        );

        self.color_fifo.remove(0);
        self.color_fifo.push(Color {
            r: (self.mac.0 >> 4).clamp(0, 0xff) as u8,
            g: (self.mac.1 >> 4).clamp(0, 0xff) as u8,
            b: (self.mac.2 >> 4).clamp(0, 0xff) as u8,
            code: self.color.code,
        });
    }

    /// Sums 3 Z values in the screen FIFO (1, 2 and 3) and multiplies them
    /// by ZSF3. The result is >> 12 and put in OTZ.
    /// 
    /// Somehow "computes an average"
    pub fn avsz3(&mut self) {
        let value = self.zsf3 as i64;
        let sum = (self.z_fifo[1] as i64) + (self.z_fifo[2] as i64) + (self.z_fifo[3] as i64);
        self.set_mac0(value * sum);
        self.set_otz((value * sum) >> 12);
    }

    /// Sums all the Z values in the screen FIFO (0, 1, 2 and 3) and multiplies
    /// them by ZSF4. The result is >> 12 and put in OTZ.
    /// 
    /// Somehow "computes an average"
    pub fn avsz4(&mut self) {
        let value = self.zsf4 as i64;
        let sum = (self.z_fifo[0] as i64)
            + (self.z_fifo[1] as i64)
            + (self.z_fifo[2] as i64)
            + (self.z_fifo[3] as i64);
        self.set_mac0(value * sum);
        self.set_otz((value * sum) >> 12);
    }

    fn set_mac0(&mut self, value: i64) {
        self.mac0 = value;

        if self.mac0 >= 0x8000_0000 {
            self.flags.set_mac0_of_pos(true);
        } else if self.mac0 < -0x8000_0000 {
            self.flags.set_mac0_of_neg(true);
        }
    }

    fn set_otz(&mut self, value: i64) {
        let saturated = value.clamp(0, 0xffff);
        if saturated != value {
            self.flags.set_sz3_otz_sat(true);
        }

        self.otz = saturated as u16;
    }

    fn set_mac_ir(&mut self, value: Vector3, lm_flag: bool) {
        println!("Setting MAC TO {:016x} {:016x} {:016x}", value.0, value.1, value.2);
        self.mac = value;

        if self.mac.0 >= 0x800_0000_0000 {
            self.flags.set_mac1_of_pos(true);
        } else if self.mac.0 < -0x800_0000_0000 {
            self.flags.set_mac1_of_neg(true);
        }

        if self.mac.1 >= 0x800_0000_0000 {
            self.flags.set_mac2_of_pos(true);
        } else if self.mac.1 < -0x800_0000_0000 {
            self.flags.set_mac2_of_neg(true);
        }

        if self.mac.2 >= 0x800_0000_0000 {
            self.flags.set_mac3_of_pos(true);
        } else if self.mac.2 < -0x800_0000_0000 {
            self.flags.set_mac3_of_neg(true);
        }

        if self.op_shift() {
            self.mac = self.mac.shift_fraction()
        }

        self.mac = self.mac.truncate();
        self.set_ir(self.mac, lm_flag);
    }

    fn push_color(&mut self, value: Vector3) {
        println!("PUSHING COLOR {:#?}", value);

        let value = value >> 4;

        let r = value.0.clamp(0, 0xff) as u8;
        let g = value.1.clamp(0, 0xff) as u8;
        let b = value.2.clamp(0, 0xff) as u8;

        if r as u64 as i64 != value.0 {
            self.flags.set_color_r_sat(true);
        }
        
        if g as u64 as i64 != value.1 {
            self.flags.set_color_g_sat(true);
        }

        if b as u64 as i64 != value.2 {
            self.flags.set_color_b_sat(true);
        }

        self.color_fifo.remove(0);
        self.color_fifo.push(Color {
            r, g, b, 
            code: self.color.code,
        });
    }

    fn set_ir(&mut self, value: Vector3, lm_flag: bool) {
        self.ir = value;
        self.saturate_ir(lm_flag);
    }

    fn saturate_ir(&mut self, lm: bool) {
        let min = if lm { 0 } else { -0x8000 };

        let f0 = self.ir.0.clamp(min, 0x7fff);
        let f1 = self.ir.1.clamp(min, 0x7fff);
        let f2 = self.ir.2.clamp(min, 0x7fff);

        if f0 != self.ir.0 {
            self.flags.set_ir1_sat(true);
        }
        if f1 != self.ir.1 {
            self.flags.set_ir2_sat(true);
        }
        if f2 != self.ir.2 {
            self.flags.set_ir3_sat(true);
        }

        self.ir = Vector3(f0, f1, f2);
    }
}
