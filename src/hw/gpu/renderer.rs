// use gl::types::{GLint, GLshort, GLsizei, GLsizeiptr, GLubyte, GLuint};

// use std::mem::size_of;
// use std::ptr;
// use std::slice;

// use crate::hw::gpu::shaders::{
//     compile_shader, find_program_attrib, find_program_uniform, link_program,
// };

// pub struct Renderer {
//     /// SDL3 Window
//     #[allow(dead_code)]
//     // window: sdl3::video::Window,
//     /// OpenGL Context
//     #[allow(dead_code)]
//     // gl_context: sdl3::video::GLContext,
//     /// Framebuffer horizontal resolution (native: 1024)
//     fb_x_res: u16,
//     /// Framebuffer vertical resolution (native: 512)
//     fb_y_res: u16,
//     /// Vertex shader object
//     #[allow(dead_code)]
//     vertex_shader: GLuint,
//     /// Fragment shader object
//     #[allow(dead_code)]
//     fragment_shader: GLuint,
//     /// OpenGL Program object
//     #[allow(dead_code)]
//     program: GLuint,
//     /// OpenGL Vertex array object
//     #[allow(dead_code)]
//     vertex_array_object: GLuint,
//     /// Buffer containing the vertice positions
//     positions: Buffer<Position>,
//     /// Buffer containing the vertice colors
//     colors: Buffer<Color>,
//     /// Current number or vertices in the buffers
//     nvertices: u32,
//     /// Index of the "offset" shader uniform
//     uniform_offset: GLint,
// }

// impl Renderer {
//     pub fn new() -> Renderer {
//         // let sdl_context = sdl3::init().unwrap();
//         let video_subsystem = sdl_context.video().unwrap();

//         let gl_attr = video_subsystem.gl_attr();
//         gl_attr.set_context_profile(GLProfile::Core);
//         gl_attr.set_context_flags().debug().forward_compatible().set();
//         gl_attr.set_context_version(4, 1);
//         gl_attr.set_multisample_buffers(1);
//         gl_attr.set_multisample_samples(4);

//         let window = video_subsystem
//             .window("RPSX", 1024, 512)
//             .opengl()
//             .build()
//             .expect("Failed to create window");

//         let gl_context = window.gl_create_context().unwrap();
//         window.gl_make_current(&gl_context).expect("Failed to make GL context current");

//         gl::load_with(|s| {
//             video_subsystem.gl_get_proc_address(s).unwrap() as *const std::os::raw::c_void
//         });

//         unsafe {
//             gl::ClearColor(0.1, 0.1, 0.15, 1.0);
//             gl::Clear(gl::COLOR_BUFFER_BIT);
//             gl::Enable(gl::SCISSOR_TEST);
//             gl::Scissor(0, 0, 1024_i32, 512_i32);
//         }

//         video_subsystem.gl_set_swap_interval(0).unwrap();
//         window.gl_swap_window();

//         let vs_src = include_str!("shaders/vertex.glsl");
//         let fs_src = include_str!("shaders/fragment.glsl");

//         // Compile the shaders
//         let vertex_shader = compile_shader(vs_src, gl::VERTEX_SHADER);
//         let fragment_shader = compile_shader(fs_src, gl::FRAGMENT_SHADER);
//         // Link the shader program
//         let program = link_program(&[vertex_shader, fragment_shader]);
//         // Activate the program
//         unsafe {
//             gl::UseProgram(program);
//         }

//         let mut vao = 0;
//         unsafe {
//             gl::GenVertexArrays(1, &mut vao);
//             // Bind our VAO
//             gl::BindVertexArray(vao);
//         }

//         // Setup the "position" attribute. First we create the buffer
//         // holding the positions (this call also binds it)
//         let positions = Buffer::new();

//         unsafe {
//             // Then we retreive the index for the attribute in the
//             // shader
//             let index = find_program_attrib(program, "vertex_position");

//             // Enable it
//             gl::EnableVertexAttribArray(index);

//             // Link the buffer and the index: 2 GLshort attributes,
//             // not normalized. That should send the data untouched to
//             // the vertex shader.
//             gl::VertexAttribIPointer(index, 2, gl::SHORT, 0, ptr::null());
//         }

//         // Setup the "color" attribute and bind it
//         let colors = Buffer::new();

//         unsafe {
//             let index = find_program_attrib(program, "vertex_color");
//             gl::EnableVertexAttribArray(index);

//             // Link the buffer and the index: 3 GLByte attributes, normalized.
//             gl::VertexAttribPointer(index, 3, gl::UNSIGNED_BYTE, gl::TRUE, 0, ptr::null());
//         }

//         let uniform_offset = find_program_uniform(program, "offset");
//         unsafe {
//             gl::Uniform2i(uniform_offset, 0, 0);
//         }

//         // let mut event_pump = sdl_context.event_pump().expect("Failed to get event pump");

