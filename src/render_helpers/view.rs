use std::cell::RefCell;
use std::sync::Once;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Element, Id, RenderElement};
use smithay::backend::renderer::gles::{
    ffi, Capability, GlesError, GlesFrame, GlesRenderer, GlesTexture,
};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::{Frame, FrameContext, Offscreen, Texture, TextureFilter};
use smithay::gpu_span_location;
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Logical, Physical, Rectangle, Scale, Size, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::render_helpers::renderer::AsGlesFrame;
use crate::utils::view::OutputViewport;

/// Linear below threshold, Nearest at or above.
///
/// `Linear` is the renderer's resting default, so this is the filter for every frame;
/// only crossing the threshold changes what the sampler does.
pub fn view_filter(scale_factor: f64, threshold: f64) -> TextureFilter {
    debug_assert!(
        !scale_factor.is_nan() && !threshold.is_nan(),
        "view_filter called with NaN"
    );
    if scale_factor < threshold {
        TextureFilter::Linear
    } else {
        TextureFilter::Nearest
    }
}

pub struct Viewport {
    id: Id,
    commit: CommitCounter,
    last: Option<(OutputViewport, TextureFilter)>,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            commit: CommitCounter::default(),
            last: None,
        }
    }

    pub fn render(
        &mut self,
        viewport: OutputViewport,
        output_size: Size<f64, Logical>,
        scale: Scale<f64>,
        filter: TextureFilter,
    ) -> ViewportElement {
        let changed = self
            .last
            .map(|(v, f)| v != viewport || f != filter)
            .unwrap_or(true);
        if changed {
            self.commit.increment();
        }
        self.last = Some((viewport, filter));
        ViewportElement::new(
            self.id.clone(),
            self.commit,
            viewport,
            output_size,
            scale,
            filter,
        )
    }
}

#[derive(Debug)]
pub struct ViewportElement {
    id: Id,
    commit: CommitCounter,
    viewport: OutputViewport,
    output_size: Size<f64, Logical>,
    scale: Scale<f64>,
    filter: TextureFilter,
}

#[derive(Debug, Default)]
struct Inner {
    intermediate: Option<GlesTexture>,
}

impl ViewportElement {
    pub fn new(
        id: Id,
        commit: CommitCounter,
        viewport: OutputViewport,
        output_size: Size<f64, Logical>,
        scale: Scale<f64>,
        filter: TextureFilter,
    ) -> Self {
        Self {
            id,
            commit,
            viewport,
            output_size,
            scale,
            filter,
        }
    }

    /// The exact (unrounded) crop for this frame and the integer capture window around it,
    /// resolved in framebuffer space.
    fn crop_window(
        &self,
        fb_size: Size<i32, Physical>,
        transform: Transform,
    ) -> Option<(Rectangle<f64, Physical>, Rectangle<i32, Physical>)> {
        if fb_size.w == 0 || fb_size.h == 0 {
            return None;
        }
        let output_rect = Rectangle::from_size(fb_size);
        let vv = self
            .viewport
            .visible_viewport(self.output_size)
            .to_physical(self.scale);
        let vv = vv.intersection(output_rect.to_f64())?;
        let vv: Rectangle<f64, Physical> = transform.transform_rect_in(vv, &fb_size.to_f64());

        let x0 = vv.loc.x.floor().max(0.) as i32;
        let y0 = vv.loc.y.floor().max(0.) as i32;
        let x1 = ((vv.loc.x + vv.size.w).floor() + 1.).min(fb_size.w as f64) as i32;
        let y1 = ((vv.loc.y + vv.size.h).floor() + 1.).min(fb_size.h as f64) as i32;
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let window = Rectangle::new((x0, y0).into(), Size::from((x1 - x0, y1 - y0)));

        Some((vv, window))
    }
}

impl Inner {
    fn new() -> Self {
        Self { intermediate: None }
    }
}

