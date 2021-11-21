mod renderer;
mod shaders;

use std::cell::RefCell;
use std::rc::Weak;

use bitfield::bitfield;
use renderer::{Color, Position, Renderer};

use crate::hw::bus::{Bus, BusDevice, PsxEventType, R3000Type};

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
    renderer: Option<Renderer>,

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

    bus: Weak<RefCell<Bus>>,

    set: bool,
}

impl Gpu {
    pub fn new() -> Gpu {
        Gpu {
            renderer: None,

            gpustat: GpuStat(0x1480_2000),
            buffer: vec![],
            remaining_words: 0,

            drawing_area_left: 0,
            drawing_area_top: 0,
            drawing_area_right: 0,
            drawing_area_bottom: 0,
            drawing_offset: (0, 0),

            bus: Weak::new(),

            set: false,
        }
    }

    pub fn link(&mut self, bus: Weak<RefCell<Bus>>) {
        self.bus = bus;
    }

    pub fn load_renderer(&mut self) {
        self.renderer = Some(Renderer::new());
    }
}

impl BusDevice for Gpu {
    fn write<T: R3000Type>(&mut self, addr: u32, value: u32) {
        if !self.set {
            let cpu_freq = 33868800;
            let vblank_freq = 60;
            let vblank_cycles = cpu_freq / vblank_freq;
            self.bus
                .upgrade()
                .unwrap()
                .borrow()
                .add_event(PsxEventType::VBlank, 0, vblank_cycles);
            self.set = true;
        }

        if std::mem::size_of::<T>() != 4 {
            // println!("Unhandled {}-bytes GPU read", std::mem::size_of::<T>());
        }

        match addr {
            0 => self.process_gp0(value),
            4 => self.process_gp1(value),
            _ => panic!("Invalid write to gpu"),
        }
    }

