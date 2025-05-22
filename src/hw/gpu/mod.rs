mod renderer;
mod shaders;

use std::cell::RefCell;
use std::rc::Weak;

use bitfield::bitfield;
use crustationgui::{
    gpu_command::{PsxPrimitiveVertex, PsxUv},
    GpuCommand, PsxColor, PsxVertex,
};
// use renderer::{Color, Position, Renderer};

use crate::hw::bus::{Bus, BusDevice, PsxEventType};

bitfield! {
    struct GpuStat(u32);
    impl Debug;

    pub texture_page_x_base, _: 3, 0;
    pub texture_page_y_base, _: 4;
    pub semi_transparency, _: 6, 5;
    pub texture_page_colors, _: 8, 7;
    pub dither_24_to_15, _: 9;
    pub drawing_to_display_allowed, _: 10;
    pub mask_bit_while_drawing, _: 11;
    pub draw_pixels, _: 12;
    pub interlaced_field, _: 13;
    pub reverse_flag, _: 14;
    pub texture_disable, _: 15;
    pub horizontal_res2, _: 16;
    pub horizontal_res1, _: 18, 17;
    pub vertical_res, _: 19;
    pub video_mode, _: 20;
    pub color_depth, _: 21;
    pub vertical_interlace, _: 22;
    pub display_enable, set_display_enable: 23;
    pub irq, set_irq: 24;
    pub dma, _: 25;
    pub ready_for_command, _: 26;
    pub ready_to_send, _: 27;
    pub ready_to_receive, _: 28;
    pub dma_direction, _: 30, 29;
    pub even_odd, set_even_odd: 31;
}

pub struct Gpu {
    // renderer: Option<Renderer>,
    gpustat: GpuStat,
    buffer: Vec<u32>,
    remaining_words: usize,

    /// Left-most column of drawing area
    drawing_area_left: u16,
    /// Top-most line of drawing area
    drawing_area_top: u16,
    /// Right-most column of drawing area
    drawing_area_right: u16,
    /// Bottom-most line of drawing area
    drawing_area_bottom: u16,
    /// Drawing offset in the framebuffer
    drawing_offset: (i16, i16),

    horizontal_res: u16,
    vertical_res: u16,
    display_top: u16,
    display_left: u16,

    texpage_e1: u32,

    /// Renderer command channel
    renderer_tx: crossbeam_channel::Sender<GpuCommand>,
}

impl Gpu {
    pub fn new(renderer_tx: crossbeam_channel::Sender<GpuCommand>) -> Gpu {
        Gpu {
            // renderer: None,
            gpustat: GpuStat(0x1480_2000),
            buffer: vec![],
            remaining_words: 0,

            drawing_area_left: 0,
            drawing_area_top: 0,
            drawing_area_right: 1023,
            drawing_area_bottom: 511,
            drawing_offset: (0, 0),

            horizontal_res: 0,
            vertical_res: 0,
            display_top: 0,
            display_left: 0,

            texpage_e1: 0,

            renderer_tx,
        }
    }

    pub fn load_renderer(&mut self) {
        // self.renderer = Some(Renderer::new());
    }
}

impl BusDevice for Gpu {
    fn write<const S: u32>(&mut self, addr: u32, value: u32) {
        if S != 4 {
            // println!("Unhandled {}-bytes GPU read",
            // std::mem::size_of::<T>());
        }

        match addr {
            0 => self.process_gp0(value),
            4 => self.process_gp1(value),
            _ => panic!("Invalid write to gpu"),
        }
    }

    fn read<const S: u32>(&mut self, addr: u32) -> u32 {
        if S != 4 {
            // println!("Unhandled {}-bytes GPU read", std::mem::size_of::<T>());
            return 0;
        }

        match addr {
            0 => {
                // println!("Read GPUREAD");
                0
            }
            4 => {
                // println!("Read GPUSTAT");
                self.gpustat.0 | (1 << 27)
            }
            _ => panic!("Invalid read to gpu"),
        }
    }
}

impl Gpu {
    pub fn vblank(&mut self) {
        if !self.gpustat.vertical_res() {
            // 240 lines
            // println!("Don't know how to handle VBlank in 240 mode");
            // self.gpustat.set_even_odd(!self.gpustat.even_odd());
        } else {
            // 480 lines
            self.gpustat.set_even_odd(!self.gpustat.even_odd());
        }

        // println!("VSync");
        self.gpustat.set_irq(true);
        // self.bus.upgrade().unwrap().borrow().send_irq(0);

        // println!("VSync IRQ");
        // if let Some(renderer) = &mut self.renderer {
        //     renderer.poll_events();
        //     renderer.draw();
        // }
    }

