use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use glam::{Mat3, Vec2};
use niri_config::CornerRadius;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement};
use smithay::backend::renderer::gles::{
    ffi, GlesError, GlesFrame, GlesRenderer, GlesTexture, Uniform, UniformName, UniformType,
};
use smithay::backend::renderer::utils::{CommitCounter, OpaqueRegions};
use smithay::backend::renderer::{Color32F, Frame as _, FrameContext, Offscreen, Texture as _};
use smithay::gpu_span_location;
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Logical, Physical, Rectangle, Scale, Size, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::render_helpers::background_effect::RenderParams;
use crate::render_helpers::blur::{Blur, BlurOptions};
use crate::render_helpers::effect_buffer::EffectBuffer;
use crate::render_helpers::renderer::AsGlesFrame as _;
use crate::render_helpers::shader_element::{ShaderProgram, ShaderRenderElement};
use crate::render_helpers::shaders::{mat3_uniform, ProgramType};
use crate::render_helpers::xray::{Xray, XrayPos};
use crate::render_helpers::RenderCtx;
use crate::utils::region::TransformedRegion;

#[derive(Debug, Clone)]
pub struct GlassProgram(ShaderProgram);

#[derive(Debug)]
pub struct GlassEffect {
    id: Id,
    commit: CommitCounter,
}

#[derive(Debug)]
pub struct GlassEffectElement {
    id: Id,
    commit: CommitCounter,
    geometry: Rectangle<f64, Logical>,
    clip_geo: Rectangle<f64, Logical>,
    corner_radius: CornerRadius,
    subregion: Option<TransformedRegion>,
    scale: f32,
    blur_options: Option<BlurOptions>,
    noise: f32,
    saturation: f32,
    pointer: Option<(f32, f32)>,
    time: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct GlassEffectParams {
    pub noise: f32,
    pub saturation: f32,
    pub pointer: Option<(f32, f32)>,
    pub time: f32,
}

#[derive(Debug)]
pub struct GlassXrayElement {
    buffer: Rc<RefCell<EffectBuffer>>,
    id: Id,
    geometry: Rectangle<f64, Logical>,
    src: Rectangle<f64, Buffer>,
    subregion: Option<TransformedRegion>,
    input_to_clip_geo: Mat3,
    clip_geo_size: Vec2,
    corner_radius: CornerRadius,
    scale: f32,
    blur: bool,
    noise: f32,
    saturation: f32,
    pointer: Option<(f32, f32)>,
    time: f32,
    bg_color: Color32F,
}

#[derive(Debug)]
struct Inner {
    framebuffer: Option<GlesTexture>,
    blur: Option<Blur>,
    blurred: Option<GlesTexture>,
    subregion_damage: Vec<Rectangle<i32, Physical>>,
}

impl GlassProgram {
    pub fn compile(renderer: &mut GlesRenderer) -> Result<Self, GlesError> {
        compile_glass_program(renderer).map(Self)
    }

    pub fn program(&self) -> ShaderProgram {
        self.0.clone()
    }
}

impl GlassEffect {
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            commit: CommitCounter::default(),
        }
    }

    pub fn damage(&mut self) {
        self.commit.increment();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        ns: Option<usize>,
        params: RenderParams,
        blur_options: Option<BlurOptions>,
        noise: f32,
        saturation: f32,
        pointer: Option<(f32, f32)>,
        time: f32,
    ) -> GlassEffectElement {
        let (clip_geo, corner_radius) = params
            .clip
            .unwrap_or((params.geometry, CornerRadius::default()));

        let mut id = self.id.clone();
        if let Some(ns) = ns {
            id = id.namespaced(ns);
        }

        GlassEffectElement {
            id,
            commit: self.commit,
            geometry: params.geometry,
            clip_geo,
            corner_radius,
            subregion: params.subregion,
            scale: params.scale as f32,
            blur_options,
            noise,
            saturation,
            pointer,
            time,
        }
    }
}