impl Element for ViewportElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        let viewport_size = self.viewport.visible_viewport(self.output_size).size;

        let size = viewport_size.to_buffer(self.scale, self.transform());
        Rectangle::from_size(size)
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        Rectangle::from_size(self.output_size.to_physical_precise_round(self.scale))
    }

    fn is_framebuffer_effect(&self) -> bool {
        true
    }
}

impl RenderElement<GlesRenderer> for ViewportElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        let Some(cache) = cache else { return Ok(()) };
        let Some(inner) = cache.get::<RefCell<Inner>>() else {
            return Ok(());
        };
        let inner = inner.borrow();
        let Some(texture) = &inner.intermediate else {
            return Ok(());
        };

        // Clamp dst the same way as capture_framebuffer.
        let output_rect = Rectangle::from_size(frame.output_size());
        let clamped_dst = match dst.intersection(output_rect) {
            Some(clamped) => clamped,
            None => return Ok(()),
        };

        let transform = frame.transformation();
        let Some((vv_fb, window)) = self.crop_window(frame.output_size(), transform) else {
            return Ok(());
        };
        let src: Rectangle<f64, Buffer> = Rectangle::new(
            (
                vv_fb.loc.x - window.loc.x as f64,
                vv_fb.loc.y - window.loc.y as f64,
            )
                .into(),
            Size::from((vv_fb.size.w, vv_fb.size.h)),
        );
        frame.render_texture_from_to(
            texture,
            src,
            clamped_dst,
            damage,
            &[],
            transform.invert(),
            1.,
            None,
            &[],
        )
    }

    fn capture_framebuffer(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), GlesError> {
        let _span = tracy_client::span!("ViewportElement::capture_framebuffer");
        let location = gpu_span_location!("ViewportElement::capture_framebuffer");

        frame.with_gpu_span(location, |frame| {
            let output_rect = Rectangle::from_size(frame.output_size());
            let transform = frame.transformation();
            let supports_blit = frame.capabilities().contains(&Capability::Blit);
            let fb_size = frame.output_size();

            let mut guard = frame.renderer();
            let inner = cache.get_or_insert::<RefCell<Inner>, _>(|| RefCell::new(Inner::new()));
            let mut inner = inner.borrow_mut();
            let inner = &mut *inner;

            if !supports_blit {
                static WARN: Once = Once::new();
                WARN.call_once(|| {
                    warn!(
                        "viewport effect disabled: renderer does not support framebuffer blitting"
                    );
                });
                return Ok(());
            }

            // Clamp dst to the output, exactly as framebuffer_effect.rs does.
            let clamped_dst = match dst.intersection(output_rect) {
                Some(clamped) => clamped,
                None => return Ok(()),
            };

            let (_vv, window) = match self.crop_window(fb_size, transform) {
                Some(crop) => crop,
                None => return Ok(()),
            };

            // Size the texture from dst, not from the crop.
            let tex_size = transform.transform_size(clamped_dst.size);
            let tex_size = tex_size.to_logical(1).to_buffer(1, Transform::Normal);

            if inner
                .intermediate
                .as_ref()
                .is_some_and(|t| t.size() != tex_size)
            {
                inner.intermediate = None;
            }
            if inner.intermediate.is_none() {
                let renderer = guard.as_mut();
                inner.intermediate = Some(renderer.create_buffer(Fourcc::Abgr8888, tex_size)?);
            }
            let texture = inner.intermediate.as_ref().unwrap();

            drop(guard);

            let filter = match self.filter {
                TextureFilter::Linear => ffi::LINEAR,
                TextureFilter::Nearest => ffi::NEAREST,
            };

            // Blit the capture window into the texture 1:1, top-left anchored; the part of
            // the texture past the window is never sampled.
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
                    texture.tex_id(),
                    0,
                );

                gl.BlitFramebuffer(
                    window.loc.x,
                    window.loc.y,
                    window.loc.x + window.size.w,
                    window.loc.y + window.size.h,
                    0,
                    0,
                    window.size.w,
                    window.size.h,
                    ffi::COLOR_BUFFER_BIT,
                    filter,
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

            Ok(())
        })
    }
}

impl<'render> RenderElement<TtyRenderer<'render>> for ViewportElement {
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
}