    pub fn process_gp0(&mut self, command: u32) {
        self.buffer.push(command);

        if self.remaining_words == 0 {
            // First command in a possible list
            let opcode = command >> 24;
            self.remaining_words = match opcode {
                0x68 | 0x6a | 0x70 | 0x72 | 0x78 | 0x7a => 1,
                0x02 | 0x60 | 0x62 | 0x6c | 0x6d | 0x6e | 0x6f | 0x74 | 0x75 | 0x76 | 0x77
                | 0x7c | 0x7d | 0x7e | 0x7f | 0xa0 | 0xc0 => 2,
                0x20 | 0x22 | 0x64 | 0x65 | 0x66 | 0x67 | 0x80 => 3,
                0x28 | 0x2a => 4,
                0x30 | 0x32 => 5,
                0x24 | 0x25 | 0x26 | 0x27 => 6,
                0x38 | 0x3a => 7,
                0x2c | 0x2d | 0x2e | 0x2f | 0x34 | 0x36 => 8,
                0x3c | 0x3e => 11,
                0x40 | 0x42 | 0x48 | 0x4a | 0x50 | 0x52 | 0x58 | 0x5a => 0x55555555,
                _ => 0,
            };
        } else if self.remaining_words == 0x5555_5555 {
            // List terminator
            if command == 0x5555_5555 {
                self.remaining_words = 0
            }
        } else {
            self.remaining_words -= 1;
        }

        if self.remaining_words == 0 {
            let command = self.buffer[0];
            let opcode = command >> 24;

            match opcode as u8 {
                0x00 | 0x04..=0x1e | 0xe0 | 0xe7..=0xef => self.gp0_00_nop(),
                0x01 => self.gp0_01_clear_cache(),
                0x02 => self.gp0_02_fill_rectangle(),
                0x03 => self.gp0_03_nop2(),
                0x1f => self.gp0_1f_interrupt_request(),
                0x20 => self.gp0_20_mono_triangle(),
                0x22 => self.gp0_22_mono_triangle_alpha(),
                0x24 => self.gp0_24_triangle_texture_blended(),
                0x25 => self.gp0_25_triangle_texture_raw(),
                0x26 => self.gp0_26_triangle_alpha_texture_blended(),
                0x27 => self.gp0_27_triangle_alpha_texture_raw(),
                0x28 => self.gp0_28_mono_square(),
                0x2a => self.gp0_2a_mono_square_alpha(),
                0x2c => self.gp0_2c_square_texture_blended(),
                0x2d => self.gp0_2d_square_texture_raw(),
                0x2e => self.gp0_2e_square_alpha_texture_blended(),
                0x2f => self.gp0_2f_square_alpha_texture_raw(),
                0x30 => self.gp0_30_shaded_triangle(),
                0x32 => self.gp0_32_shaded_triangle_alpha(),
                0x34 => self.gp0_34_shaded_textured_triangle_blend(),
                0x36 => self.gp0_36_shaded_textured_triangle_alpha_blend(),
                0x38 => self.gp0_38_shaded_square(),
                0x3a => self.gp0_3a_shaded_square_alpha(),
                0x3c => self.gp0_3c_shaded_textured_square_blend(),
                0x3e => self.gp0_3e_shaded_textured_square_alpha_blend(),
                0x40 => self.gp0_40_mono_line(),
                0x42 => self.gp0_42_mono_line_alpha(),
                0x48 => self.gp0_48_mono_polyline(),
                0x4a => self.gp0_4a_mono_polyline_alpha(),
                0x50 => self.gp0_50_shaded_line(),
                0x52 => self.gp0_52_shaded_line_alpha(),
                0x58 => self.gp0_58_shaded_polyline(),
                0x5a => self.gp0_5a_shaded_polyline_alpha(),
                0x60 => self.gp0_60_mono_rectangle(),
                0x62 => self.gp0_62_mono_rectangle_alpha(),
                0x68 => self.gp0_68_mono_rectangle_dot(),
                0x6a => self.gp0_6a_mono_rectangle_dot_alpha(),
                0x70 => self.gp0_70_mono_rectangle_8(),
                0x72 => self.gp0_72_mono_rectangle_8_alpha(),
                0x78 => self.gp0_78_mono_rectangle_16(),
                0x7a => self.gp0_7a_mono_rectangle_16_alpha(),
                0x64 => self.gp0_64_textured_rectangle_blend(),
                0x65 => self.gp0_65_textured_rectangle_raw(),
                0x66 => self.gp0_66_textured_rectangle_alpha_blend(),
                0x67 => self.gp0_67_textured_rectangle_alpha_raw(),
                0x6c => self.gp0_6c_textured_rectangle_dot_blend(),
                0x6d => self.gp0_6d_textured_rectangle_dot_raw(),
                0x6e => self.gp0_6e_textured_rectangle_dot_alpha_blend(),
                0x6f => self.gp0_6f_textured_rectangle_dot_alpha_raw(),
                0x74 => self.gp0_74_textured_rectangle_8_blend(),
                0x75 => self.gp0_75_textured_rectangle_8_raw(),
                0x76 => self.gp0_76_textured_rectangle_8_alpha_blend(),
                0x77 => self.gp0_77_textured_rectangle_8_alpha_raw(),
                0x7c => self.gp0_7c_textured_rectangle_16_blend(),
                0x7d => self.gp0_7d_textured_rectangle_16_raw(),
                0x7e => self.gp0_7e_textured_rectangle_16_alpha_blend(),
                0x7f => self.gp0_7f_textured_rectangle_16_alpha_raw(),
                0x80..=0x9f => self.gp0_80_copy_vram_vram(),
                0xa0..=0xbf => self.gp0_a0_copy_cpu_vram(),
                0xc0..=0xdf => self.gp0_c0_copy_vram_cpu(),
                0xe1 => self.gp0_e1_draw_mode(),
                0xe2 => self.gp0_e2_texture_window(),
                0xe3 => self.gp0_e3_drawing_area_top_left(),
                0xe4 => self.gp0_e4_drawing_area_bottom_right(),
                0xe5 => self.gp0_e5_drawing_offset(),
                0xe6 => self.gp0_e6_mask_bit(),
                0x21
                | 0x23
                | 0x29
                | 0x2b
                | 0x31
                | 0x33
                | 0x35
                | 0x37
                | 0x39
                | 0x3b
                | 0x3d
                | 0x3f
                | 0x41
                | 0x43..=0x47
                | 0x49
                | 0x4a..=0x4f
                | 0x51
                | 0x53..=0x57
                | 0x59
                | 0x5b..=0x5f
                | 0x61
                | 0x63
                | 0x69
                | 0x6b
                | 0x71
                | 0x73
                | 0x79
                | 0x7b
                | 0xf0..=0xff => {
                    // println!("[GPU] GP0({:02x}): unknown/garbage", opcode);
                }
            }

            if !(0xa0..=0xbf).contains(&opcode) {
                self.buffer.clear();
            }
        }
    }

