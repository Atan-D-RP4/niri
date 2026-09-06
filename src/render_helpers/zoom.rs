use smithay::backend::renderer::element::utils::Relocate;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{FrameContext, Renderer, TextureFilter};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::render_helpers::renderer::AsGlesFrame;
use crate::utils::geometry::Local;
use crate::utils::view::{OutputViewCtx, ViewportTransform};

/// Runs a draw/capture call with the texture filter set, restoring `Linear` after.
///
/// Restoration runs even if the body errors, so the filter never leaks into
/// later draws on the same renderer.
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
                // One setter may have succeeded; reset best-effort.
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

/// Linear below threshold, Nearest at or above, `None` at or below 1x.
///
/// The switch is abrupt: crossing the threshold changes quality in one frame.
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

/// Whether two sampled states use different texture-filter bands.
///
/// Pure comparison behind `OutputState::zoom_filter_for`, which owns the
/// per-output transition. A flip changes pixels without changing geometry,
/// so the damage path must invalidate when this returns `true`.
pub fn zoom_filter_changed(
    previous: Option<TextureFilter>,
    current: Option<TextureFilter>,
) -> bool {
    previous != current
}

/// Whether a threshold change flips the materialized filter at `factor`.
///
/// Config-reload path: geometry is unchanged, so `true` must also queue a
/// redraw, otherwise no frame runs and stale pixels stay on screen.
pub fn threshold_flips_filter(factor: f64, old_threshold: f64, new_threshold: f64) -> bool {
    zoom_filter_changed(
        zoom_filter(factor, old_threshold),
        zoom_filter(factor, new_threshold),
    )
}

