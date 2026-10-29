use std::cell::RefCell;
use std::sync::Once;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{
    ffi, Capability, GlesError, GlesFrame, GlesRenderer, GlesTexture,
};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{Frame, FrameContext, Offscreen, Texture, TextureFilter};
use smithay::gpu_span_location;
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{
    Buffer, Coordinate, Logical, Physical, Point, Rectangle, Scale, Size, Transform,
};

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
    last: Option<OutputViewport>,
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
    ) -> ViewportElement {
        let changed = self.last.map(|v| v != viewport).unwrap_or(true);
        if changed {
            self.commit.increment();
        }
        self.last = Some(viewport);
        ViewportElement::new(self.id.clone(), self.commit, viewport, output_size, scale)
    }
}

#[derive(Debug)]
pub struct ViewportElement {
    id: Id,
    commit: CommitCounter,
    viewport: OutputViewport,
    output_size: Size<f64, Logical>,
    scale: Scale<f64>,
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
    ) -> Self {
        Self {
            id,
            commit,
            viewport,
            output_size,
            scale,
        }
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

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        Rectangle::from_size(self.output_size.to_physical_precise_round(scale))
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        _commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        DamageSet::from_slice(&[self.geometry(scale)])
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

        // The texture is already the full output size (after transform), so this is
        // a 1:1 blit — exactly MagnifierElement's draw.
        frame.render_texture_from_to(
            texture,
            Rectangle::from_size(texture.size().to_f64()),
            clamped_dst,
            damage,
            &[],
            frame.transformation().invert(),
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

            let mut guard = frame.renderer();
            let inner = cache.get_or_insert::<RefCell<Inner>, _>(|| RefCell::new(Inner::new()));
            let mut inner = inner.borrow_mut();
            let inner = &mut *inner;

            // Same reset pattern as framebuffer_effect.rs.
            inner.intermediate = None;

            if !supports_blit {
                static WARN: Once = Once::new();
                WARN.call_once(|| {
                    warn!(
                        "viewport effect disabled: renderer does not support framebuffer blitting"
                    );
                });
                return Ok(());
            }

            // 1. Clamp dst to the output, exactly as framebuffer_effect.rs does.
            let clamped_dst = match dst.intersection(output_rect) {
                Some(clamped) => clamped,
                None => return Ok(()),
            };

            // 2. Compute the visible viewport in output space, then clamp to output.
            let vv = self
                .viewport
                .visible_viewport(self.output_size)
                .to_physical_precise_round(self.scale);
            let vv = match vv.intersection(output_rect) {
                Some(vv) => vv,
                None => return Ok(()),
            };

            // 3. Transform the capture rect into framebuffer space. This is the viewport analogue
            //    of MagnifierElement's capture_rect.
            let capture_rect = transform.transform_rect_in(vv, &output_rect.size);

            // 4. Size the texture from dst, not from the crop. This matches MagnifierElement's
            //    tex_size = transform.transform_size(dst.size).
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

            // 5. Blit capture_rect -> full texture. Same GL sequence as framebuffer_effect.rs and
            //    MagnifierElement.
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
//
// impl<E: Element> ViewElement<E> {
//     pub fn from_element(
//         element: E,
//         viewport: OutputViewport,
//         scale: Scale<f64>,
//         pointer_loc: Option<Point<i32, Physical>>,
//     ) -> Self {
//         Self {
//             element,
//             viewport,
//             scale,
//             pointer_loc,
//         }
//     }
//
//     /// Maps a rectangle from content space to screen space.
//     fn apply_to_rect<C: Coordinate>(&self, rect: Rectangle<C, Physical>) -> Rectangle<C,
// Physical> {         let rect = rect.to_f64();
//         let transformed = self
//             .viewport
//             .content_to_screen_rect(rect.to_logical(self.scale))
//             .to_physical(self.scale);
//         let transformed_loc = Point::from((
//             C::from_f64(transformed.loc.x.round()),
//             C::from_f64(transformed.loc.y.round()),
//         ));
//         let transaction_size = Size::from((
//             C::from_f64(transformed.size.w.round()),
//             C::from_f64(transformed.size.h.round()),
//         ));
//         Rectangle::new(transformed_loc, transaction_size)
//     }
//
//     /// Maps a region relative to the wrapped element into a region relative to this
//     /// element: made absolute in content space, mapped through the view, then made
//     /// relative to the element's origin again.
//     fn transform_relative_rect<C: Coordinate>(
//         &self,
//         inner_geometry: Rectangle<C, Physical>,
//         rect: Rectangle<C, Physical>,
//     ) -> Rectangle<C, Physical> {
//         let mut absolute = rect;
//         absolute.loc += inner_geometry.loc;
//         let mut transformed = self.apply_to_rect(absolute);
//         transformed.loc -= self.apply_to_rect(inner_geometry).loc;
//         transformed
//     }
// }