    // also GP0(04..=1E, E0, E7..=EF)
    fn gp0_00_nop(&mut self) {
        // println!("[GPU] GP0(00): Nop");
    }

    fn gp0_01_clear_cache(&mut self) {
        // flush texture cache?
        // println!("[GPU] GP0(01): Flush texture cache");
    }

    // +2
    fn gp0_02_fill_rectangle(&mut self) {
        let color_bgr24 = self.buffer[0] & 0xff_ffff;
        let top_left_x = self.buffer[1] & 0xffff;
        let top_left_y = self.buffer[1] >> 16;
        let width = self.buffer[2] & 0xffff;
        let height = self.buffer[2] >> 16;

        println!(
            "[GPU] GP0(02): Fill rectangle from ({}, {}) with size {}x{} with BGR {:06x}",
            top_left_x, top_left_y, width, height, color_bgr24
        );
    }

    fn gp0_03_nop2(&mut self) {
        // println!("[GPU] GP0(03): Unknown (nop?)");
    }

    fn gp0_1f_interrupt_request(&mut self) {
        // println!("[GPU] GP0(1F): Interrupt request");
    }

    // +3
    fn gp0_20_mono_triangle(&mut self) {
        // BIOS

        let vertices = [
            PsxVertex::from_position_and_color(self.buffer[1], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[2], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[3], self.buffer[0]),
        ];

        self.renderer_tx
            .send(GpuCommand::DrawGouraudTriangle { vertices })
            .unwrap();
    }

    // 21 garbage

