use std::cell::RefCell;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Element, Id, RenderElement};
use smithay::backend::renderer::gles::{ffi, GlesError, GlesFrame, GlesRenderer, GlesTexture};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::{Frame as _, FrameContext, Offscreen, Texture as _};
use smithay::gpu_span_location;
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::render_helpers::renderer::AsGlesFrame as _;

/// zoom post-process effect.
/// Composites the output normally, then zooms in cursor-centered region
#[derive(Debug)]
pub struct Zoom {
    id: Id,
    commit: CommitCounter,
}

#[derive(Debug)]
pub struct ZoomElement {
    id: Id,
    commit: CommitCounter,
    /// Destination geometry in output-local logical coordinates (the full output).
    geometry: Rectangle<f64, Logical>,
    /// Pointer position in output-local physical coordinates.
    pointer_pos: Point<i32, Physical>,
    /// Current zoom factor (>= 1.0).
    zoom: f64,
}

#[derive(Debug, Default)]
struct Inner {
    intermediate: Option<GlesTexture>,
}

impl Zoom {
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            commit: CommitCounter::default(),
        }
    }

    /// Mark the effect as damaged to trigger a capture on the next frame.
    pub fn damage(&mut self) {
        self.commit.increment();
    }

    pub fn render(
        &self,
        geometry: Rectangle<f64, Logical>,
        pointer_pos: Point<i32, Physical>,
        zoom: f64,
    ) -> ZoomElement {
        ZoomElement {
            id: self.id.clone(),
            commit: self.commit,
            geometry,
            pointer_pos,
            zoom,
        }
    }
}

impl Default for Zoom {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for ZoomElement {
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

impl RenderElement<GlesRenderer> for ZoomElement {
    fn capture_framebuffer(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), GlesError> {
        let _span = tracy_client::span!("ZoomElement::capture_framebuffer");
        let location = gpu_span_location!("ZoomElement::capture_framebuffer");
        frame.with_gpu_span(location, |frame| {
            let output_rect = Rectangle::from_size(frame.output_size());
            let transform = frame.transformation();

            let mut guard = frame.renderer();

            let inner = cache.get_or_insert::<RefCell<Inner>, _>(|| RefCell::new(Inner::default()));
            let mut inner = inner.borrow_mut();
            let inner = &mut *inner;

            // Crop area centered on pointer position, clamped within output bounds.
            let zoom = self.zoom.max(1.);
            let src_w = ((dst.size.w as f64) / zoom).round().max(1.) as i32;
            let src_h = ((dst.size.h as f64) / zoom).round().max(1.) as i32;
            let max_x = (output_rect.size.w - src_w).max(0);
            let max_y = (output_rect.size.h - src_h).max(0);
            let src_x = (self.pointer_pos.x - src_w / 2).clamp(0, max_x);
            let src_y = (self.pointer_pos.y - src_h / 2).clamp(0, max_y);
            let capture_rect = Rectangle::new((src_x, src_y).into(), (src_w, src_h).into());

            // Account for output rotation/transform.
            let capture_rect = transform.transform_rect_in(capture_rect, &output_rect.size);
            let tex_size = transform.transform_size(dst.size);
            let buf_size = tex_size.to_logical(1).to_buffer(1, Transform::Normal);

            // Reuse intermediate texture if size matches, avoiding per-frame reallocation.
            if inner
                .intermediate
                .as_ref()
                .is_some_and(|t| t.size() != buf_size)
            {
                inner.intermediate = None;
            }

            let texture = if let Some(tex) = &inner.intermediate {
                tex
            } else {
                let renderer = guard.as_mut();
                let tex: GlesTexture = renderer.create_buffer(Fourcc::Abgr8888, buf_size)?;
                inner.intermediate.insert(tex)
            };

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
                    texture.tex_id(),
                    0,
                );

                gl.BlitFramebuffer(
                    capture_rect.loc.x,
                    capture_rect.loc.y,
                    capture_rect.loc.x + capture_rect.size.w,
                    capture_rect.loc.y + capture_rect.size.h,
                    0,
                    0,
                    tex_size.w,
                    tex_size.h,
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

            Ok(())
        })
    }

    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        let Some(cache) = cache else {
            return Ok(());
        };
        let Some(inner) = cache.get::<RefCell<Inner>>() else {
            return Ok(());
        };
        let inner = inner.borrow();

        let Some(texture) = &inner.intermediate else {
            return Ok(());
        };

        frame.render_texture_from_to(
            texture,
            Rectangle::from_size(texture.size().to_f64()),
            dst,
            damage,
            opaque_regions,
            frame.transformation().invert(),
            1.,
            None,
            &[],
        )
    }
}

impl<'render> RenderElement<TtyRenderer<'render>> for ZoomElement {
    fn capture_framebuffer(
        &self,
        frame: &mut TtyFrame<'_, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), TtyRendererError<'render>> {
        let gles_frame = frame.as_gles_frame();
        RenderElement::<GlesRenderer>::capture_framebuffer(self, gles_frame, src, dst, cache)?;
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
            self,
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