    fn read<T: R3000Type>(&mut self, addr: u32) -> u32 {
        if std::mem::size_of::<T>() != 4 {
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
        self.bus.upgrade().unwrap().borrow().send_irq(0);

        if let Some(renderer) = &mut self.renderer {
            renderer.draw();
        }
    }

    pub fn process_gp0(&mut self, command: u32) {
        println!("[GP0] {:08x}", command);

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

        println!("[GPU] GP0(02): Fill rectangle from ({}, {}) with size {}x{} with BGR {:06x}",
            top_left_x, top_left_y,
            width, height,
            color_bgr24);
    }

    fn gp0_03_nop2(&mut self) {
        // println!("[GPU] GP0(03): Unknown (nop?)");
    }

    fn gp0_1f_interrupt_request(&mut self) {
        // println!("[GPU] GP0(1F): Interrupt request");
    }

    // +3
    fn gp0_20_mono_triangle(&mut self) {
        println!("[GPU] GP0(20): mono_triangle");

        let vertices = [
            Position::parse(self.buffer[1]),
            Position::parse(self.buffer[2]),
            Position::parse(self.buffer[3]),
        ];

        let colors = [
            Color::parse(self.buffer[0]),
            Color::parse(self.buffer[0]),
            Color::parse(self.buffer[0]),
        ];

        println!("Triangle at {:?} with colors {:?}", vertices, colors);

        if let Some(renderer) = &mut self.renderer {
            renderer.push_triangle(vertices, colors);
        }
    }

    // 21 garbage

    // +3
    fn gp0_22_mono_triangle_alpha(&mut self) {
        // println!("[GPU] GP0(22): mono_triangle_alpha");
    }

    // 23 garbage

    // +6
    fn gp0_24_triangle_texture_blended(&mut self) {
        // println!("[GPU] GP0(24): triangle_texture_blended");
    }

    // +6
    fn gp0_25_triangle_texture_raw(&mut self) {
        // println!("[GPU] GP0(25): triangle_texture_raw");
    }

    // +6
    fn gp0_26_triangle_alpha_texture_blended(&mut self) {
        // println!("[GPU] GP0(26): triangle_alpha_texture_blended");
    }

    // +6
    fn gp0_27_triangle_alpha_texture_raw(&mut self) {
        // println!("[GPU] GP0(27): triangle_alpha_texture_raw");
    }

    // +4
    fn gp0_28_mono_square(&mut self) {
        // println!("[GPU] GP0(28): mono_square");

        let positions = [
            Position::parse(self.buffer[1]),
            Position::parse(self.buffer[2]),
            Position::parse(self.buffer[3]),
            Position::parse(self.buffer[4]),
        ];

        // Only one color repeated 4 times
        let colors = [Color::parse(self.buffer[0]); 4];

        if let Some(renderer) = &mut self.renderer {
            renderer.push_quad(positions, colors);
        }
    }

    // 29 garbage

    // +4
    fn gp0_2a_mono_square_alpha(&mut self) {
        // println!("[GPU] GP0(2a): mono_square_alpha");
    }

    // 2b garbage

    // +8
    fn gp0_2c_square_texture_blended(&mut self) {
        // println!("[GPU] GP0(2c): square_texture_blended");
    }

    // +8
    fn gp0_2d_square_texture_raw(&mut self) {
        // println!("[GPU] GP0(2d): square_texture_raw");
    }

    // +8
    fn gp0_2e_square_alpha_texture_blended(&mut self) {
        // println!("[GPU] GP0(2e): square_alpha_texture_blended");
    }

    // +8
    fn gp0_2f_square_alpha_texture_raw(&mut self) {
        // println!("[GPU] GP0(2f): square_alpha_texture_raw");
    }

    // +5
    fn gp0_30_shaded_triangle(&mut self) {
        // println!("[GPU] GP0(30): shaded_triangle");

        let vertices = [
            Position::parse(self.buffer[1]),
            Position::parse(self.buffer[3]),
            Position::parse(self.buffer[5]),
        ];

        let colors = [
            Color::parse(self.buffer[0]),
            Color::parse(self.buffer[2]),
            Color::parse(self.buffer[4]),
        ];

        if let Some(renderer) = &mut self.renderer {
            renderer.push_triangle(vertices, colors);
        }
    }

    // 31 garbage

    // +5
    fn gp0_32_shaded_triangle_alpha(&mut self) {
        // println!("[GPU] GP0(32): shaded_triangle_alpha");
    }

    // 33 garbage

    // +8
    fn gp0_34_shaded_textured_triangle_blend(&mut self) {
        // println!("[GPU] GP0(34): shaded_textured_triangle_blend");
    }

    // 35 garbage

    // +8
    fn gp0_36_shaded_textured_triangle_alpha_blend(&mut self) {
        // println!("[GPU] GP0(36): shaded_textured_triangle_alpha_blend");
    }

    // 37 garbage

    // +7
    fn gp0_38_shaded_square(&mut self) {
        // println!("[GPU] GP0(38): shaded_square");

        let positions = [
            Position::parse(self.buffer[1]),
            Position::parse(self.buffer[3]),
            Position::parse(self.buffer[5]),
            Position::parse(self.buffer[7]),
        ];

        let colors = [
            Color::parse(self.buffer[0]),
            Color::parse(self.buffer[2]),
            Color::parse(self.buffer[4]),
            Color::parse(self.buffer[6]),
        ];

        if let Some(renderer) = &mut self.renderer {
            renderer.push_quad(positions, colors);
        }
    }

    // 39 garbage

    // +7
    fn gp0_3a_shaded_square_alpha(&mut self) {
        // println!("[GPU] GP0(3a): shaded_square_alpha");
    }

    // 3b garbage

    // +11
    fn gp0_3c_shaded_textured_square_blend(&mut self) {
        // println!("[GPU] GP0(3c): shaded_textured_square_blend");
    }

    // 3d garbage

    // +11
    fn gp0_3e_shaded_textured_square_alpha_blend(&mut self) {
        // println!("[GPU] GP0(3e): shaded_textured_square_alpha_blend");
    }

    // 3f garbage

    // +infinite until 0x5555_5555
    fn gp0_40_mono_line(&mut self) {
        // println!("[GPU] GP0(40): mono_line");
    }

    // +infinite until 0x5555_5555
    fn gp0_42_mono_line_alpha(&mut self) {
        // println!("[GPU] GP0(42): mono_line_alpha");
    }

    // +infinite until 0x5555_5555
    fn gp0_48_mono_polyline(&mut self) {
        // println!("[GPU] GP0(48): mono_polyline");
    }

    // +infinite until 0x5555_5555
    fn gp0_4a_mono_polyline_alpha(&mut self) {
        // println!("[GPU] GP0(4a): mono_polyline_alpha");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_50_shaded_line(&mut self) {
        // println!("[GPU] GP0(50): shaded_line");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_52_shaded_line_alpha(&mut self) {
        // println!("[GPU] GP0(52): shaded_line_alpha");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_58_shaded_polyline(&mut self) {
        // println!("[GPU] GP0(58): shaded_polyline");
    }

    // +2*infinite until 0x5555_5555
    fn gp0_5a_shaded_polyline_alpha(&mut self) {
        // println!("[GPU] GP0(5a): shaded_polyline_alpha");
    }

    // +2
    fn gp0_60_mono_rectangle(&mut self) {
        // println!("[GPU] GP0(60): mono_rectangle");
    }

    // +2
    fn gp0_62_mono_rectangle_alpha(&mut self) {
        // println!("[GPU] GP0(62): mono_rectangle_alpha");
    }

    // +1
    fn gp0_68_mono_rectangle_dot(&mut self) {
        // println!("[GPU] GP0(68): mono_rectangle_dot");
    }

    // +1
    fn gp0_6a_mono_rectangle_dot_alpha(&mut self) {
        // println!("[GPU] GP0(6a): mono_rectangle_dot_alpha");
    }

    // +1
    fn gp0_70_mono_rectangle_8(&mut self) {
        // println!("[GPU] GP0(70): mono_rectangle_8");
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
        println!("[GPU] GP0(64): textured_rectangle_blend");

        let top_left = Position::parse(self.buffer[1]);

        let size = Position::parse(self.buffer[3]);

        let positions = [
            top_left,
            Position(top_left.0 + size.0, top_left.1),
            Position(top_left.0, top_left.1 + size.1),
            Position(top_left.0 + size.0, top_left.1 + size.1),
        ];

        let colors = [Color::parse(self.buffer[0]); 4];

        if let Some(renderer) = &mut self.renderer {
            renderer.push_quad(positions, colors);
        }
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
        // println!("[GPU] GP0(74): textured_rectangle_8_blend");
    }

    // +2
    fn gp0_75_textured_rectangle_8_raw(&mut self) {
        // println!("[GPU] GP0(75): textured_rectangle_8_raw");
    }

    // +2
    fn gp0_76_textured_rectangle_8_alpha_blend(&mut self) {
        // println!("[GPU] GP0(76): textured_rectangle_8_alpha_blend");
    }

    // +2
    fn gp0_77_textured_rectangle_8_alpha_raw(&mut self) {
        // println!("[GPU] GP0(77): textured_rectangle_8_alpha_raw");
    }

    // +2
    fn gp0_7c_textured_rectangle_16_blend(&mut self) {
        // println!("[GPU] GP0(7c): textured_rectangle_16_blend");
    }

    // +2
    fn gp0_7d_textured_rectangle_16_raw(&mut self) {
        // println!("[GPU] GP0(7d): textured_rectangle_16_raw");
    }

    // +2
    fn gp0_7e_textured_rectangle_16_alpha_blend(&mut self) {
        // println!("[GPU] GP0(7e): textured_rectangle_16_alpha_blend");
    }

    // +2
    fn gp0_7f_textured_rectangle_16_alpha_raw(&mut self) {
        // println!("[GPU] GP0(7f): textured_rectangle_16_alpha_raw");
    }

    // +3
    fn gp0_80_copy_vram_vram(&mut self) {
        // println!("[GPU] GP0(80): copy_vram_vram");
    }

    // +2 +(width * height)
    fn gp0_a0_copy_cpu_vram(&mut self) {
        // println!("[GPU] GP0(a0): copy_cpu_vram");
        if self.buffer.len() == 3 {
            // Check 3rd word, multiply high and low halfword
            // that's the number of remaining halfwords to read.

            let size = self.buffer[2] as usize;
            let width = size & 0xffff;
            let height = size >> 16;
            let size = width * height;
            // println!("Remaining {}x{} = {}", width, height, size);
            self.remaining_words = if size % 2 == 0 {
                size / 2
            } else {
                size / 2 + 1
            };
        } else {
            // println!("[GPU] Copy with {} words", self.buffer.len());
        }

        if self.remaining_words == 0 {
            self.buffer.clear();
        }
    }

    // +2 +(width * height)
    fn gp0_c0_copy_vram_cpu(&mut self) {
        // println!("[GPU] GP0(c0): copy_vram_cpu");
        // if self.buffer.len() == 3 {
        // Check 3rd word, multiply high and low halfword
        // that's the number of remaining halfwords to read.

        // let size = self.buffer[2] as usize;
        // let width = size & 0xffff;
        // let height = size >> 16;
        // let size = width * height;
        // println!("Remaining {}x{} = {}", width, height, size);

        // Yeah, the other way around...
        // self.remaining_words = if size % 2 == 0 { size / 2 } else { size / 2 + 1};
        // } else {
        // println!("[GPU] Copy with {} words", self.buffer.len());
        // }
    }

    fn gp0_e1_draw_mode(&mut self) {
        // println!("[GPU] GP0(e1): draw_mode");
    }

    fn gp0_e2_texture_window(&mut self) {
        // println!("[GPU] GP0(e2): texture_window");
    }

    fn gp0_e3_drawing_area_top_left(&mut self) {
        // println!("[GPU] GP0(e3): drawing_area_top_left");

        let val = self.buffer[0];

        self.drawing_area_top = ((val >> 10) & 0x3ff) as u16;
        self.drawing_area_left = (val & 0x3ff) as u16;

        self.update_drawing_area();
    }

    fn gp0_e4_drawing_area_bottom_right(&mut self) {
        // println!("[GPU] GP0(e4): drawing_area_bottom_right");

        let val = self.buffer[0];

        self.drawing_area_bottom = ((val >> 10) & 0x3ff) as u16;
        self.drawing_area_right = (val & 0x3ff) as u16;

        self.update_drawing_area();
    }

    fn update_drawing_area(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_drawing_area(
                self.drawing_area_left,
                self.drawing_area_top,
                self.drawing_area_right,
                self.drawing_area_bottom,
            );
        }
    }

