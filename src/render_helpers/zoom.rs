use smithay::backend::renderer::element::utils::Relocate;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{FrameContext, Renderer, TextureFilter};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::render_helpers::renderer::AsGlesFrame;
use crate::utils::geometry::{Local, PointExt, PointLocalExt};
use crate::utils::view::{round_rect, transform_rect, ViewportTransform};

/// Runs a draw with the filter set, restoring `Linear` after, even on error.
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

/// True when the filter band changed; a flip needs damage despite same geometry.
pub fn zoom_filter_changed(
    previous: Option<TextureFilter>,
    current: Option<TextureFilter>,
) -> bool {
    previous != current
}

/// True when a threshold change flips the filter at `factor`; needs a redraw.
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
    location: Point<f64, Physical>,
    relocate: Relocate,
    filter: Option<TextureFilter>,
    /// Band flipped since the last materialized frame; forces full damage.
    filter_changed: bool,
}

/// Tip-glued cursor placement for the live pointer and the preview.
pub(crate) fn place_cursor(
    viewport: ViewportTransform,
    focal: Point<f64, Physical>,
    display: Point<f64, Local>,
    hotspot: Point<i32, Physical>,
    graphic_scale: f64,
    scale: Scale<f64>,
) -> (Point<f64, Physical>, ViewportTransform) {
    let focal_local: Point<f64, Local> = focal.to_logical(scale).assume_local();
    let target_rounded: Point<i32, Physical> = viewport
        .content_to_screen(display)
        .to_physical_precise_round(scale);
    let hotspot_scaled = hotspot.to_f64().upscale(graphic_scale).to_i32_round();
    (
        (target_rounded - hotspot_scaled).to_f64(),
        ViewportTransform::new(focal_local, graphic_scale),
    )
}

impl<E: Element> ZoomElement<E> {
    pub fn from_element(
        element: E,
        viewport: ViewportTransform,
        location: Point<f64, Physical>,
        relocate: Relocate,
        filter: Option<TextureFilter>,
        filter_changed: bool,
    ) -> Self {
        Self {
            element,
            viewport,
            location,
            relocate,
            filter,
            filter_changed,
        }
    }

    /// Cursor element with tip-glued placement.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cursor(
        elem: E,
        viewport: ViewportTransform,
        focal: Point<f64, Physical>,
        display: Point<f64, Local>,
        hotspot: Point<i32, Physical>,
        graphic_scale: f64,
        scale: Scale<f64>,
        filter: Option<TextureFilter>,
        filter_changed: bool,
    ) -> Self {
        let (final_pos, wrapper) =
            place_cursor(viewport, focal, display, hotspot, graphic_scale, scale);
        Self::from_element(
            elem,
            wrapper,
            final_pos,
            Relocate::Absolute,
            filter,
            filter_changed,
        )
    }
}
impl<E: Element> Element for ZoomElement<E> {
    fn id(&self) -> &Id {
        self.element.id()
    }

    fn current_commit(&self) -> CommitCounter {
        // Damage history, not a fingerprint; don't fake one.
        self.element.current_commit()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.element.src()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let mut geometry =
            transform_rect(&self.viewport, self.element.geometry(scale).to_f64(), scale);

        match self.relocate {
            Relocate::Absolute => geometry.loc = self.location,
            Relocate::Relative => geometry.loc += self.location,
        }

        // NOTE: to_i32_up() avoids jitter but oversizes the screenshot selection.
        round_rect(geometry)
    }