#[derive(Debug)]
pub struct ZoomElement<E> {
    element: E,
    viewport: ViewportTransform,
    view_ctx: OutputViewCtx,
    location: Point<f64, Physical>,
    relocate: Relocate,
    filter: Option<TextureFilter>,
    /// Band flipped since the last materialized frame; forces full damage.
    filter_changed: bool,
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
            filter_changed: false,
        }
    }

    /// Cursor element with tip-glued placement, shared by the live pointer
    /// and the screenshot preview.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cursor(
        elem: E,
        focal: Point<f64, Physical>,
        display: Point<f64, Local>,
        hotspot: Point<i32, Physical>,
        viewport: ViewportTransform,
        graphic_scale: f64,
        view_ctx: OutputViewCtx,
        scale: Scale<f64>,
    ) -> Self {
        let (final_pos, wrapper) =
            viewport.place_cursor(focal, display, hotspot, graphic_scale, scale);
        Self::from_element(elem, wrapper, view_ctx, final_pos, Relocate::Absolute)
    }

    pub fn with_filter(mut self, filter: Option<TextureFilter>) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_filter_changed(mut self, changed: bool) -> Self {
        self.filter_changed = changed;
        self
    }

    /// Viewport math in Local, unit crossings at the Physical boundary.
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
        // The tracker compares derived geometry itself; a CommitCounter is a
        // monotonic damage history, not a value fingerprint — don't fake one.
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
        if self.filter_changed {
            // Same geometry and commit, new filter: damage the whole element
            // (element-relative, hence zeroed location).
            return DamageSet::from_slice(&[Rectangle::new(
                Point::from((0, 0)),
                self.geometry(scale).size,
            )]);
        }
        // Damage is element-relative; the tracker adds the location itself,
        // so neither the viewport origin nor the relocation belongs here.
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
    use crate::utils::geometry::RectExt;

    #[derive(Debug, Clone)]
    struct StaticElement {
        id: Id,
        geometry: Rectangle<i32, Physical>,
    }

    impl Element for StaticElement {
        fn id(&self) -> &Id {
            &self.id
        }

        fn current_commit(&self) -> CommitCounter {
            CommitCounter::default()
        }

        fn src(&self) -> Rectangle<f64, Buffer> {
            Rectangle::from_size((8., 8.).into())
        }

        fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
            self.geometry
        }
    }

    /// IDENTITY wrapper preserves geometry: repositioning only, no scaling.
    #[test]
    fn identity_viewport_preserves_geometry() {
        let cases = [
            // (view scale, geometry)
            (
                Scale::from(1.),
                Rectangle::new((7, 5).into(), (24, 24).into()),
            ),
            (
                Scale::from(1.),
                Rectangle::new((0, 0).into(), (100, 100).into()),
            ),
            (
                Scale::from(1.5),
                Rectangle::new((7, 5).into(), (25, 25).into()),
            ),
        ];

        for (scale, geometry) in cases {
            let view_ctx = OutputViewCtx::new(
                Rectangle::new((0., 0.).into(), (150., 150.).into()).assume_global(),
                Rectangle::new((0., 0.).into(), (100., 100.).into()).assume_local(),
                Transform::Normal,
                scale,
            );

            // Relative with a zero offset leaves geometry untouched.
            let wrapped = ZoomElement::from_element(
                StaticElement {
                    id: Id::new(),
                    geometry,
                },
                ViewportTransform::identity(),
                view_ctx,
                Point::from((0., 0.)),
                Relocate::Relative,
            );
            assert_eq!(wrapped.geometry(scale), geometry);

            // Absolute pointed at the same location also preserves size.
            let wrapped = ZoomElement::from_element(
                StaticElement {
                    id: Id::new(),
                    geometry,
                },
                ViewportTransform::identity(),
                view_ctx,
                geometry.loc.to_f64(),
                Relocate::Absolute,
            );
            assert_eq!(wrapped.geometry(scale), geometry);
        }
    }

    fn placement_viewport() -> ViewportTransform {
        ViewportTransform::new((4., 4.).into(), 2.)
    }

    #[test]
    fn tip_glued_when_scaling() {
        // Graphic top-left (10, 10), hotspot (2, 3): tip (12, 13) goes
        // through the viewport, not the top-left.
        let tip = Point::<f64, Physical>::from((12., 13.));
        let display = Point::<f64, Local>::from((12., 13.));
        let (final_pos, wrapper) =
            placement_viewport().place_cursor(tip, display, (2, 3).into(), 2., Scale::from(1.));

        // Tip displays at focal + (p - focal) * 2; top-left lands at (16, 16).
        assert_eq!(final_pos, Point::<f64, Physical>::from((16., 16.)));
        // The wrapper scales around the tip itself.
        assert_eq!(wrapper.focal, Point::<f64, Local>::from((12., 13.)));
        assert_eq!(wrapper.factor, 2.);
    }

    #[test]
    fn placement_matches_rigid_transform() {
        // Focal at the origin is a pure scale: hotspot-centered placement
        // must agree with wrapping the whole graphic.
        let viewport = ViewportTransform::new((0., 0.).into(), 2.);
        let tip = Point::<f64, Physical>::from((10., 10.));
        let display = Point::<f64, Local>::from((10., 10.));
        let (final_pos, _) =
            viewport.place_cursor(tip, display, (4, 4).into(), 2., Scale::from(1.));

        // Tip (10, 10) -> (20, 20); top-left (6, 6) -> (12, 12).
        assert_eq!(final_pos, Point::<f64, Physical>::from((12., 12.)));
    }

    #[test]
    fn placement_crosses_scale_once() {
        // Fractional output scale: Local math, one crossing each way.
        let tip = Point::<f64, Physical>::from((21., 21.));
        let display = Point::<f64, Local>::from((10.5, 10.5));
        let (final_pos, _) =
            placement_viewport().place_cursor(tip, display, (0, 0).into(), 1., Scale::from(2.));

        // Logical (10.5, 10.5) -> (17, 17) -> physical (34, 34).
        assert_eq!(final_pos, Point::<f64, Physical>::from((34., 34.)));
    }

    #[test]
    fn placement_splits_focal_and_display() {
        // Live path: raw tip anchors the wrapper, constrained display maps
        // the target.
        let focal = Point::<f64, Physical>::from((0., 0.));
        let display = Point::<f64, Local>::from((10., 10.));
        let (final_pos, wrapper) =
            placement_viewport().place_cursor(focal, display, (2, 3).into(), 2., Scale::from(1.));

        assert_eq!(final_pos, Point::<f64, Physical>::from((12., 10.)));
        assert_eq!(wrapper.focal, Point::<f64, Local>::from((0., 0.)));
    }

    #[test]
    fn zoom_filter_selects_by_band() {
        // (factor, threshold) -> filter. Bands: <= 1.0 is None (unzoomed),
        // (1.0, threshold) is Linear, [threshold, ..) is Nearest.
        let cases = [
            ((1.0, 2.0), None),
            ((0.5, 2.0), None),
            ((0.0, 2.0), None),
            ((1.001, 2.0), Some(TextureFilter::Linear)),
            ((1.5, 2.0), Some(TextureFilter::Linear)),
            ((1.999, 2.0), Some(TextureFilter::Linear)),
            ((2.0 - f64::EPSILON, 2.0), Some(TextureFilter::Linear)),
            ((2.0, 2.0), Some(TextureFilter::Nearest)),
            ((3.0, 2.0), Some(TextureFilter::Nearest)),
            ((100.0, 2.0), Some(TextureFilter::Nearest)),
            // Extreme thresholds: Nearest kicks in immediately above 1x,
            // or Linear holds for any sane factor.
            ((1.001, 1.001), Some(TextureFilter::Nearest)),
            ((100.0, 1e9), Some(TextureFilter::Linear)),
        ];

        for ((factor, threshold), expected) in cases {
            assert_eq!(
                zoom_filter(factor, threshold),
                expected,
                "zoom_filter({factor}, {threshold})"
            );
        }
    }

    #[test]
    fn zoom_filter_changed_detects_band_crossing() {
        // Crossings in either direction invalidate; staying in-band does not.
        assert!(zoom_filter_changed(
            zoom_filter(1.99, 2.0),
            zoom_filter(2.0, 2.0),
        ));
        assert!(zoom_filter_changed(
            zoom_filter(2.0, 2.0),
            zoom_filter(1.99, 2.0),
        ));
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
        // Zooming out into the unfiltered band invalidates too: None is a
        // band value here, not "unchanged".
        assert!(zoom_filter_changed(
            zoom_filter(1.2, 2.0),
            zoom_filter(1.0, 2.0),
        ));
    }

    #[test]
    fn threshold_change_invalidates_only_on_band_flip() {
        // Unzoomed: filter is None on both sides, never invalidates.
        assert!(!threshold_flips_filter(1.0, 2.0, 1.0));
        // Same threshold: nothing changes.
        assert!(!threshold_flips_filter(1.5, 2.0, 2.0));
        // 1.5x flips Linear -> Nearest when the threshold drops to 1.0.
        assert!(threshold_flips_filter(1.5, 2.0, 1.0));
        assert!(threshold_flips_filter(1.5, 1.0, 2.0));
        // Stays inside one band: no invalidation.
        assert!(!threshold_flips_filter(1.5, 2.0, 3.0));
        assert!(!threshold_flips_filter(3.0, 2.0, 2.5));
    }

    #[test]
    fn filter_change_damages_full_geometry() {
        let scale = Scale::from(1.);
        let view_ctx = OutputViewCtx::new(
            Rectangle::new((0., 0.).into(), (100., 100.).into()).assume_global(),
            Rectangle::new((0., 0.).into(), (100., 100.).into()).assume_local(),
            Transform::Normal,
            scale,
        );
        let geometry = Rectangle::new((7, 5).into(), (24, 24).into());
        let current = CommitCounter::default();
        let element = |filter_changed: bool| {
            ZoomElement::from_element(
                StaticElement {
                    id: Id::new(),
                    geometry,
                },
                ViewportTransform::identity(),
                view_ctx,
                Point::from((0., 0.)),
                Relocate::Relative,
            )
            .with_filter_changed(filter_changed)
        };

        // Inner commit matches: no damage without a filter change.
        let unchanged = element(false);
        assert!(unchanged.damage_since(scale, Some(current)).is_empty());

        // Same commit, but the filter band flipped: full element damage,
        // element-relative, so a zeroed location with the wrapped size.
        let damage = element(true).damage_since(scale, Some(current));
        assert_eq!(damage.len(), 1);
        assert_eq!(
            damage[0],
            Rectangle::new(Point::from((0, 0)), unchanged.geometry(scale).size)
        );
    }
}
