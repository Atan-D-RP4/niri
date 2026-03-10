use std::cmp::max;
use std::rc::Rc;

use anyhow::{ensure, Context as _};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::{ffi, link_program, GlesError, GlesRenderer, GlesTexture};
use smithay::backend::renderer::{ContextId, Offscreen as _, Renderer as _, Texture as _};
use smithay::gpu_span_location;
use smithay::utils::{Buffer, Size};

use crate::render_helpers::shaders::Shaders;

/// Threshold above which tiled blur rendering is used (2048×2048 pixels).
const TILING_THRESHOLD: i32 = 2048;

/// Tile size (in pixels) used for the tiled blur path.
const TILE_SIZE: i32 = 1024;

#[derive(Debug)]
pub struct Blur {
    program: BlurProgram,
    /// Context ID of the renderer that created the program and the textures.
    renderer_context_id: ContextId<GlesTexture>,
    /// Output texture followed by intermediate textures, large to small.
    ///
    /// Created lazily and stored here to avoid recreating blur textures frequently.
    textures: Vec<GlesTexture>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct BlurOptions {
    pub passes: u8,
    pub offset: f64,
}

impl From<niri_config::Blur> for BlurOptions {
    fn from(config: niri_config::Blur) -> Self {
        Self {
            passes: config.passes,
            offset: config.offset,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlurProgram(Rc<BlurProgramInner>);

#[derive(Debug)]
struct BlurProgramInner {
    down: BlurProgramInternal,
    up: BlurProgramInternal,
}

#[derive(Debug)]
struct BlurProgramInternal {
    program: ffi::types::GLuint,
    uniform_tex: ffi::types::GLint,
    uniform_half_pixel: ffi::types::GLint,
    uniform_offset: ffi::types::GLint,
    attrib_vert: ffi::types::GLint,
}

unsafe fn compile_program(gl: &ffi::Gles2, src: &str) -> Result<BlurProgramInternal, GlesError> {
    let program = unsafe { link_program(gl, include_str!("shaders/blur.vert"), src)? };

    let vert = c"vert";
    let tex = c"tex";
    let half_pixel = c"half_pixel";
    let offset = c"offset";

    Ok(BlurProgramInternal {
        program,
        uniform_tex: gl.GetUniformLocation(program, tex.as_ptr()),
        uniform_half_pixel: gl.GetUniformLocation(program, half_pixel.as_ptr()),
        uniform_offset: gl.GetUniformLocation(program, offset.as_ptr()),
        attrib_vert: gl.GetAttribLocation(program, vert.as_ptr()),
    })
}

impl BlurProgram {
    pub fn compile(renderer: &mut GlesRenderer) -> anyhow::Result<Self> {
        renderer
            .with_context(move |gl| unsafe {
                let down = compile_program(gl, include_str!("shaders/blur_down.frag"))
                    .context("error compiling blur_down shader")?;
                let up = compile_program(gl, include_str!("shaders/blur_up.frag"))
                    .context("error compiling blur_up shader")?;
                Ok(Self(Rc::new(BlurProgramInner { down, up })))
            })
            .context("error making GL context current")?
    }

    pub fn destroy(self, renderer: &mut GlesRenderer) -> Result<(), GlesError> {
        renderer.with_context(move |gl| unsafe {
            gl.DeleteProgram(self.0.down.program);
            gl.DeleteProgram(self.0.up.program);
        })
    }
}

impl Blur {
    pub fn new(renderer: &mut GlesRenderer) -> Option<Self> {
        let program = Shaders::get(renderer).blur.clone()?;
        Some(Self {
            program,
            renderer_context_id: renderer.context_id(),
            textures: Vec::new(),
        })
    }

    pub fn context_id(&self) -> ContextId<GlesTexture> {
        self.renderer_context_id.clone()
    }

    pub fn prepare_textures(
        &mut self,
        mut create_texture: impl FnMut(Fourcc, Size<i32, Buffer>) -> Result<GlesTexture, GlesError>,
        source: &GlesTexture,
        options: BlurOptions,
    ) -> anyhow::Result<()> {
        let _span = tracy_client::span!("Blur::prepare_textures");

        let passes = options.passes.clamp(1, 31) as usize;
        let size = source.size();

        if let Some(output) = self.textures.first_mut() {
            let old_size = output.size();
            if old_size != size {
                trace!(
                    "recreating textures: output size changed from {} × {} to {} × {}",
                    old_size.w,
                    old_size.h,
                    size.w,
                    size.h
                );
                self.textures.clear();
            } else if !output.is_unique_reference() {
                debug!("recreating textures: not unique",);
                // We only need to recreate the output texture here, but this case shouldn't really
                // happen anyway, and this is simpler.
                self.textures.clear();
            }
        }

        // Create any missing textures.
        let mut w = size.w;
        let mut h = size.h;
        for i in 0..=passes {
            let size = Size::new(w, h);
            w = max(1, w / 2);
            h = max(1, h / 2);

            if self.textures.len() > i {
                // This texture already exists.
                continue;
            }

            // debug!("creating texture for step {i} sized {w} × {h}");

            let texture: GlesTexture =
                create_texture(Fourcc::Abgr8888, size).context("error creating texture")?;
            self.textures.push(texture);
        }

        // Drop any no longer needed textures.
        self.textures.drain(passes + 1..);

        Ok(())
    }

    pub fn render(
        &mut self,
        renderer: &mut GlesRenderer,
        source: &GlesTexture,
        options: BlurOptions,
    ) -> anyhow::Result<GlesTexture> {
        let _span = tracy_client::span!("Blur::render");
        trace!("rendering blur");

        let size = source.size();
        if size.w > TILING_THRESHOLD || size.h > TILING_THRESHOLD {
            trace!("using tiled blur for {}x{}", size.w, size.h);
            return self.render_tiled(renderer, source, options);
        }

        ensure!(
            renderer.context_id() == self.renderer_context_id,
            "wrong renderer"
        );

        let passes = options.passes.clamp(1, 31) as usize;
        let size = source.size();

        ensure!(
            self.textures.len() == passes + 1,
            "wrong textures len: expected {}, got {}",
            passes + 1,
            self.textures.len()
        );

        let output = &mut self.textures[0];
        ensure!(
            output.size() == size,
            "wrong output texture size: expected {size:?}, got {:?}",
            output.size()
        );

        ensure!(
            output.is_unique_reference(),
            "output texture has a non-unique reference"
        );

        let program = &*self.program.0;
        renderer.with_profiled_context(gpu_span_location!("Blur::render"), |gl| unsafe {
            while gl.GetError() != ffi::NO_ERROR {}

            gl.Disable(ffi::BLEND);
            gl.Disable(ffi::SCISSOR_TEST);

            gl.ActiveTexture(ffi::TEXTURE0);

            let mut fbos = [0; 2];
            gl.GenFramebuffers(fbos.len() as _, fbos.as_mut_ptr());

            Blur::render_inner(
                gl,
                fbos[0],
                program,
                source.tex_id(),
                &self.textures,
                options,
            );

            gl.BindFramebuffer(ffi::DRAW_FRAMEBUFFER, 0);
            gl.DeleteFramebuffers(fbos.len() as _, fbos.as_ptr());
        })?;

        Ok(self.textures[0].clone())
    }

    fn render_tiled(
        &mut self,
        renderer: &mut GlesRenderer,
        source: &GlesTexture,
        options: BlurOptions,
    ) -> anyhow::Result<GlesTexture> {
        let _span = tracy_client::span!("Blur::render_tiled");

        let passes = options.passes.clamp(1, 31) as usize;
        let src_size = source.size();

        // Dual-pass pyramid reaches ~2^passes source pixels per offset unit.
        let overlap = ((1i32 << passes) as f64 * options.offset).ceil() as i32;
        let overlap = overlap.clamp(8, 256);

        let output = self.textures[0].clone();

        struct TileInfo {
            pad_x0: i32,
            pad_y0: i32,
            pad_x1: i32,
            pad_y1: i32,
            tile_x: i32,
            tile_y: i32,
            textures: Vec<GlesTexture>,
        }

        let mut tiles: Vec<TileInfo> = Vec::new();
        let mut tile_y = 0i32;
        while tile_y < src_size.h {
            let mut tile_x = 0i32;
            while tile_x < src_size.w {
                let pad_x0 = (tile_x - overlap).max(0);
                let pad_y0 = (tile_y - overlap).max(0);
                let pad_x1 = (tile_x + TILE_SIZE + overlap).min(src_size.w);
                let pad_y1 = (tile_y + TILE_SIZE + overlap).min(src_size.h);
                let pad_w = pad_x1 - pad_x0;
                let pad_h = pad_y1 - pad_y0;

                let mut textures: Vec<GlesTexture> = Vec::new();
                let mut tw = pad_w;
                let mut th = pad_h;
                for _ in 0..=passes {
                    let sz = Size::<i32, Buffer>::new(tw, th);
                    tw = max(1, tw / 2);
                    th = max(1, th / 2);
                    let t = renderer
                        .create_buffer(Fourcc::Abgr8888, sz)
                        .context("error creating tile texture")?;
                    textures.push(t);
                }

                tiles.push(TileInfo {
                    pad_x0,
                    pad_y0,
                    pad_x1,
                    pad_y1,
                    tile_x,
                    tile_y,
                    textures,
                });
                tile_x += TILE_SIZE;
            }
            tile_y += TILE_SIZE;
        }

        let program = &*self.program.0;
        renderer.with_profiled_context(gpu_span_location!("Blur::render_tiled"), |gl| unsafe {
            while gl.GetError() != ffi::NO_ERROR {}

            let mut current_fbo = 0i32;
            let mut viewport = [0i32; 4];
            gl.GetIntegerv(ffi::FRAMEBUFFER_BINDING, &mut current_fbo as *mut _);
            gl.GetIntegerv(ffi::VIEWPORT, viewport.as_mut_ptr());

            gl.Disable(ffi::BLEND);
            gl.Disable(ffi::SCISSOR_TEST);
            gl.ActiveTexture(ffi::TEXTURE0);

            let mut fbos = [0u32; 2];
            gl.GenFramebuffers(fbos.len() as _, fbos.as_mut_ptr());

            for tile in &tiles {
                let TileInfo {
                    pad_x0,
                    pad_y0,
                    pad_x1,
                    pad_y1,
                    tile_x,
                    tile_y,
                    textures,
                } = tile;
                let pad_w = pad_x1 - pad_x0;
                let pad_h = pad_y1 - pad_y0;

                gl.BindFramebuffer(ffi::READ_FRAMEBUFFER, fbos[1]);
                gl.FramebufferTexture2D(
                    ffi::READ_FRAMEBUFFER,
                    ffi::COLOR_ATTACHMENT0,
                    ffi::TEXTURE_2D,
                    source.tex_id(),
                    0,
                );
                gl.BindFramebuffer(ffi::DRAW_FRAMEBUFFER, fbos[0]);
                gl.FramebufferTexture2D(
                    ffi::DRAW_FRAMEBUFFER,
                    ffi::COLOR_ATTACHMENT0,
                    ffi::TEXTURE_2D,
                    textures[0].tex_id(),
                    0,
                );
                gl.BlitFramebuffer(
                    *pad_x0,
                    *pad_y0,
                    *pad_x1,
                    *pad_y1,
                    0,
                    0,
                    pad_w,
                    pad_h,
                    ffi::COLOR_BUFFER_BIT,
                    ffi::NEAREST,
                );

                Blur::render_inner(
                    gl,
                    fbos[0],
                    program,
                    textures[0].tex_id(),
                    textures,
                    options,
                );

                // Crop: blit inner (non-overlap) region to output.
                let inner_x0 = tile_x - pad_x0;
                let inner_y0 = tile_y - pad_y0;
                let inner_x1 = (inner_x0 + TILE_SIZE).min(pad_w);
                let inner_y1 = (inner_y0 + TILE_SIZE).min(pad_h);
                let dst_x1 = (tile_x + TILE_SIZE).min(src_size.w);
                let dst_y1 = (tile_y + TILE_SIZE).min(src_size.h);

                gl.BindFramebuffer(ffi::READ_FRAMEBUFFER, fbos[1]);
                gl.FramebufferTexture2D(
                    ffi::READ_FRAMEBUFFER,
                    ffi::COLOR_ATTACHMENT0,
                    ffi::TEXTURE_2D,
                    textures[0].tex_id(),
                    0,
                );
                gl.BindFramebuffer(ffi::DRAW_FRAMEBUFFER, fbos[0]);
                gl.FramebufferTexture2D(
                    ffi::DRAW_FRAMEBUFFER,
                    ffi::COLOR_ATTACHMENT0,
                    ffi::TEXTURE_2D,
                    output.tex_id(),
                    0,
                );
                gl.BlitFramebuffer(
                    inner_x0,
                    inner_y0,
                    inner_x1,
                    inner_y1,
                    *tile_x,
                    *tile_y,
                    dst_x1,
                    dst_y1,
                    ffi::COLOR_BUFFER_BIT,
                    ffi::NEAREST,
                );
            }

            gl.BindFramebuffer(ffi::FRAMEBUFFER, 0);
            gl.DeleteFramebuffers(fbos.len() as _, fbos.as_ptr());

            gl.Enable(ffi::BLEND);
            gl.Enable(ffi::SCISSOR_TEST);
            gl.BindFramebuffer(ffi::FRAMEBUFFER, current_fbo as u32);
            gl.Viewport(viewport[0], viewport[1], viewport[2], viewport[3]);
        })?;

        Ok(output)
    }

    /// Runs the dual-pass (down then up) blur on `pyramid` using `gl`.
    ///
    /// `source_id` is the texture sampled for the first downsample step; subsequent steps read from
    /// `pyramid[i]` and write to `pyramid[i+1]`. The final upsample writes back to `pyramid[0]`.
    /// `pyramid` must have exactly `passes + 1` entries.
    unsafe fn render_inner(
        gl: &ffi::Gles2,
        draw_fbo: ffi::types::GLuint,
        program: &BlurProgramInner,
        source_id: ffi::types::GLuint,
        pyramid: &[GlesTexture],
        options: BlurOptions,
    ) {
        let passes = pyramid.len() - 1;
        let vertices: [f32; 12] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0];

        // Down
        gl.BindFramebuffer(ffi::DRAW_FRAMEBUFFER, draw_fbo);
        let down = &program.down;
        gl.UseProgram(down.program);
        gl.Uniform1i(down.uniform_tex, 0);
        gl.Uniform1f(down.uniform_offset, options.offset as f32);
        gl.EnableVertexAttribArray(down.attrib_vert as u32);
        gl.BindBuffer(ffi::ARRAY_BUFFER, 0);
        gl.VertexAttribPointer(
            down.attrib_vert as u32,
            2,
            ffi::FLOAT,
            ffi::FALSE,
            0,
            vertices.as_ptr().cast(),
        );
        for i in 0..passes {
            let src = if i == 0 {
                source_id
            } else {
                pyramid[i].tex_id()
            };
            let dst = &pyramid[i + 1];
            let dst_size = dst.size();
            let w = dst_size.w;
            let h = dst_size.h;
            gl.Viewport(0, 0, w, h);
            // During downsampling, half_pixel is half of the destination pixel.
            gl.Uniform2f(down.uniform_half_pixel, 0.5 / w as f32, 0.5 / h as f32);
            let dst = dst.tex_id();
            trace!("drawing down {src} to {dst}");
            gl.FramebufferTexture2D(
                ffi::DRAW_FRAMEBUFFER,
                ffi::COLOR_ATTACHMENT0,
                ffi::TEXTURE_2D,
                dst,
                0,
            );
            gl.BindTexture(ffi::TEXTURE_2D, src);
            gl.TexParameteri(ffi::TEXTURE_2D, ffi::TEXTURE_MIN_FILTER, ffi::LINEAR as i32);
            gl.TexParameteri(ffi::TEXTURE_2D, ffi::TEXTURE_MAG_FILTER, ffi::LINEAR as i32);
            gl.TexParameteri(
                ffi::TEXTURE_2D,
                ffi::TEXTURE_WRAP_S,
                ffi::CLAMP_TO_EDGE as i32,
            );
            gl.TexParameteri(
                ffi::TEXTURE_2D,
                ffi::TEXTURE_WRAP_T,
                ffi::CLAMP_TO_EDGE as i32,
            );
            gl.DrawArrays(ffi::TRIANGLES, 0, 6);
        }
        gl.DisableVertexAttribArray(down.attrib_vert as u32);

        // Up
        let up = &program.up;
        gl.UseProgram(up.program);
        gl.Uniform1i(up.uniform_tex, 0);
        gl.Uniform1f(up.uniform_offset, options.offset as f32);
        gl.EnableVertexAttribArray(up.attrib_vert as u32);
        gl.BindBuffer(ffi::ARRAY_BUFFER, 0);
        gl.VertexAttribPointer(
            up.attrib_vert as u32,
            2,
            ffi::FLOAT,
            ffi::FALSE,
            0,
            vertices.as_ptr().cast(),
        );
        for i in (0..passes).rev() {
            let src = &pyramid[i + 1];
            let dst = &pyramid[i];
            let dst_size = dst.size();
            let w = dst_size.w;
            let h = dst_size.h;
            gl.Viewport(0, 0, w, h);
            // During upsampling, half_pixel is half of the source pixel.
            let src_size = src.size();
            gl.Uniform2f(
                up.uniform_half_pixel,
                0.5 / src_size.w as f32,
                0.5 / src_size.h as f32,
            );
            let src = src.tex_id();
            let dst = dst.tex_id();
            trace!("drawing up {src} to {dst}");
            gl.FramebufferTexture2D(
                ffi::DRAW_FRAMEBUFFER,
                ffi::COLOR_ATTACHMENT0,
                ffi::TEXTURE_2D,
                dst,
                0,
            );
            gl.BindTexture(ffi::TEXTURE_2D, src);
            gl.TexParameteri(ffi::TEXTURE_2D, ffi::TEXTURE_MIN_FILTER, ffi::LINEAR as i32);
            gl.TexParameteri(ffi::TEXTURE_2D, ffi::TEXTURE_MAG_FILTER, ffi::LINEAR as i32);
            gl.TexParameteri(
                ffi::TEXTURE_2D,
                ffi::TEXTURE_WRAP_S,
                ffi::CLAMP_TO_EDGE as i32,
            );
            gl.TexParameteri(
                ffi::TEXTURE_2D,
                ffi::TEXTURE_WRAP_T,
                ffi::CLAMP_TO_EDGE as i32,
            );
            gl.DrawArrays(ffi::TRIANGLES, 0, 6);
        }
        gl.DisableVertexAttribArray(up.attrib_vert as u32);
    }
}
