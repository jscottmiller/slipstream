//! Minimal GL renderer for the cabinet UI: one shader, one dynamic vertex
//! buffer, textured quads with per-vertex color. Coordinates are pixels
//! with the origin top-left; the vertex shader maps to NDC. Batches flush
//! on texture change, so group draws by texture where convenient.

use anyhow::{anyhow, Result};
use glow::HasContext;

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn gray(v: f32) -> Self {
        Self::rgba(v, v, v, 1.0)
    }

    /// Same hue scaled toward black — for dimming artwork.
    pub fn dimmed(self, f: f32) -> Self {
        Self::rgba(self.r * f, self.g * f, self.b * f, self.a)
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

pub struct Texture {
    pub raw: glow::Texture,
    pub w: u32,
    pub h: u32,
}

const VERTEX_FLOATS: usize = 8; // x y u v r g b a

pub struct Renderer {
    gl: glow::Context,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    u_screen: glow::UniformLocation,
    white: Texture,
    verts: Vec<f32>,
    batch_tex: Option<glow::Texture>,
    screen: (f32, f32),
}

const VS: &str = r#"#version 330 core
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
layout(location=2) in vec4 a_color;
uniform vec2 u_screen;
out vec2 v_uv;
out vec4 v_color;
void main() {
    vec2 ndc = a_pos / u_screen * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
    v_uv = a_uv;
    v_color = a_color;
}"#;

const FS: &str = r#"#version 330 core
in vec2 v_uv;
in vec4 v_color;
uniform sampler2D u_tex;
out vec4 frag;
void main() {
    frag = texture(u_tex, v_uv) * v_color;
}"#;

impl Renderer {
    pub fn new(gl: glow::Context) -> Result<Self> {
        unsafe {
            let program = gl.create_program().map_err(|e| anyhow!(e))?;
            for (kind, src) in [(glow::VERTEX_SHADER, VS), (glow::FRAGMENT_SHADER, FS)] {
                let shader = gl.create_shader(kind).map_err(|e| anyhow!(e))?;
                gl.shader_source(shader, src);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    return Err(anyhow!("shader: {}", gl.get_shader_info_log(shader)));
                }
                gl.attach_shader(program, shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(anyhow!("link: {}", gl.get_program_info_log(program)));
            }
            let u_screen = gl
                .get_uniform_location(program, "u_screen")
                .ok_or_else(|| anyhow!("missing u_screen uniform"))?;

            let vao = gl.create_vertex_array().map_err(|e| anyhow!(e))?;
            let vbo = gl.create_buffer().map_err(|e| anyhow!(e))?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = (VERTEX_FLOATS * 4) as i32;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 16);

            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            let white = create_texture(&gl, 1, 1, &[255, 255, 255, 255])?;
            Ok(Self {
                gl,
                program,
                vao,
                vbo,
                u_screen,
                white,
                verts: Vec::with_capacity(4096),
                batch_tex: None,
                screen: (1.0, 1.0),
            })
        }
    }

    pub fn create_texture(&self, w: u32, h: u32, rgba: &[u8]) -> Result<Texture> {
        create_texture(&self.gl, w, h, rgba)
    }

    pub fn update_texture(&self, tex: &Texture, x: u32, y: u32, w: u32, h: u32, rgba: &[u8]) {
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex.raw));
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                x as i32,
                y as i32,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba)),
            );
        }
    }

    pub fn begin(&mut self, w: u32, h: u32, clear: Color) {
        self.screen = (w as f32, h as f32);
        self.verts.clear();
        self.batch_tex = None;
        unsafe {
            self.gl.viewport(0, 0, w as i32, h as i32);
            self.gl.clear_color(clear.r, clear.g, clear.b, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn rect(&mut self, r: Rect, c: Color) {
        let raw = self.white.raw;
        self.push_quad(raw, r, [0.0, 0.0, 1.0, 1.0], [c, c, c, c]);
    }

    /// Vertical gradient: `top` color along the top edge, `bottom` below.
    pub fn rect_vgradient(&mut self, r: Rect, top: Color, bottom: Color) {
        let raw = self.white.raw;
        self.push_quad(raw, r, [0.0, 0.0, 1.0, 1.0], [top, top, bottom, bottom]);
    }

    pub fn image(&mut self, tex: &Texture, dst: Rect, tint: Color) {
        self.push_quad(tex.raw, dst, [0.0, 0.0, 1.0, 1.0], [tint; 4]);
    }

    /// A quad with explicit normalized UVs — the glyph-atlas path.
    pub fn quad_uv(&mut self, tex: glow::Texture, dst: Rect, uv: [f32; 4], c: Color) {
        self.push_quad(tex, dst, uv, [c; 4]);
    }

    /// Corners are ordered top-left, top-right, bottom-left, bottom-right.
    fn push_quad(&mut self, tex: glow::Texture, r: Rect, uv: [f32; 4], c: [Color; 4]) {
        if self.batch_tex != Some(tex) {
            self.flush();
            self.batch_tex = Some(tex);
        }
        let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
        let (u0, v0, u1, v1) = (uv[0], uv[1], uv[2], uv[3]);
        let corners = [
            (x0, y0, u0, v0, c[0]),
            (x1, y0, u1, v0, c[1]),
            (x0, y1, u0, v1, c[2]),
            (x1, y0, u1, v0, c[1]),
            (x1, y1, u1, v1, c[3]),
            (x0, y1, u0, v1, c[2]),
        ];
        for (x, y, u, v, c) in corners {
            self.verts
                .extend_from_slice(&[x, y, u, v, c.r, c.g, c.b, c.a]);
        }
    }

    pub fn end(&mut self) {
        self.flush();
    }

    fn flush(&mut self) {
        let Some(tex) = self.batch_tex else { return };
        if self.verts.is_empty() {
            return;
        }
        unsafe {
            self.gl.use_program(Some(self.program));
            self.gl
                .uniform_2_f32(Some(&self.u_screen), self.screen.0, self.screen.1);
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            let bytes: &[u8] =
                std::slice::from_raw_parts(self.verts.as_ptr() as *const u8, self.verts.len() * 4);
            self.gl
                .buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.draw_arrays(
                glow::TRIANGLES,
                0,
                (self.verts.len() / VERTEX_FLOATS) as i32,
            );
        }
        self.verts.clear();
    }

    /// Read back the current framebuffer (for the dev screenshot dump).
    pub fn read_pixels(&self, w: u32, h: u32) -> Vec<u8> {
        let mut buf = vec![0u8; (w * h * 4) as usize];
        unsafe {
            self.gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut buf)),
            );
        }
        buf
    }
}

fn create_texture(gl: &glow::Context, w: u32, h: u32, rgba: &[u8]) -> Result<Texture> {
    unsafe {
        let raw = gl.create_texture().map_err(|e| anyhow!(e))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(raw));
        for (k, v) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, k, v);
        }
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            w as i32,
            h as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(rgba)),
        );
        Ok(Texture { raw, w, h })
    }
}