impl GlassEffectElement {
    fn compute_input_to_clip_geo(
        &self,
        crop: Rectangle<f64, Logical>,
        transform: Transform,
    ) -> (Mat3, Vec2) {
        let offset = crop.loc - (self.clip_geo.loc - self.geometry.loc);
        let offset = Vec2::new(offset.x as f32, offset.y as f32);
        let crop_size = Vec2::new(crop.size.w as f32, crop.size.h as f32);
        let clip_size = Vec2::new(self.clip_geo.size.w as f32, self.clip_geo.size.h as f32);

        let input_to_clip_geo =
            Mat3::from_scale(crop_size / clip_size) * Mat3::from_translation(offset / crop_size);

        let transform_mat = Mat3::from_translation(Vec2::new(0.5, 0.5))
            * Mat3::from_cols_array(transform.matrix().as_ref())
            * Mat3::from_translation(Vec2::new(-0.5, -0.5));

        let clip_geo_size = Vec2::new(self.clip_geo.size.w as f32, self.clip_geo.size.h as f32);
        (input_to_clip_geo * transform_mat, clip_geo_size)
    }

    fn compute_uniforms(
        &self,
        crop: Rectangle<f64, Logical>,
        transform: Transform,
    ) -> Rc<[Uniform<'static>]> {
        let (input_to_clip_geo, clip_geo_size) = self.compute_input_to_clip_geo(crop, transform);
        glass_uniforms(
            clip_geo_size,
            self.corner_radius,
            input_to_clip_geo,
            self.noise,
            self.saturation,
            Color32F::TRANSPARENT,
            self.pointer,
            self.time,
        )
        .into()
    }
}

impl Element for GlassEffectElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        let size = self.geometry.size.to_buffer(1., Transform::Normal);
        Rectangle::from_size(size)
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.geometry.to_physical_precise_round(scale)
    }

    fn is_framebuffer_effect(&self) -> bool {
        true
    }
}

impl RenderElement<GlesRenderer> for GlassEffectElement {
    fn capture_framebuffer(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), GlesError> {
        let _span = tracy_client::span!("GlassEffectElement::capture_framebuffer");
        let location = gpu_span_location!("GlassEffectElement::capture_framebuffer");
        frame.with_gpu_span(location, |frame| {
            let output_rect = Rectangle::from_size(frame.output_size());
            let transform = frame.transformation();

            let mut guard = frame.renderer();

            let inner = cache
                .get_or_insert::<RefCell<Inner>, _>(|| RefCell::new(Inner::new(guard.as_mut())));
            let mut inner = inner.borrow_mut();
            let inner = &mut *inner;

            inner.blurred = None;

            let clamped_dst = match dst.intersection(output_rect) {
                Some(clamped) => clamped,
                None => return Ok(()),
            };
            let clamp_scale = clamped_dst.size.to_f64() / dst.size.to_f64();

            let dst = transform.transform_rect_in(clamped_dst, &output_rect.size);

            let size = src
                .size
                .to_logical(1., Transform::Normal)
                .upscale(clamp_scale)
                .to_physical_precise_round(self.scale);
            let size = transform.transform_size(size);
            let size = size.to_logical(1).to_buffer(1, Transform::Normal);

            if inner
                .framebuffer
                .as_ref()
                .is_some_and(|fb| fb.size() != size)
            {
                inner.framebuffer = None;
            }
            let framebuffer = if let Some(fb) = &inner.framebuffer {
                fb
            } else {
                trace!("creating framebuffer texture sized {} × {}", size.w, size.h);
                let renderer = guard.as_mut();
                let texture = renderer.create_buffer(Fourcc::Abgr8888, size)?;
                inner.framebuffer.insert(texture)
            };

            let mut blur = Option::zip(inner.blur.as_mut(), self.blur_options);
            if let Some((b, options)) = &mut blur {
                let renderer = guard.as_mut();
                if let Err(err) = b.prepare_textures(
                    |fourcc, size| renderer.create_buffer(fourcc, size),
                    framebuffer,
                    *options,
                ) {
                    warn!("error preparing blur textures: {err:?}");
                    blur = None;
                }
            }

            drop(guard);

            frame.with_context(|gl| unsafe {
                while gl.GetError() != ffi::NO_ERROR {}

                let mut current_fbo = 0i32;
                gl.GetIntegerv(ffi::DRAW_FRAMEBUFFER_BINDING, &mut current_fbo as *mut _);

                gl.Disable(ffi::SCISSOR_TEST);

                let mut fbo = 0;
                gl.GenFramebuffers(1, &mut fbo as *mut _);
                gl.BindFramebuffer(ffi::DRAW_FRAMEBUFFER, fbo);

                gl.FramebufferTexture2D(
                    ffi::DRAW_FRAMEBUFFER,
                    ffi::COLOR_ATTACHMENT0,
                    ffi::TEXTURE_2D,
                    framebuffer.tex_id(),
                    0,
                );

                gl.BlitFramebuffer(
                    dst.loc.x,
                    dst.loc.y,
                    dst.loc.x + dst.size.w,
                    dst.loc.y + dst.size.h,
                    0,
                    0,
                    size.w,
                    size.h,
                    ffi::COLOR_BUFFER_BIT,
                    ffi::LINEAR,
                );

                gl.BindFramebuffer(ffi::DRAW_FRAMEBUFFER, current_fbo as u32);
                gl.Enable(ffi::SCISSOR_TEST);
                gl.DeleteFramebuffers(1, &mut fbo as *mut _);

                if gl.GetError() != ffi::NO_ERROR {
                    Err(GlesError::BlitError)
                } else {
                    Ok(())
                }
            })??;

            if self.blur_options.is_none() {
                inner.blurred = Some(framebuffer.clone());
                return Ok(());
            }

            inner.blurred = Some(framebuffer.clone());
            if let Some((blur, options)) = blur {
                let mut guard = frame.renderer();
                let renderer = guard.as_mut();
                match blur.render(renderer, framebuffer, options) {
                    Ok(blurred) => inner.blurred = Some(blurred),
                    Err(err) => warn!("error rendering blur: {err:?}"),
                }
            }

            Ok(())
        })
    }

    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        let Some(cache) = cache else {
            return Ok(());
        };
        let Some(inner) = cache.get::<RefCell<Inner>>() else {
            return Ok(());
        };
        let mut inner = inner.borrow_mut();
        let inner = &mut *inner;