    // +3
    fn gp0_22_mono_triangle_alpha(&mut self) {
        println!("[GPU] GP0(22): mono_triangle_alpha");
    }

    // 23 garbage

    // +6
    fn gp0_24_triangle_texture_blended(&mut self) {
        println!("[GPU] GP0(24): triangle_texture_blended");
    }

    // +6
    fn gp0_25_triangle_texture_raw(&mut self) {
        println!("[GPU] GP0(25): triangle_texture_raw");
    }

    // +6
    fn gp0_26_triangle_alpha_texture_blended(&mut self) {
        println!("[GPU] GP0(26): triangle_alpha_texture_blended");
    }

    // +6
    fn gp0_27_triangle_alpha_texture_raw(&mut self) {
        println!("[GPU] GP0(27): triangle_alpha_texture_raw");
    }

    // +4
    fn gp0_28_mono_square(&mut self) {
        // BIOS
        // println!("[GPU] GP0(28): mono_square");

        let triangle1 = [
            PsxVertex::from_position_and_color(self.buffer[1], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[2], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[3], self.buffer[0]),
        ];

        let triangle2 = [
            PsxVertex::from_position_and_color(self.buffer[2], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[3], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[4], self.buffer[0]),
        ];

        self.renderer_tx
            .send(GpuCommand::DrawGouraudTriangle {
                vertices: triangle1,
            })
            .unwrap();
        self.renderer_tx
            .send(GpuCommand::DrawGouraudTriangle {
                vertices: triangle2,
            })
            .unwrap();
    }

    // 29 garbage

    // +4
    fn gp0_2a_mono_square_alpha(&mut self) {
        println!("[GPU] GP0(2a): mono_square_alpha");
    }

    // 2b garbage

    // +8
    fn gp0_2c_square_texture_blended(&mut self) {
        // BIOS
        let mod_r = (self.buffer[0] & 0x0000FF) as u8;
        let mod_g = ((self.buffer[0] >> 8) & 0x0000FF) as u8;
        let mod_b = ((self.buffer[0] >> 16) & 0x0000FF) as u8;

        let clut_attr = (self.buffer[2] >> 16) as u16;
        let texpage_attr = (self.buffer[4] >> 16) as u16;

        let y0 = (self.buffer[1] >> 16) as i16;
        let x0 = (self.buffer[1] & 0xFFFF) as i16;
        let u0 = (self.buffer[2] & 0xFF) as u8;
        let v0 = ((self.buffer[2] >> 8) & 0xFF) as u8;

        let y1 = (self.buffer[3] >> 16) as i16;
        let x1 = (self.buffer[3] & 0xFFFF) as i16;
        let u1 = (self.buffer[4] & 0xFF) as u8;
        let v1 = ((self.buffer[4] >> 8) & 0xFF) as u8;

        let y2 = (self.buffer[5] >> 16) as i16;
        let x2 = (self.buffer[5] & 0xFFFF) as i16;
        let u2 = (self.buffer[6] & 0xFF) as u8;
        let v2 = ((self.buffer[6] >> 8) & 0xFF) as u8;

        let y3 = (self.buffer[7] >> 16) as i16;
        let x3 = (self.buffer[7] & 0xFFFF) as i16;
        let u3 = (self.buffer[8] & 0xFF) as u8;
        let v3 = ((self.buffer[8] >> 8) & 0xFF) as u8;

        self.renderer_tx
            .send(GpuCommand::DrawTexturedQuad {
                vertices: [
                    PsxPrimitiveVertex { x: x0, y: y0 },
                    PsxPrimitiveVertex { x: x1, y: y1 },
                    PsxPrimitiveVertex { x: x2, y: y2 },
                    PsxPrimitiveVertex { x: x3, y: y3 },
                ],
                uvs: [
                    PsxUv { u: u0, v: v0 },
                    PsxUv { u: u1, v: v1 },
                    PsxUv { u: u2, v: v2 },
                    PsxUv { u: u3, v: v3 },
                ],
                clut_attr,
                texpage_attr,
                modulation_color: PsxColor {
                    r: mod_r,
                    g: mod_g,
                    b: mod_b,
                },
            })
            .unwrap();
    }

    // +8
    fn gp0_2d_square_texture_raw(&mut self) {
        println!("[GPU] GP0(2d): square_texture_raw");
    }

    // +8
    fn gp0_2e_square_alpha_texture_blended(&mut self) {
        println!("[GPU] GP0(2e): square_alpha_texture_blended");
    }

