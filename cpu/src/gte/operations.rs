use crustationlogger::*;

use super::algebra::Axis::{X, Y, Z};
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
            self.flags.0 = 0x80010000;
        } else if mac0 < -(2_i64.pow(31)) {
            self.flags.0 = 0x80008000;
        } else {
            self.flags.0 = 0;
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