//         Renderer {
//             window,
//             gl_context,
//             // event_pump,
//             fb_x_res: 1024,
//             fb_y_res: 512,
//             vertex_shader,
//             fragment_shader,
//             program,
//             vertex_array_object: vao,
//             positions,
//             colors,
//             nvertices: 0,
//             uniform_offset,
//         }
//     }

//     pub fn push_triangle(&mut self, positions: [Position; 3], colors: [Color; 3]) {
//         // Make sure we have enough room left to queue the vertex
//         if self.nvertices + 3 > 64 * 1024 {
//             println!("Vertex attribute buffers full, forcing draw");
//             self.draw();
//         }

//         for i in 0..3 {
//             // Push
//             self.positions.set(self.nvertices, positions[i]);
//             self.colors.set(self.nvertices, colors[i]);
//             self.nvertices += 1;
//         }
//     }

//     pub fn draw(&mut self) {
//         unsafe {
//             // gl::ClearColor(0.1, 0.1, 0.15, 1.0);
//             // gl::Clear(gl::COLOR_BUFFER_BIT); // Add | gl::DEPTH_BUFFER_BIT if using depth testing
    
//             // Make sure all the data from the persistent mappings is
//             // flushed to the buffer
//             // gl::FlushMappedBufferRange(
//             //     gl::ARRAY_BUFFER,
//             //     0,
//             //     (self.nvertices * size_of::<Position>() as u32) as GLsizeiptr,
//             // );

//             gl::DrawArrays(gl::TRIANGLES, 0, self.nvertices as GLsizei);
//         }

//         // Wait for GPU to complete
//         // unsafe {
//         //     let sync = gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0);

//         //     loop {
//         //         let r = gl::ClientWaitSync(sync, gl::SYNC_FLUSH_COMMANDS_BIT, 10000000);

//         //         if r == gl::ALREADY_SIGNALED || r == gl::CONDITION_SATISFIED {
//         //             // Drawing done
//         //             break;
//         //         }
//         //     }
//         // }

//         // Reset the buffers
//         self.nvertices = 0;

//         self.window.gl_swap_window();
//     }

//     pub fn poll_events(&mut self) {
//         // for event in self.event_pump.poll_iter() {
//         //     match event {
//         //         sdl2::event::Event::Quit { .. } => {
//         //             println!("Quit event received");
//         //             std::process::exit(0);
//         //         }
//         //         _ => {}
//         //     }
//         // }
//     }

//     pub fn set_draw_offset(&mut self, x: i16, y: i16) {
//         // Force draw for the primitives with the current offset
//         self.draw();

//         // Update the uniform value
//         unsafe {
//             // println!("Setting offset to {} {}", x, y);
//             gl::Uniform2i(self.uniform_offset, x as GLint, y as GLint);
//         }
//     }

//     /// Set the drawing area. Coordinates are offsets in the
//     /// PlayStation VRAM
//     pub fn set_drawing_area(&mut self, left: u16, top: u16, right: u16, bottom: u16) {
//         // Render any pending primitives
//         self.draw();

//         // println!(
//         //     "Setting drawing area: [{}x{}->{}x{}]",
//         //     left,
//         //     top,
//         //     right,
//         //     bottom
//         // );

//         let fb_x_res = self.fb_x_res as GLint;
//         let fb_y_res = self.fb_y_res as GLint;

//         // Scale PlayStation VRAM coordinates if our framebuffer is
//         // not at the native resolution
//         let left = (left as GLint * fb_x_res) / 1024;
//         let right = (right as GLint * fb_x_res) / 1024;

//         let top = (top as GLint * fb_y_res) / 512;
//         let bottom = (bottom as GLint * fb_y_res) / 512;

//         // Width and height are inclusive
//         let width = right - left + 1;
//         let height = bottom - top + 1;

//         // OpenGL has (0, 0) at the bottom left, the PSX at the top left
//         let bottom = fb_y_res - bottom - 1;

//         if width < 0 || height < 0 {
//             // XXX What should we do here?
//             println!(
//                 "Unsupported drawing area: {}x{} [{}x{}->{}x{}]",
//                 width, height, left, top, right, bottom
//             );
//             unsafe {
//                 // Don't draw anything...
//                 gl::Scissor(0, 0, 0, 0);
//             }
//         } else {
//             // println!(
//             //     "Setting drawing area: {}x{} [{}x{}->{}x{}]",
//             //     width, height, left, top, right, bottom
//             // );
//             unsafe {
//                 gl::Scissor(left, bottom, width, height);
//             }
//         }
//     }

//     pub fn push_quad(&mut self, positions: [Position; 4], colors: [Color; 4]) {
//         // Make sure we have enough room left to queue the vertex. We
//         // need to push two triangles to draw a quad, so 6 vertex
//         if self.nvertices + 6 > 64 * 1024 {
//             self.draw();
//         }

//         // Push the first triangle
//         for i in 0..3 {
//             self.positions.set(self.nvertices, positions[i]);
//             self.colors.set(self.nvertices, colors[i]);
//             self.nvertices += 1;
//         }