    fn transform(&self) -> Transform {
        self.element.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        if self.filter_changed {
            // New filter, same geometry: damage the whole element.
            return DamageSet::from_slice(&[Rectangle::new(
                Point::from((0, 0)),
                self.geometry(scale).size,
            )]);
        }
        // Damage is element-relative; the tracker adds the location.
        let inner_geometry = self.element.geometry(scale).to_f64();

        self.element
            .damage_since(scale, commit)
            .into_iter()
            .map(|rect| {
                let rect = rect.to_f64();
                let absolute = Rectangle::new(inner_geometry.loc + rect.loc, rect.size);
                let mut transformed = transform_rect(&self.viewport, absolute, scale);
                transformed.loc -= transform_rect(&self.viewport, inner_geometry, scale).loc;
                transformed.to_i32_up()
            })
            .collect()
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let inner_geometry = self.element.geometry(scale).to_f64();
        self.element
            .opaque_regions(scale)
            .into_iter()
            .map(|rect| {
                let rect = rect.to_f64();
                let absolute = Rectangle::new(inner_geometry.loc + rect.loc, rect.size);
                let mut transformed = transform_rect(&self.viewport, absolute, scale);
                transformed.loc -= transform_rect(&self.viewport, inner_geometry, scale).loc;
                // NOTE: to_i32_round() here to avoid oversizing the opaque region.
                transformed.to_i32_round()
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

    /// Identity wrapper preserves geometry.
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
            // Relative with a zero offset leaves geometry untouched.
            let wrapped = ZoomElement::from_element(
                StaticElement {
                    id: Id::new(),
                    geometry,
                },
                ViewportTransform::identity(),
                Point::from((0., 0.)),
                Relocate::Relative,
                None,
                false,
            );
            assert_eq!(wrapped.geometry(scale), geometry);

            // Absolute pointed at the same location also preserves size.
            let wrapped = ZoomElement::from_element(
                StaticElement {
                    id: Id::new(),
                    geometry,
                },
                ViewportTransform::identity(),
                geometry.loc.to_f64(),
                Relocate::Absolute,
                None,
                false,
            );
            assert_eq!(wrapped.geometry(scale), geometry);
        }
    }

    fn placement_viewport() -> ViewportTransform {
        ViewportTransform::new((4., 4.).into(), 2.)
    }

    #[test]
    fn tip_glued_when_scaling() {
        // Tip (12, 13) goes through the viewport, not the top-left.
        let tip = Point::<f64, Physical>::from((12., 13.));
        let display = Point::<f64, Local>::from((12., 13.));
        let (final_pos, wrapper) = place_cursor(
            placement_viewport(),
            tip,
            display,
            (2, 3).into(),
            2.,
            Scale::from(1.),
        );

        // Tip displays at focal + (p - focal) * 2; top-left lands at (16, 16).
        assert_eq!(final_pos, Point::<f64, Physical>::from((16., 16.)));
        assert_eq!(wrapper.focal, Point::<f64, Local>::from((12., 13.)));
        assert_eq!(wrapper.factor, 2.);
    }

    #[test]
    fn placement_matches_rigid_transform() {
        // Focal at the origin is a pure scale.
        let viewport = ViewportTransform::new((0., 0.).into(), 2.);
        let tip = Point::<f64, Physical>::from((10., 10.));
        let display = Point::<f64, Local>::from((10., 10.));
        let (final_pos, _) =
            place_cursor(viewport, tip, display, (4, 4).into(), 2., Scale::from(1.));

        // Tip (10, 10) -> (20, 20); top-left (6, 6) -> (12, 12).
        assert_eq!(final_pos, Point::<f64, Physical>::from((12., 12.)));
    }

    #[test]
    fn placement_crosses_scale_once() {
        // Fractional output scale: Local math, one crossing each way.
        let tip = Point::<f64, Physical>::from((21., 21.));
        let display = Point::<f64, Local>::from((10.5, 10.5));
        let (final_pos, _) = place_cursor(
            placement_viewport(),
            tip,
            display,
            (0, 0).into(),
            1.,
            Scale::from(2.),
        );

        // Logical (10.5, 10.5) -> (17, 17) -> physical (34, 34).
        assert_eq!(final_pos, Point::<f64, Physical>::from((34., 34.)));
    }

    #[test]
    fn placement_splits_focal_and_display() {
        // Live path: raw tip anchors the wrapper.
        let focal = Point::<f64, Physical>::from((0., 0.));
        let display = Point::<f64, Local>::from((10., 10.));
        let (final_pos, wrapper) = place_cursor(
            placement_viewport(),
            focal,
            display,
            (2, 3).into(),
            2.,
            Scale::from(1.),
        );

        assert_eq!(final_pos, Point::<f64, Physical>::from((12., 10.)));
        assert_eq!(wrapper.focal, Point::<f64, Local>::from((0., 0.)));
    }

    #[test]
    fn zoom_filter_selects_by_band() {
        // Bands: 1x or below is None, below threshold Linear, above Nearest.
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
            // Nearest right above 1x, or Linear for any sane factor.
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
        // None is a band value too, so zooming out invalidates.
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
        let geometry = Rectangle::new((7, 5).into(), (24, 24).into());
        let current = CommitCounter::default();
        let element = |filter_changed: bool| {
            ZoomElement::from_element(
                StaticElement {
                    id: Id::new(),
                    geometry,
                },
                ViewportTransform::identity(),
                Point::from((0., 0.)),
                Relocate::Relative,
                None,
                filter_changed,
            )
        };

        // Inner commit matches: no damage without a filter change.
        let unchanged = element(false);
        assert!(unchanged.damage_since(scale, Some(current)).is_empty());

        // Same commit, flipped band: full element damage at zeroed location.
        let damage = element(true).damage_since(scale, Some(current));
        assert_eq!(damage.len(), 1);
        assert_eq!(
            damage[0],
            Rectangle::new(Point::from((0, 0)), unchanged.geometry(scale).size)
        );
    }
}
