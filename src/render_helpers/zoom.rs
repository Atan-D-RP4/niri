use smithay::backend::renderer::element::utils::Relocate;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{FrameContext, Renderer, TextureFilter};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::render_helpers::renderer::AsGlesFrame;
use crate::utils::view::{OutputViewCtx, ViewportTransform};

/// Helper macro: wrap a draw/capture_framebuffer call with filter set/restore.
///
/// `$get_guard` is an expression that yields a renderer guard (e.g. `frame.renderer()` or
/// `frame.as_gles_frame().renderer()`). `$filter` is `self.filter` (or any
/// `Option<TextureFilter>`). `$body` is the draw or capture_framebuffer call expression (which
/// returns a `Result`).
///
/// The filter is always restored to `TextureFilter::Linear` after the body runs,
/// even if the body returns an error. This prevents leaking a non-default filter
/// state into subsequent draw calls on the same renderer.
macro_rules! with_filter {
    ($get_guard:expr, $filter:expr, $body:expr $(,)?) => {{
        if let Some(filter) = $filter {
            let set_result = {
                let mut guard = $get_guard;
                let upscale = guard.as_mut().upscale_filter(filter);
                let downscale = guard.as_mut().downscale_filter(filter);
                upscale.and(downscale)
            };

            if let Err(err) = set_result {
                // A renderer exposes separate upscale/downscale setters. If
                // one setter partially succeeded, make a best-effort reset.
                let _ = {
                    let mut guard = $get_guard;
                    let upscale = guard.as_mut().upscale_filter(TextureFilter::Linear);
                    let downscale = guard.as_mut().downscale_filter(TextureFilter::Linear);
                    upscale.and(downscale)
                };
                return Err(err.into());
            }

            // Capture result first so restoration always runs.
            let result = $body;

            let restore_result = {
                let mut guard = $get_guard;
                let upscale = guard.as_mut().upscale_filter(TextureFilter::Linear);
                let downscale = guard.as_mut().downscale_filter(TextureFilter::Linear);
                upscale.and(downscale)
            };

            match (result, restore_result) {
                (Err(err), _) => Err(err),
                (Ok(value), Ok(())) => Ok(value),
                (Ok(_), Err(err)) => Err(err.into()),
            }
        } else {
            $body
        }
    }};
}

/// Linear below threshold, nearest-neighbour at or above.
///
/// Returns `Some(Linear)` when `1.0 < zoom_factor < threshold`, `Some(Nearest)`
/// when `zoom_factor >= threshold`, and `None` when `zoom_factor <= 1.0`.
///
/// Callers must ensure `zoom_factor > 1.0` before calling (the `None` return
/// for `zoom_factor <= 1.0` exists for API consistency with `ZoomElement.filter`
/// which is `Option<TextureFilter>`).
///
/// The switch at `threshold` is abrupt — there is no blending range. During an
/// animation or gesture that crosses this boundary, the visual quality changes
/// in a single frame.
pub fn zoom_filter(zoom_factor: f64, threshold: f64) -> Option<TextureFilter> {
    debug_assert!(
        !zoom_factor.is_nan() && !threshold.is_nan(),
        "zoom_filter called with NaN"
    );
    (zoom_factor > 1.0).then_some(match zoom_factor < threshold {
        true => TextureFilter::Linear,
        false => TextureFilter::Nearest,
    })
}

/// Whether changing between two sampled zoom states changes the texture filter.
///
/// A filter change affects rendered pixels without necessarily changing an
/// element's geometry. The output damage path must therefore invalidate the
/// affected output when this returns `true`; changing the wrapper field alone
/// is not enough because the renderer may otherwise skip `draw()`.
pub fn zoom_filter_changed(
    previous: Option<TextureFilter>,
    current: Option<TextureFilter>,
) -> bool {
    previous != current
}

#[derive(Debug)]
pub struct ZoomElement<E> {
    element: E,
    viewport: ViewportTransform,
    view_ctx: OutputViewCtx,
    location: Point<f64, Physical>,
    relocate: Relocate,
    filter: Option<TextureFilter>,
}