    // +8
    fn gp0_2f_square_alpha_texture_raw(&mut self) {
        println!("[GPU] GP0(2f): square_alpha_texture_raw");
    }

    // +5
    fn gp0_30_shaded_triangle(&mut self) {
        // BIOS
        // println!("[GPU] GP0(30): shaded_triangle");

        let vertices = [
            PsxVertex::from_position_and_color(self.buffer[1], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[3], self.buffer[2]),
            PsxVertex::from_position_and_color(self.buffer[5], self.buffer[4]),
        ];

        self.renderer_tx
            .send(GpuCommand::DrawGouraudTriangle { vertices })
            .unwrap();
    }

    // 31 garbage

    // +5
    fn gp0_32_shaded_triangle_alpha(&mut self) {
        println!("[GPU] GP0(32): shaded_triangle_alpha");
    }

    // 33 garbage

    // +8
    fn gp0_34_shaded_textured_triangle_blend(&mut self) {
        println!("[GPU] GP0(34): shaded_textured_triangle_blend");
    }

    // 35 garbage

    // +8
    fn gp0_36_shaded_textured_triangle_alpha_blend(&mut self) {
        println!("[GPU] GP0(36): shaded_textured_triangle_alpha_blend");
    }

    // 37 garbage

    // +7
    fn gp0_38_shaded_square(&mut self) {
        // BIOS
        // println!("[GPU] GP0(38): shaded_square, {:?}", self.buffer);

        let triangle1 = [
            PsxVertex::from_position_and_color(self.buffer[1], self.buffer[0]),
            PsxVertex::from_position_and_color(self.buffer[3], self.buffer[2]),
            PsxVertex::from_position_and_color(self.buffer[5], self.buffer[4]),
        ];

        let triangle2 = [
            PsxVertex::from_position_and_color(self.buffer[3], self.buffer[2]),
            PsxVertex::from_position_and_color(self.buffer[5], self.buffer[4]),
            PsxVertex::from_position_and_color(self.buffer[7], self.buffer[6]),
        ];

        self.renderer_tx
            .send(GpuCommand::DrawGouraudTriangle {
                vertices: triangle1,
            })
            .unwrap();
        self.renderer_tx
            .send(GpuCommand::DrawGouraudTriangle {
                vertices: triangle2,
            })
            .unwrap();
    }

    // 39 garbage

    // +7
    fn gp0_3a_shaded_square_alpha(&mut self) {
        println!("[GPU] GP0(3a): shaded_square_alpha");
    }

    // 3b garbage

    // +11
    fn gp0_3c_shaded_textured_square_blend(&mut self) {
        println!("[GPU] GP0(3c): shaded_textured_square_blend");
    }

    // 3d garbage

    // +11
    fn gp0_3e_shaded_textured_square_alpha_blend(&mut self) {
        println!("[GPU] GP0(3e): shaded_textured_square_alpha_blend");
    }

    // 3f garbage

    // +infinite until 0x5555_5555
    fn gp0_40_mono_line(&mut self) {
        println!("[GPU] GP0(40): mono_line");
    }

    // +infinite until 0x5555_5555
    fn gp0_42_mono_line_alpha(&mut self) {
        println!("[GPU] GP0(42): mono_line_alpha");
    }

    // +infinite until 0x5555_5555
    fn gp0_48_mono_polyline(&mut self) {
        println!("[GPU] GP0(48): mono_polyline");
    }