        let Some(sharp) = &inner.framebuffer else {
            return Ok(());
        };
        let blurred = inner.blurred.as_ref().unwrap_or(sharp);

        let output_rect = Rectangle::from_size(frame.output_size());
        let clamped_dst = match dst.intersection(output_rect) {
            Some(clamped) => clamped,
            None => return Ok(()),
        };
        let clamp_offset = clamped_dst.loc - dst.loc;

        let filtered = &mut inner.subregion_damage;
        filtered.clear();

        if let Some(subregion) = &self.subregion {
            let mut crop = src.to_logical(1., Transform::Normal, &src.size);
            crop.loc += self.geometry.loc;
            subregion.filter_damage(crop, dst, damage, filtered);
        } else {
            filtered.extend(damage.iter());
        };

        if clamped_dst != dst {
            let r = Rectangle::new(clamp_offset, clamped_dst.size);
            filtered.retain_mut(|d| {
                if let Some(mut crop) = d.intersection(r) {
                    crop.loc -= clamp_offset;
                    *d = crop;
                    true
                } else {
                    false
                }
            });
        }

        if filtered.is_empty() {
            return Ok(());
        }
        let damage = &filtered[..];

        let src_loc = src.loc.to_logical(1., Transform::Normal, &src.size);
        let dst_to_src = src.size / dst.size.to_f64();
        let crop = Rectangle::new(
            src_loc + clamp_offset.to_f64().upscale(dst_to_src).to_logical(1.),
            clamped_dst.size.to_f64().upscale(dst_to_src).to_logical(1.),
        );

        let transform = frame.transformation();
        let uniforms = self.compute_uniforms(crop, transform);
        draw_glass_shader(
            frame,
            self.geometry.size,
            self.scale,
            Rectangle::from_size(sharp.size().to_f64()),
            clamped_dst,
            damage,
            sharp,
            blurred,
            uniforms,
        )
    }
}

impl Element for GlassXrayElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.buffer.borrow().commit()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.src
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.geometry.to_physical_precise_round(scale)
    }

    fn opaque_regions(&self, _scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        OpaqueRegions::default()
    }
}

impl GlassXrayElement {
    fn compute_uniforms(&self) -> Rc<[Uniform<'static>]> {
        glass_uniforms(
            self.clip_geo_size,
            self.corner_radius,
            self.input_to_clip_geo,
            self.noise,
            self.saturation,
            self.bg_color,
            self.pointer,
            self.time,
        )
        .into()
    }
}