impl<E: Element> ZoomElement<E> {
    pub fn from_element(
        element: E,
        viewport: ViewportTransform,
        view_ctx: OutputViewCtx,
        location: Point<f64, Physical>,
        relocate: Relocate,
    ) -> Self {
        Self {
            element,
            viewport,
            view_ctx,
            location,
            relocate,
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: Option<TextureFilter>) -> Self {
        self.filter = filter;
        self
    }

    /// Applies the Local viewport operation at the Physical render boundary.
    ///
    /// The inner element exposes Physical geometry, but the viewport transform
    /// deliberately operates only on Local logical geometry. Keep the two unit
    /// conversions explicit and use the output context as their authority.
    fn transform_rect(&self, rect: Rectangle<f64, Physical>) -> Rectangle<f64, Physical> {
        let local = self.view_ctx.physical_rect_to_local(rect);
        let transformed = self.viewport.apply_rect(local);
        self.view_ctx.local_rect_to_physical(transformed)
    }
}

impl<E: Element> Element for ZoomElement<E> {
    fn id(&self) -> &Id {
        self.element.id()
    }

    fn current_commit(&self) -> CommitCounter {
        // OutputDamageTracker compares the derived geometry independently of
        // the commit. Do not turn floating-point parameters into a fake
        // CommitCounter: that counter is a monotonic damage history, not a
        // value fingerprint.
        self.element.current_commit()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.element.src()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        debug_assert_eq!(self.view_ctx.scale, scale);
        let mut geometry = self.transform_rect(self.element.geometry(scale).to_f64());

        match self.relocate {
            Relocate::Absolute => geometry.loc = self.location,
            Relocate::Relative => geometry.loc += self.location,
        }

        // NOTE: to_i32_up() would avoid 1-pixel jitter here but breaks
        // the screenshot selection region by oversizing geometry.
        let loc = geometry.loc.to_i32_round();
        let bottom_right = (geometry.loc + geometry.size).to_i32_round();
        Rectangle::new(loc, (bottom_right - loc).to_size())
    }

    fn transform(&self) -> Transform {
        self.element.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        debug_assert_eq!(self.view_ctx.scale, scale);
        // Damage is relative to the element geometry. OutputDamageTracker adds
        // the current geometry location after this method returns, so neither
        // the viewport origin nor the relocation belongs here.
        let inner_geometry = self.element.geometry(scale).to_f64();

        self.element
            .damage_since(scale, commit)
            .into_iter()
            .map(|rect| {
                let rect = rect.to_f64();
                let absolute = Rectangle::new(inner_geometry.loc + rect.loc, rect.size);
                let mut transformed = self.transform_rect(absolute);
                transformed.loc -= self.transform_rect(inner_geometry).loc;
                transformed.to_i32_up()
            })
            .collect()
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        debug_assert_eq!(self.view_ctx.scale, scale);
        let inner_geometry = self.element.geometry(scale).to_f64();
        self.element
            .opaque_regions(scale)
            .into_iter()
            .map(|rect| {
                let rect = rect.to_f64();
                let absolute = Rectangle::new(inner_geometry.loc + rect.loc, rect.size);
                let mut transformed = self.transform_rect(absolute);
                transformed.loc -= self.transform_rect(inner_geometry).loc;
                transformed.to_i32_up()
            })
            .collect()
    }

    fn alpha(&self) -> f32 {
        self.element.alpha()
    }

    fn kind(&self) -> Kind {
        self.element.kind()
    }

    fn is_framebuffer_effect(&self) -> bool {
        self.element.is_framebuffer_effect()
    }
}

impl<E: RenderElement<GlesRenderer>> RenderElement<GlesRenderer> for ZoomElement<E> {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        with_filter!(
            frame.renderer(),
            self.filter,
            self.element
                .draw(frame, src, dst, damage, opaque_regions, cache),
        )
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        self.element.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), GlesError> {
        with_filter!(
            frame.renderer(),
            self.filter,
            self.element.capture_framebuffer(frame, src, dst, cache),
        )
    }
}