    // +infinite until 0x5555_5555
    fn gp0_4a_mono_polyline_alpha(&mut self) {
        println!("[GPU] GP0(4a): mono_polyline_alpha");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_50_shaded_line(&mut self) {
        println!("[GPU] GP0(50): shaded_line");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_52_shaded_line_alpha(&mut self) {
        println!("[GPU] GP0(52): shaded_line_alpha");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_58_shaded_polyline(&mut self) {
        println!("[GPU] GP0(58): shaded_polyline");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_5a_shaded_polyline_alpha(&mut self) {
        println!("[GPU] GP0(5a): shaded_polyline_alpha");
    }

    // +2
    fn gp0_60_mono_rectangle(&mut self) {
        println!("[GPU] GP0(60): mono_rectangle");
    }

    // +2
    fn gp0_62_mono_rectangle_alpha(&mut self) {
        println!("[GPU] GP0(62): mono_rectangle_alpha");
    }

    // +1
    fn gp0_68_mono_rectangle_dot(&mut self) {
        let rgb = self.buffer[0] & 0xFFFFFF;

        let vertex = PsxVertex::from_position_and_color(self.buffer[1], rgb);
        let mut v2 = vertex.clone();
        let mut v3 = vertex.clone();

        v2.x += 1;
        v3.y += 1;

        self.renderer_tx
            .send(GpuCommand::DrawGouraudTriangle {
                vertices: [vertex, v2, v3],
            })
            .unwrap();
    }

    // +1
    fn gp0_6a_mono_rectangle_dot_alpha(&mut self) {
        println!("[GPU] GP0(6a): mono_rectangle_dot_alpha");
    }

    // +1
    fn gp0_70_mono_rectangle_8(&mut self) {
        println!("[GPU] GP0(70): mono_rectangle_8");
    }

    // +1
    fn gp0_72_mono_rectangle_8_alpha(&mut self) {
        println!("[GPU] GP0(72): mono_rectangle_8_alpha");
    }

    // +1
    fn gp0_78_mono_rectangle_16(&mut self) {
        println!("[GPU] GP0(78): mono_rectangle_16");
    }

    // +1
    fn gp0_7a_mono_rectangle_16_alpha(&mut self) {
        println!("[GPU] GP0(7a): mono_rectangle_16_alpha");
    }

    // +3
    fn gp0_64_textured_rectangle_blend(&mut self) {
        // BIOS TODO
        println!("[GPU] GP0(64): textured_rectangle_blend, {:?}", self.buffer);

        // let top_left = Position::parse(self.buffer[1]);

        // let size = Position::parse(self.buffer[3]);

        // let positions = [
        //     PsxPrimitiveVertex{x: top_left.0, y: top_left.1},
        //     PsxPrimitiveVertex{x: top_left.0 + size.0, y:top_left.1},
        //     PsxPrimitiveVertex{x: top_left.0, y:top_left.1 + size.1},
        //     PsxPrimitiveVertex{x: top_left.0 + size.0, y:top_left.1 +
        // size.1}, ];

        // let clut = self.buffer[2] >> 16;
        // let uv = self.buffer[2] & 0xFFFF;
        // let u0 = (uv & 0xFF) as u8;
        // let v0 = ((uv >> 8) & 0xFF) as u8;

        // self.renderer_tx.send(GpuCommand::DrawTexturedQuad {
        //     vertices: positions,
        //     uvs: [
        //         PsxUv { u: u0, v: v0 },
        //         PsxUv { u: u0, v: v0 },
        //         PsxUv { u: u0, v: v0 },
        //         PsxUv { u: u0, v: v0 },
        //     ],
        //     clut_attr: clut as u16,
        //     texpage_attr: self.texpage_e1 as u16,
        //     modulation_color: PsxColor { r: 128, g: 128, b: 128 },
        // }).unwrap();
    }

    // +3
    fn gp0_65_textured_rectangle_raw(&mut self) {
        println!("[GPU] GP0(65): textured_rectangle_raw");
    }

    // +3
    fn gp0_66_textured_rectangle_alpha_blend(&mut self) {
        println!("[GPU] GP0(66): textured_rectangle_alpha_blend");
    }

    // +3
    fn gp0_67_textured_rectangle_alpha_raw(&mut self) {
        println!("[GPU] GP0(67): textured_rectangle_alpha_raw");
    }

    // +2
    fn gp0_6c_textured_rectangle_dot_blend(&mut self) {
        println!("[GPU] GP0(6c): textured_rectangle_dot_blend");
    }

    // +2
    fn gp0_6d_textured_rectangle_dot_raw(&mut self) {
        println!("[GPU] GP0(6d): textured_rectangle_dot_raw");
    }

    // +2
    fn gp0_6e_textured_rectangle_dot_alpha_blend(&mut self) {
        println!("[GPU] GP0(6e): textured_rectangle_dot_alpha_blend");
    }

    // +2
    fn gp0_6f_textured_rectangle_dot_alpha_raw(&mut self) {
        println!("[GPU] GP0(6f): textured_rectangle_dot_alpha_raw");
    }

    // +2
    fn gp0_74_textured_rectangle_8_blend(&mut self) {
        println!("[GPU] GP0(74): textured_rectangle_8_blend");
    }

    // +2
    fn gp0_75_textured_rectangle_8_raw(&mut self) {
        println!("[GPU] GP0(75): textured_rectangle_8_raw");
    }

    // +2
    fn gp0_76_textured_rectangle_8_alpha_blend(&mut self) {
        println!("[GPU] GP0(76): textured_rectangle_8_alpha_blend");
    }

    // +2
    fn gp0_77_textured_rectangle_8_alpha_raw(&mut self) {
        println!("[GPU] GP0(77): textured_rectangle_8_alpha_raw");
    }

    // +2
    fn gp0_7c_textured_rectangle_16_blend(&mut self) {
        println!("[GPU] GP0(7c): textured_rectangle_16_blend");
    }

    // +2
    fn gp0_7d_textured_rectangle_16_raw(&mut self) {
        println!("[GPU] GP0(7d): textured_rectangle_16_raw");
    }

    // +2
    fn gp0_7e_textured_rectangle_16_alpha_blend(&mut self) {
        println!("[GPU] GP0(7e): textured_rectangle_16_alpha_blend");
    }

    // +2
    fn gp0_7f_textured_rectangle_16_alpha_raw(&mut self) {
        println!("[GPU] GP0(7f): textured_rectangle_16_alpha_raw");
    }

    // +3
    fn gp0_80_copy_vram_vram(&mut self) {
        // BIOS TODO
        println!("[GPU] GP0(80): copy_vram_vram");
    }

    // +2 +(width * height)
    fn gp0_a0_copy_cpu_vram(&mut self) {
        // BIOS
        if self.buffer.len() == 3 {
            // Check 3rd word, multiply high and low halfword
            // that's the number of remaining halfwords to read.

            let size = self.buffer[2] as usize;
            let width = size & 0xffff;
            let height = size >> 16;

            let halfwords = width * height;
            self.remaining_words = if halfwords % 2 == 0 {
                halfwords / 2
            } else {
                halfwords / 2 + 1
            };
        }

        if self.remaining_words == 0 {
            let size = self.buffer[2] as u32;
            let width = (size & 0xffff) as u16;
            let height = (size >> 16) as u16;

            let x = (self.buffer[1] & 0x3ff) as u16;
            let y = ((self.buffer[1] >> 16) & 0x1ff) as u16;

            // Transform the sequence of u32 into a sequence of u16
            let halfwords = self.buffer[3..]
                .iter()
                .flat_map(|&word| [word as u16, (word >> 16) as u16])
                .collect::<Vec<u16>>();

            // println!(
            //     "[GPU] GP0(a0): copy_cpu_vram to ({}, {}) with size {}x{} with {}
            // halfwords",     x, y, width, height, halfwords.len()
            // );

            self.renderer_tx
                .send(GpuCommand::WriteToVram {
                    x,
                    y,
                    w: width,
                    h: height,
                    pixel_data: halfwords,
                })
                .unwrap();

            self.buffer.clear();
        }
    }

    // +2 +(width * height)
    fn gp0_c0_copy_vram_cpu(&mut self) {
        println!("[GPU] GP0(c0): copy_vram_cpu");
        // if self.buffer.len() == 3 {
        // Check 3rd word, multiply high and low halfword
        // that's the number of remaining halfwords to read.

        // let size = self.buffer[2] as usize;
        // let width = size & 0xffff;
        // let height = size >> 16;
        // let size = width * height;
        // println!("Remaining {}x{} = {}", width, height, size);

        // Yeah, the other way around...
        // self.remaining_words = if size % 2 == 0 { size / 2 } else { size / 2
        // + 1}; } else {
        // println!("[GPU] Copy with {} words", self.buffer.len());
        // }
    }

    fn gp0_e1_draw_mode(&mut self) {
        // println!("[GPU] GP0(e1): draw_mode, {:08x}", self.buffer[0]);
        self.texpage_e1 = self.buffer[0] & 0x3fff;
    }

    fn gp0_e2_texture_window(&mut self) {
        // BIOS TODO
        // println!("[GPU] GP0(e2): texture_window, {:08x}", self.buffer[0]);
    }

    fn gp0_e3_drawing_area_top_left(&mut self) {
        // BIOS

        let val = self.buffer[0];

        self.drawing_area_top = ((val >> 10) & 0x3ff) as u16;
        self.drawing_area_left = (val & 0x3ff) as u16;

        self.update_drawing_area();
    }

    fn gp0_e4_drawing_area_bottom_right(&mut self) {
        // BIOS

        let val = self.buffer[0];

        self.drawing_area_bottom = ((val >> 10) & 0x3ff) as u16;
        self.drawing_area_right = (val & 0x3ff) as u16;

        self.update_drawing_area();
    }

    fn update_drawing_area(&mut self) {
        self.renderer_tx
            .send(GpuCommand::SetDrawingArea {
                x1: self.drawing_area_left,
                y1: self.drawing_area_top,
                x2: self.drawing_area_right,
                y2: self.drawing_area_bottom,
            })
            .unwrap();
    }

    fn gp0_e5_drawing_offset(&mut self) {
        // BIOS
        let val = self.buffer[0];

        let x = (val & 0x7ff) as u16;
        let y = ((val >> 11) & 0x7ff) as u16;

        // Values are 11bit two's complement signed values, we need to
        // shift the value to 16bits to force sign extension
        let x = ((x << 5) as i16) >> 5;
        let y = ((y << 5) as i16) >> 5;

        self.drawing_offset = (x, y);

        // if let Some(renderer) = &mut self.renderer {
        //     renderer.set_draw_offset(x, y);
        // }
    }

    fn gp0_e6_mask_bit(&mut self) {
        // BIOS TODO
        // println!("[GPU] GP0(e6): mask_bit, mask: {:08x}", self.buffer[0] &
        // 3);
    }

    fn process_gp1(&mut self, command: u32) {
        let opcode = command >> 24;
        let arguments = command & 0xff_ffff;

        match opcode {
            0x00 => {
                // println!("[GPU] GP1(0): NOP");
                self.gpustat.0 = 0x1480_2000;
            }
            0x01 => {
                // println!("[GPU] GP1(1): clear fifo");
                self.buffer.clear();
                self.remaining_words = 0;
            }
            0x02 => {
                // println!("[GPU] GP1(2): ACK IRQ");
            }
            0x03 => {
                self.gpustat.set_display_enable(arguments & 1 != 0);
                // println!("[GPU] GP1(3): Display enable: {}", arguments & 1);
            }
            0x04 => {
                // println!("[GPU] GP1(4): DMA Direction: {}", arguments & 3);
            }
            0x05 => {
                // println!("[GPU] GP1(5): Start of display area {} {}",
                self.display_left = (arguments & 0x3ff) as u16;
                self.display_top = ((arguments >> 10) & 0x1ff) as u16;

                self.renderer_tx
                    .send(GpuCommand::SetDisplayArea {
                        x: self.display_left,
                        y: self.display_top,
                        w: self.horizontal_res,
                        h: self.vertical_res,
                    })
                    .unwrap();
            }
            0x06 => {
                // println!("[GPU] GP1(6): Horizontal display range {} {}",
                // arguments & 0xfff, (arguments >> 12) & 0xfff);
            }
            0x07 => {
                // println!("[GPU] GP1(7): Vertical display range {} {}",
                // arguments & 0x3ff, (arguments >> 10) & 0x3ff);
            }
            0x08 => {
                self.gpustat.0 &= !(0x7F_4000);
                self.gpustat.0 |= (arguments & 0x80) << 7;
                self.gpustat.0 |= (arguments & 0x40) << 10;
                self.gpustat.0 |= (arguments & 0x3f) << 17;

                let horizontal_resolution_idx = (arguments & 0x3) | ((arguments & 0x40) >> 4);
                self.horizontal_res = match horizontal_resolution_idx {
                    0 => 256,
                    1 => 320,
                    2 => 512,
                    3 => 640,
                    _ => 368,
                };

                self.vertical_res = if (arguments & 0x40) != 0 { 480 } else { 240 };

                // TODO: understand why the BIOS sets 640x240, which is wrong
                self.vertical_res = 480;

                self.renderer_tx
                    .send(GpuCommand::SetDisplayArea {
                        x: self.display_left,
                        y: self.display_top,
                        w: self.horizontal_res,
                        h: self.vertical_res,
                    })
                    .unwrap();
            }
            0x10..=0x1f => {
                // println!("[GPU] Unimplemented GP1(0x10): Get GPU info");
            }
            _ => {
                panic!("[GPU] Unknown GP1 opcode: {:02x}", opcode)
            }
        }
    }

    // fn is_ntsc(&self) -> bool {
    //     !self.gpustat.video_mode()
    // }

    // fn is_pal(&self) -> bool {
    //     !self.is_ntsc()
    // }

    // fn scanlines(&self) -> u32 {
    //     if self.is_ntsc() {
    //         263
    //     } else {
    //         314
    //     }
    // }
}