impl RenderElement<GlesRenderer> for GlassXrayElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
        _cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        let mut buffer = self.buffer.borrow_mut();
        let (sharp, blurred) = match buffer.render_pair(frame, self.blur) {
            Ok(x) => x,
            Err(err) => {
                warn!("error rendering effect buffer: {err:?}");
                return Ok(());
            }
        };

        let mut filtered_damage = Vec::new();
        let damage = if let Some(subregion) = &self.subregion {
            let src_to_geo = self.geometry.size / self.src.size;

            let mut crop = src;
            crop.loc -= self.src.loc;
            crop = crop.upscale(src_to_geo);
            let mut crop = crop.to_logical(1., Transform::Normal, &Size::default());
            crop.loc += self.geometry.loc;

            subregion.filter_damage(crop, dst, damage, &mut filtered_damage);

            if filtered_damage.is_empty() {
                return Ok(());
            }
            &filtered_damage[..]
        } else {
            damage
        };

        let uniforms = self.compute_uniforms();

        draw_glass_shader(
            frame,
            self.geometry.size,
            self.scale,
            src,
            dst,
            damage,
            &sharp,
            &blurred,
            uniforms,
        )
    }
}

impl<'render> RenderElement<TtyRenderer<'render>> for GlassEffectElement {
    fn capture_framebuffer(
        &self,
        frame: &mut TtyFrame<'_, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), TtyRendererError<'render>> {
        let gles_frame = frame.as_gles_frame();
        RenderElement::<GlesRenderer>::capture_framebuffer(&self, gles_frame, src, dst, cache)?;
        Ok(())
    }

    fn draw(
        &self,
        frame: &mut TtyFrame<'_, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), TtyRendererError<'render>> {
        let gles_frame = frame.as_gles_frame();
        RenderElement::<GlesRenderer>::draw(
            &self,
            gles_frame,
            src,
            dst,
            damage,
            opaque_regions,
            cache,
        )?;
        Ok(())
    }
}

impl<'render> RenderElement<TtyRenderer<'render>> for GlassXrayElement {
    fn draw(
        &self,
        frame: &mut TtyFrame<'_, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), TtyRendererError<'render>> {
        let gles_frame = frame.as_gles_frame();
        RenderElement::<GlesRenderer>::draw(
            &self,
            gles_frame,
            src,
            dst,
            damage,
            opaque_regions,
            cache,
        )?;
        Ok(())
    }
}

impl Inner {
    fn new(renderer: &mut GlesRenderer) -> Self {
        Self {
            framebuffer: None,
            blur: Blur::new(renderer),
            blurred: None,
            subregion_damage: Vec::new(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_xray(
    xray: &Xray,
    ctx: RenderCtx<GlesRenderer>,
    params: RenderParams,
    xray_pos: XrayPos,
    blur: bool,
    effect: GlassEffectParams,
    push: &mut dyn FnMut(GlassXrayElement),
) {
    let zoom = xray_pos.zoom;
    let pos_in_backdrop = xray_pos.pos_in_backdrop.upscale(zoom);

    let (clip_geo, corner_radius) = params
        .clip
        .unwrap_or((params.geometry, CornerRadius::default()));

    let clip_offset = clip_geo.loc - params.geometry.loc;
    let clip_pos_in_backdrop = pos_in_backdrop + clip_offset.upscale(zoom);
    let geo_in_backdrop = Rectangle::new(pos_in_backdrop, params.geometry.size.upscale(zoom));

    let mut backdrop = xray.backdrop[ctx.target as usize].borrow_mut();
    let backdrop_geo = Rectangle::from_size(backdrop.logical_size());
    let intersection_with_backdrop = backdrop_geo.intersection(geo_in_backdrop);

    let mut skip_backdrop = intersection_with_backdrop.is_none();

    let mut background = xray.background[ctx.target as usize].borrow_mut();
    let prev = background.commit();
    if background.prepare(ctx.renderer, blur) {
        if background.commit() != prev {
            debug!("background damaged");
        }

        let clip_geo_size = Vec2::new(clip_geo.size.w as f32, clip_geo.size.h as f32);
        let buf_size = background.logical_size();

        for (ws_geo, bg_color) in &xray.workspaces {
            let crop = if bg_color.is_opaque() && ws_geo.contains_rect(geo_in_backdrop) {
                skip_backdrop = true;
                Some(geo_in_backdrop)
            } else {
                ws_geo.intersection(geo_in_backdrop)
            };

            let Some(crop) = crop else {
                continue;
            };

            if bg_color.is_opaque()
                && intersection_with_backdrop.is_some_and(|backdrop| crop.contains_rect(backdrop))
            {
                skip_backdrop = true;
            }

            let ws_zoom = ws_geo.size / buf_size;

            let src = Rectangle::new(crop.loc - ws_geo.loc, crop.size).downscale(ws_zoom);
            let src = src.to_buffer(background.scale(), Transform::Normal, &buf_size);

            let buf_size = Vec2::new(buf_size.w as f32, buf_size.h as f32);
            let pos_against_buf = (clip_pos_in_backdrop - ws_geo.loc).downscale(ws_zoom);
            let pos_against_buf = Vec2::new(pos_against_buf.x as f32, pos_against_buf.y as f32);
            let ws_zoom_vec = Vec2::new(ws_zoom.x as f32, ws_zoom.y as f32);
            let input_to_clip_geo = Mat3::from_scale(ws_zoom_vec / zoom as f32)
                * Mat3::from_scale(buf_size / clip_geo_size)
                * Mat3::from_translation(-pos_against_buf / buf_size);

            let mut geometry =
                Rectangle::new(crop.loc - geo_in_backdrop.loc, crop.size).downscale(zoom);
            geometry.loc += params.geometry.loc;

            push(GlassXrayElement {
                buffer: xray.background[ctx.target as usize].clone(),
                id: background.id().clone(),
                geometry,
                src,
                subregion: params.subregion.clone(),
                input_to_clip_geo,
                clip_geo_size,
                corner_radius,
                scale: params.scale as f32,
                blur,
                noise: effect.noise,
                saturation: effect.saturation,
                pointer: effect.pointer,
                time: effect.time,
                bg_color: *bg_color,
            });
        }
    }

    if skip_backdrop {
        return;
    }

    let prev = backdrop.commit();
    if backdrop.prepare(ctx.renderer, blur) {
        if backdrop.commit() != prev {
            debug!("backdrop damaged");
        }

        let buf_size = backdrop.logical_size();
        let src = geo_in_backdrop.to_buffer(backdrop.scale(), Transform::Normal, &buf_size);

        let mut clip_geo_in_backdrop = Rectangle::new(clip_offset, clip_geo.size).upscale(zoom);
        clip_geo_in_backdrop.loc += geo_in_backdrop.loc;

        let clip_pos_in_backdrop = Vec2::new(
            clip_geo_in_backdrop.loc.x as f32,
            clip_geo_in_backdrop.loc.y as f32,
        );
        let clip_geo_size = Vec2::new(
            clip_geo_in_backdrop.size.w as f32,
            clip_geo_in_backdrop.size.h as f32,
        );

        let buf_size = Vec2::new(buf_size.w as f32, buf_size.h as f32);
        let input_to_clip_geo = Mat3::from_scale(buf_size / clip_geo_size)
            * Mat3::from_translation(-clip_pos_in_backdrop / buf_size);

        push(GlassXrayElement {
            buffer: xray.backdrop[ctx.target as usize].clone(),
            id: backdrop.id().clone(),
            geometry: params.geometry,
            src,
            subregion: params.subregion,
            input_to_clip_geo,
            clip_geo_size,
            corner_radius: corner_radius.scaled_by(zoom as f32),
            scale: params.scale as f32,
            blur,
            noise: effect.noise,
            saturation: effect.saturation,
            pointer: effect.pointer,
            time: effect.time,
            bg_color: xray.backdrop_color,
        });
    }
}

fn glass_uniforms(
    clip_geo_size: Vec2,
    corner_radius: CornerRadius,
    input_to_clip_geo: Mat3,
    noise: f32,
    saturation: f32,
    bg_color: Color32F,
    pointer: Option<(f32, f32)>,
    time: f32,
) -> Vec<Uniform<'static>> {
    let pointer = pointer.unwrap_or((-1.0f32, -1.0f32));
    vec![
        Uniform::new("niri_pointer", [pointer.0, pointer.1]),
        Uniform::new("niri_window_size", <[f32; 2]>::from(clip_geo_size)),
        Uniform::new("niri_time", time),
        Uniform::new("noise", noise),
        Uniform::new("saturation", saturation),
        Uniform::new("bg_color", bg_color.components()),
        Uniform::new("geo_size", <[f32; 2]>::from(clip_geo_size)),
        Uniform::new("corner_radius", <[f32; 4]>::from(corner_radius)),
        mat3_uniform("input_to_geo", input_to_clip_geo),
    ]
}

fn compile_glass_program(renderer: &mut GlesRenderer) -> Result<ShaderProgram, GlesError> {
    ShaderProgram::compile(
        renderer,
        DEFAULT_GLASS_BACKGROUND_EFFECT,
        &[
            UniformName::new("geo_size", UniformType::_2f),
            UniformName::new("corner_radius", UniformType::_4f),
            UniformName::new("input_to_geo", UniformType::Matrix3x3),
            UniformName::new("noise", UniformType::_1f),
            UniformName::new("saturation", UniformType::_1f),
            UniformName::new("bg_color", UniformType::_4f),
            UniformName::new("niri_pointer", UniformType::_2f),
            UniformName::new("niri_window_size", UniformType::_2f),
            UniformName::new("niri_time", UniformType::_1f),
        ],
        &["niri_bg_tex", "niri_blur_tex"],
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_glass_shader(
    frame: &mut GlesFrame<'_, '_>,
    size: Size<f64, Logical>,
    scale: f32,
    src: Rectangle<f64, Buffer>,
    dst: Rectangle<i32, Physical>,
    damage: &[Rectangle<i32, Physical>],
    sharp: &GlesTexture,
    blurred: &GlesTexture,
    uniforms: Rc<[Uniform<'static>]>,
) -> Result<(), GlesError> {
    let element = ShaderRenderElement::new(
        ProgramType::Glass,
        size,
        None,
        scale,
        1.,
        uniforms,
        HashMap::from([
            (String::from("niri_bg_tex"), sharp.clone()),
            (String::from("niri_blur_tex"), blurred.clone()),
        ]),
        Kind::Unspecified,
    );

    RenderElement::<GlesRenderer>::draw(&element, frame, src, dst, damage, &[], None)
}

const DEFAULT_GLASS_BACKGROUND_EFFECT: &str = r#"
precision highp float;

#if defined(DEBUG_FLAGS)
uniform float niri_tint;
#endif

varying vec2 niri_v_coords;
uniform vec2 niri_size;

uniform sampler2D niri_bg_tex;
uniform sampler2D niri_blur_tex;

uniform vec2 geo_size;
uniform vec4 corner_radius;
uniform mat3 input_to_geo;
uniform float noise;
uniform float saturation;
uniform vec4 bg_color;
uniform vec2 niri_pointer;
uniform vec2 niri_window_size;
uniform float niri_time;

float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

float gradient_noise(vec2 uv) {
    const vec3 magic = vec3(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(uv, magic.xy)));
}

float rounded_box_sdf(vec2 p, vec2 half_size, float radius) {
    vec2 q = abs(p) - half_size + vec2(radius);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

vec2 safe_uv(vec2 uv) {
    return clamp(uv, vec2(0.0015), vec2(0.9985));
}

void main() {
    vec3 coords_geo = input_to_geo * vec3(niri_v_coords, 1.0);
    vec2 local_uv = coords_geo.xy;

    if (local_uv.x < 0.0 || 1.0 < local_uv.x || local_uv.y < 0.0 || 1.0 < local_uv.y) {
        gl_FragColor = vec4(0.0);
        return;
    }

    vec2 panel_px = local_uv * geo_size;
    vec2 half_size = geo_size * 0.5;
    vec2 centered_px = panel_px - half_size;
    float radius = max(1.0, min(min(corner_radius.x, corner_radius.y), min(corner_radius.z, corner_radius.w)));
    float sdf = rounded_box_sdf(centered_px, half_size, radius);
    float interior = max(-sdf, 0.0);
    float edge = 1.0 - smoothstep(0.0, max(radius * 0.75, 24.0), interior);
    float edge_soft = 1.0 - smoothstep(0.0, max(radius * 1.4, 42.0), interior);

    vec2 centered_uv = local_uv * 2.0 - 1.0;
    vec2 aspect = geo_size / max(min(geo_size.x, geo_size.y), 1.0);
    vec2 shaped = centered_uv * aspect;
    float radial = dot(shaped, shaped);
    float bulge = clamp(1.0 - radial, 0.0, 1.0);
    float dome = pow(bulge, 0.55);

    vec2 pointer_vec = vec2(0.0);
    float pointer_glow = 0.0;
    if (niri_pointer.x >= 0.0) {
        vec2 pointer_local = niri_pointer / max(niri_window_size, vec2(1.0));
        vec2 to_pointer = local_uv - pointer_local;
        float pointer_dist = length(to_pointer);
        float pointer_influence = (1.0 - smoothstep(0.0, 0.30, pointer_dist)) * 0.010;
        pointer_vec = normalize(to_pointer + vec2(0.0001)) * pointer_influence;
        pointer_glow = (1.0 - smoothstep(0.0, 0.18, pointer_dist)) * 0.08;
    }

    vec2 refract_vec = shaped * (0.010 + dome * 0.016 + edge * 0.012) + pointer_vec;
    float shimmer = 0.5 + 0.5 * sin(niri_time * 0.35 + local_uv.x * 8.0 - local_uv.y * 5.0);
    refract_vec *= 0.94 + shimmer * 0.06;

    vec2 ca_dir = normalize(refract_vec + vec2(1e-5));
    vec2 ca = ca_dir * (0.0012 + dome * 0.0022 + edge * 0.0015);

    vec2 sharp_uv = safe_uv(niri_v_coords + refract_vec);
    vec2 blur_uv = safe_uv(niri_v_coords + refract_vec * 0.45);
    vec4 sharp_base = texture2D(niri_bg_tex, sharp_uv);
    vec4 blurred = texture2D(niri_blur_tex, blur_uv);
    float sharp_r = texture2D(niri_bg_tex, safe_uv(sharp_uv + ca)).r;
    float sharp_b = texture2D(niri_bg_tex, safe_uv(sharp_uv - ca)).b;
    vec4 refracted = vec4(sharp_r, sharp_base.g, sharp_b, sharp_base.a);

    float clarity = clamp(0.18 + dome * 0.20 + edge * 0.34, 0.0, 0.82);
    vec4 color = mix(blurred, refracted, clarity);
    color.a = max(blurred.a, refracted.a);

    float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    color.rgb = mix(vec3(luma), color.rgb, 0.90);
    color.rgb *= vec3(0.93, 0.97, 1.00);
    color.rgb += 0.025 * color.a;

    vec3 normal = normalize(vec3(-shaped * (0.75 + dome * 0.25), 1.0 + dome * 0.85));
    vec3 view_dir = vec3(0.0, 0.0, 1.0);
    vec3 light_a = normalize(vec3(-0.42, -0.85, 0.36));
    vec3 light_b = normalize(vec3(0.34, -0.68, 0.48));
    float fresnel = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.6);
    float spec_a = pow(max(dot(reflect(-light_a, normal), view_dir), 0.0), 26.0);
    float spec_b = pow(max(dot(reflect(-light_b, normal), view_dir), 0.0), 40.0) * 0.55;
    float top_sheen = exp(-pow((local_uv.y - 0.06) * 7.2, 2.0)) * (0.16 + edge * 0.10);
    float rim = fresnel * (0.10 + edge * 0.16);
    float inner_stroke = edge_soft * (0.04 + dome * 0.03);
    float lower_shadow = smoothstep(0.15, 0.95, local_uv.y) * (1.0 - edge_soft) * 0.06;

    color.rgb += (spec_a * 0.20 + spec_b * 0.14 + top_sheen + rim + inner_stroke + pointer_glow) * color.a;
    color.rgb -= lower_shadow * color.a;

    if (saturation != 1.0) {
        const vec3 w = vec3(0.2126, 0.7152, 0.0722);
        color.rgb = mix(vec3(dot(color.rgb, w)), color.rgb, saturation);
    }

    if (noise > 0.0) {
        color.rgb += (gradient_noise(gl_FragCoord.xy) - 0.5) * noise;
    }

    color = color + bg_color * (1.0 - color.a);
    color *= niri_rounding_alpha(local_uv * geo_size, geo_size, corner_radius);
    gl_FragColor = color * niri_alpha;

    #if defined(DEBUG_FLAGS)
    if (niri_tint == 1.0)
        gl_FragColor = vec4(0.0, 0.2, 0.0, 0.2) + gl_FragColor * 0.8;
    #endif
}
"#;