impl<'render, E: RenderElement<TtyRenderer<'render>>> RenderElement<TtyRenderer<'render>>
    for ZoomElement<E>
{
    fn draw(
        &self,
        frame: &mut TtyFrame<'render, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), TtyRendererError<'render>> {
        with_filter!(
            frame.as_gles_frame().renderer(),
            self.filter,
            self.element
                .draw(frame, src, dst, damage, opaque_regions, cache),
        )
    }

    fn underlying_storage(
        &self,
        renderer: &mut TtyRenderer<'render>,
    ) -> Option<UnderlyingStorage<'_>> {
        self.element.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut TtyFrame<'render, '_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), TtyRendererError<'render>> {
        with_filter!(
            frame.as_gles_frame().renderer(),
            self.filter,
            self.element.capture_framebuffer(frame, src, dst, cache),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_filter_below_threshold() {
        // zoom_factor between 1.0 (exclusive) and threshold (exclusive) → Linear
        assert_eq!(zoom_filter(1.5, 2.0), Some(TextureFilter::Linear));
        assert_eq!(zoom_filter(1.001, 2.0), Some(TextureFilter::Linear));
        assert_eq!(zoom_filter(1.999, 2.0), Some(TextureFilter::Linear));
    }

    #[test]
    fn zoom_filter_at_or_above_threshold() {
        // zoom_factor >= threshold → Nearest
        assert_eq!(zoom_filter(2.0, 2.0), Some(TextureFilter::Nearest));
        assert_eq!(zoom_filter(3.0, 2.0), Some(TextureFilter::Nearest));
        assert_eq!(zoom_filter(100.0, 2.0), Some(TextureFilter::Nearest));
    }

    #[test]
    fn zoom_filter_at_or_below_one() {
        // zoom_factor <= 1.0 → None
        assert_eq!(zoom_filter(1.0, 2.0), None);
        assert_eq!(zoom_filter(0.5, 2.0), None);
        assert_eq!(zoom_filter(0.0, 2.0), None);
    }

    #[test]
    fn zoom_filter_threshold_sensitivity() {
        // Very low threshold makes Nearest kick in immediately above 1x
        assert_eq!(zoom_filter(1.001, 1.001), Some(TextureFilter::Nearest));
        // High threshold keeps Linear always
        assert_eq!(zoom_filter(100.0, 1e9), Some(TextureFilter::Linear));
    }

    #[test]
    fn zoom_filter_equality_exact() {
        // At exactly threshold → Nearest (the "<" is strict on the Linear side)
        assert_eq!(zoom_filter(2.0, 2.0), Some(TextureFilter::Nearest));
        // Just below threshold → Linear
        assert_eq!(
            zoom_filter(2.0 - f64::EPSILON, 2.0),
            Some(TextureFilter::Linear)
        );
    }

    #[test]
    fn zoom_filter_changed_in_both_directions() {
        assert!(zoom_filter_changed(
            zoom_filter(1.99, 2.0),
            zoom_filter(2.0, 2.0),
        ));
        assert!(zoom_filter_changed(
            zoom_filter(2.0, 2.0),
            zoom_filter(1.99, 2.0),
        ));
    }

    #[test]
    fn zoom_filter_unchanged_within_same_band() {
        assert!(!zoom_filter_changed(
            zoom_filter(1.25, 2.0),
            zoom_filter(1.75, 2.0),
        ));
        assert!(!zoom_filter_changed(
            zoom_filter(2.0, 2.0),
            zoom_filter(20.0, 2.0),
        ));
        assert!(!zoom_filter_changed(
            zoom_filter(1.0, 2.0),
            zoom_filter(1.0, 2.0)
        ));
    }

    #[test]
    #[should_panic(expected = "NaN")]
    fn zoom_filter_nan_zoom_factor() {
        zoom_filter(f64::NAN, 2.0);
    }

    #[test]
    #[should_panic(expected = "NaN")]
    fn zoom_filter_nan_threshold() {
        zoom_filter(2.0, f64::NAN);
    }
}
