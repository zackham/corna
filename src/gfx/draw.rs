use super::math::{Color, Rect};
use anyhow::Result;
use glow::HasContext;
use crate::app::UiMode;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

pub struct DrawContext {
    gl: glow::Context,
    program: glow::Program,
    vbo: glow::Buffer,
    vao: Option<glow::VertexArray>,
    vertices: Vec<Vertex>,
    viewport: [f32; 2],

    // Uniform locations
    u_viewport: Option<glow::UniformLocation>,
    u_color: Option<glow::UniformLocation>,
    u_time: Option<glow::UniformLocation>,
    u_effect_mode: Option<glow::UniformLocation>,
}

impl DrawContext {
    pub fn new(gl: glow::Context, program: glow::Program) -> Result<Self> {
        let vbo = unsafe {
            gl.create_buffer()
                .map_err(|e| anyhow::anyhow!("Failed to create buffer: {}", e))?
        };

        // VAOs are not universally available in GLES2
        let vao = None;

        let u_viewport = unsafe { gl.get_uniform_location(program, "uViewport") };
        let u_color = unsafe { gl.get_uniform_location(program, "uColor") };
        let u_time = unsafe { gl.get_uniform_location(program, "uTime") };
        let u_effect_mode = unsafe { gl.get_uniform_location(program, "uEffectMode") };

        Ok(Self {
            gl,
            program,
            vbo,
            vao,
            vertices: Vec::with_capacity(1024),
            viewport: [800.0, 600.0],
            u_viewport,
            u_color,
            u_time,
            u_effect_mode,
        })
    }

    pub fn begin(&mut self, viewport_px: [f32; 2]) {
        self.viewport = viewport_px;
        self.vertices.clear();

        unsafe {
            self.gl.viewport(0, 0, viewport_px[0] as i32, viewport_px[1] as i32);
            self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            // Enable alpha blending for transparency
            self.gl.enable(glow::BLEND);
            self.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            self.gl.use_program(Some(self.program));
            if let Some(loc) = self.u_viewport {
                self.gl.uniform_2_f32(Some(&loc), viewport_px[0], viewport_px[1]);
            }
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let x2 = x + w;
        let y2 = y + h;

        self.vertices.extend_from_slice(&[
            Vertex { pos: [x, y], uv: [0.0, 0.0] },
            Vertex { pos: [x2, y], uv: [1.0, 0.0] },
            Vertex { pos: [x2, y2], uv: [1.0, 1.0] },

            Vertex { pos: [x, y], uv: [0.0, 0.0] },
            Vertex { pos: [x2, y2], uv: [1.0, 1.0] },
            Vertex { pos: [x, y2], uv: [0.0, 1.0] },
        ]);

        self.set_color(color);
        self.flush_batch();
    }

    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, _radius: f32, color: Color) {
        // Simplified version - just draw a regular rect for now
        // TODO: implement proper rounded corners with triangles
        self.rect(x, y, w, h, color);
    }

    fn set_color(&mut self, color: Color) {
        unsafe {
            if let Some(loc) = self.u_color {
                self.gl.uniform_4_f32(Some(&loc), color.r, color.g, color.b, color.a);
            }
        }
    }

    pub fn set_time(&mut self, time: f32) {
        unsafe {
            if let Some(loc) = self.u_time {
                self.gl.uniform_1_f32(Some(&loc), time);
            }
        }
    }

    pub fn set_effect_mode(&mut self, mode: i32) {
        unsafe {
            if let Some(loc) = self.u_effect_mode {
                self.gl.uniform_1_i32(Some(&loc), mode);
            }
        }
    }

    pub fn set_progress(&mut self, progress: f32) {
        unsafe {
            let loc = self.gl.get_uniform_location(self.program, "uProgress");
            self.gl.uniform_1_f32(loc.as_ref(), progress);
        }
    }

    fn flush_batch(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        unsafe {
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));

            let data = bytemuck::cast_slice(&self.vertices);
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                data,
                glow::DYNAMIC_DRAW,
            );

            if let Some(vao) = self.vao {
                self.gl.bind_vertex_array(Some(vao));
            }

            let a_pos = self.gl.get_attrib_location(self.program, "aPos");
            let a_uv = self.gl.get_attrib_location(self.program, "aUV");

            if let Some(a_pos) = a_pos {
                self.gl.enable_vertex_attrib_array(a_pos);
                self.gl.vertex_attrib_pointer_f32(
                    a_pos,
                    2,
                    glow::FLOAT,
                    false,
                    std::mem::size_of::<Vertex>() as i32,
                    0,
                );
            }

            if let Some(a_uv) = a_uv {
                self.gl.enable_vertex_attrib_array(a_uv);
                self.gl.vertex_attrib_pointer_f32(
                    a_uv,
                    2,
                    glow::FLOAT,
                    false,
                    std::mem::size_of::<Vertex>() as i32,
                    8,
                );
            }

            self.gl.draw_arrays(glow::TRIANGLES, 0, self.vertices.len() as i32);

            if let Some(_vao) = self.vao {
                self.gl.bind_vertex_array(None);
            }
        }

        self.vertices.clear();
    }

    pub fn flush(&mut self) {
        self.flush_batch();
    }
}

impl Drop for DrawContext {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.vbo);
            if let Some(vao) = self.vao {
                self.gl.delete_vertex_array(vao);
            }
        }
    }
}