//         // Push the 2nd triangle
//         for i in 1..4 {
//             self.positions.set(self.nvertices, positions[i]);
//             self.colors.set(self.nvertices, colors[i]);
//             self.nvertices += 1;
//         }
//     }
// }

// #[derive(Copy, Clone, Debug, Default)]
// pub struct Position(pub i16, pub i16);

// impl Position {
//     pub fn parse(value: u32) -> Position {
//         let x = value & 0xfff;
//         let y = value >> 16 & 0xfff;

//         Position(x as i16, y as i16)
//     }
// }

// #[derive(Copy, Clone, Debug, Default)]
// pub struct Color(pub GLubyte, pub GLubyte, pub GLubyte);

// impl Color {
//     pub fn parse(value: u32) -> Color {
//         let r = value & 0xff;
//         let g = (value >> 8) & 0xff;
//         let b = (value >> 16) & 0xff;

//         Color(r as GLubyte, g as GLubyte, b as GLubyte)
//     }
// }

// pub struct Buffer<T> {
//     object: GLuint,
//     map: *mut T,
// }

// impl<T: Copy + Default> Buffer<T> {
//     pub fn new() -> Buffer<T> {
//         let mut object = 0;
//         let memory;
//         const BUFFER_CAPACITY_ELEMENTS: usize = 64 * 1024;

//         unsafe {
//             // Generate the buffer ID
//             gl::GenBuffers(1, &mut object);
//             // Bind the buffer to the ARRAY_BUFFER target
//             gl::BindBuffer(gl::ARRAY_BUFFER, object);

//             // Calculate the size of the buffer in bytes
//             let element_size = size_of::<T>() as GLsizeiptr;
//             let buffer_size_bytes = element_size * (BUFFER_CAPACITY_ELEMENTS as GLsizeiptr);

//             // Allocate buffer data store using glBufferData.
//             // GL_DYNAMIC_DRAW is a hint that the data will be modified frequently.
//             // Initialize with null data; we'll map it to write.
//             gl::BufferData(
//                 gl::ARRAY_BUFFER,      // target
//                 buffer_size_bytes,     // size in bytes
//                 ptr::null(),           // initial data (none)
//                 gl::DYNAMIC_DRAW,      // usage hint
//             );

//             // Map the buffer.
//             // Note: For frequent updates, consider mapping once and unmapping on drop,
//             // or using glBufferSubData if mapping/unmapping per frame is too slow.
//             // gl::MAP_WRITE_BIT is essential for writing.
//             // gl::MAP_INVALIDATE_BUFFER_BIT can be a performance hint if overwriting the whole buffer.
//             let map_access_flags = gl::MAP_WRITE_BIT | gl::MAP_INVALIDATE_BUFFER_BIT;
            
//             memory = gl::MapBufferRange(
//                 gl::ARRAY_BUFFER,    // target
//                 0,                   // offset
//                 buffer_size_bytes,   // length
//                 map_access_flags,    // access flags
//             ) as *mut T;

//             if memory.is_null() {
//                 // Check for GL errors if mapping fails
//                 let error = gl::GetError();
//                 panic!("Failed to map buffer. GL Error: {}", error);
//             }

//             // Initialize the mapped memory with default values
//             let s = slice::from_raw_parts_mut(memory, BUFFER_CAPACITY_ELEMENTS);
//             for x in s.iter_mut() {
//                 *x = T::default();
//             }
//         }

//         Buffer {
//             object,
//             map: memory,
//         }
//     }

//     pub fn set(&mut self, index: u32, val: T) {
//         if index >= 64 * 1024 {
//             panic!("buffer overflow!");
//         }

//         unsafe {
//             let p = self.map.offset(index as isize);
//             *p = val;
//         }
//     }
// }

// impl<T> Drop for Buffer<T> {
//     fn drop(&mut self) {
//         unsafe {
//             if !self.map.is_null() {
//                 // Bind the buffer to unmap it. This is important.
//                 // If another buffer is bound to GL_ARRAY_BUFFER, glUnmapBuffer would target that one.
//                 gl::BindBuffer(gl::ARRAY_BUFFER, self.object);
//                 let unmap_status = gl::UnmapBuffer(gl::ARRAY_BUFFER);
//                 if unmap_status == gl::FALSE {
//                     // An error occurred during unmapping. This can happen if the data store became corrupted.
//                     // Log this, but proceed to delete the buffer object itself.
//                     // Note: Production code might handle this more gracefully or log to a file.
//                     let error = gl::GetError();
//                     eprintln!("Error unmapping buffer object {}: GL Error {}", self.object, error);
//                 }
//                 self.map = ptr::null_mut(); // Mark as unmapped
//                  // Unbind after operation
//                 gl::BindBuffer(gl::ARRAY_BUFFER, 0);
//             }

//             // Delete the buffer object
//             // gl::DeleteBuffers requires a slice of buffer IDs.
//             gl::DeleteBuffers(1, &self.object as *const GLuint);
//         }
//     }
// }