    fn gp0_e5_drawing_offset(&mut self) {
        // println!("[GPU] GP0(e5): drawing_offset");

        let val = self.buffer[0];

        let x = (val & 0x7ff) as u16;
        let y = ((val >> 11) & 0x7ff) as u16;

        // Values are 11bit two's complement signed values, we need to
        // shift the value to 16bits to force sign extension
        let x = ((x << 5) as i16) >> 5;
        let y = ((y << 5) as i16) >> 5;

        self.drawing_offset = (x, y);

        if let Some(renderer) = &mut self.renderer {
            renderer.set_draw_offset(x, y);
        }
    }

    fn gp0_e6_mask_bit(&mut self) {
        // println!("[GPU] GP0(e6): mask_bit");
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
                // println!("[GPU] GP1(5): Start of display area {} {}", arguments & 0x3ff, (arguments >> 10) & 0x1ff);
            }
            0x06 => {
                // println!("[GPU] GP1(6): Horizontal display range {} {}", arguments & 0xfff, (arguments >> 12) & 0xfff);
            }
            0x07 => {
                // println!("[GPU] GP1(7): Vertical display range {} {}", arguments & 0x3ff, (arguments >> 10) & 0x3ff);
            }
            0x08 => {
                self.gpustat.0 &= !(0x7F_4000);
                self.gpustat.0 |= (arguments & 0x80) << 7;
                self.gpustat.0 |= (arguments & 0x40) << 10;
                self.gpustat.0 |= (arguments & 0x3f) << 17;

                // let cpu_freq = 33868800;
                // let vblank_freq = 60;
                // let vblank_cycles = cpu_freq / vblank_freq;
                // self.bus.upgrade().unwrap().borrow().add_event(PsxEventType::VBlank, 0, vblank_cycles);

                // println!("[GPU] GP1(08) - New GPUSTAT: {:08x}", self.gpustat.0);
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
