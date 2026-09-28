use glam::{DMat3, DVec2};
use smithay::backend::renderer::TextureFilter;
use smithay::desktop::space::SpaceElement;
use smithay::desktop::Space;
use smithay::output::Output;
use smithay::utils::{Coordinate, Logical, Physical, Point, Rectangle, Scale};

use crate::utils::geometry::{
    Global, Local, PointExt, PointGlobalExt, PointLocalExt, RectExt, RectLocalExt,
};

/// Immutable, sampled transformation of output-local logical geometry.
///
/// This is a view operation, not a coordinate-frame conversion. Both input and
/// output remain [`Local`]. Policy such as focal-point clamping and animation
/// belongs to the owner that constructs this value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportTransform {
    pub focal: Point<f64, Local>,
    pub factor: f64,
}

/// An output's geometry, plus how the current frame is presented for it.
///
/// The geometry fields are rebuilt on resize; the frame fields are sampled
/// once per presented frame (see `Niri::sample_frame_view`), so every render
/// path for a frame agrees on the viewport and the magnification filter band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputViewCtx {
    pub global_geo: Rectangle<f64, Global>,
    pub viewport: ViewportTransform,
    pub filter: Option<TextureFilter>,
    /// The filter band flipped since the last materialized frame; forces full
    /// damage. Cleared on the next frame sample.
    pub filter_changed: bool,
    pub scale_cursor: bool,
}

impl ViewportTransform {
    /// Transformation that leaves local geometry unchanged.
    pub fn identity() -> Self {
        Self::new((0., 0.).into(), 1.)
    }

    /// Creates a transform around `focal` with a positive scale factor.
    pub fn new(focal: Point<f64, Local>, factor: f64) -> Self {
        Self { focal, factor }
    }

    /// Applies this transform to a local point.
    pub fn apply(&self, point: Point<f64, Local>) -> Point<f64, Local> {
        let transformed = self.to_matrix() * DVec2::new(point.x, point.y).extend(1.);
        Point::new(transformed.x, transformed.y)
    }

    /// Applies the inverse transform to a local point.
    pub fn apply_inverse(&self, point: Point<f64, Local>) -> Point<f64, Local> {
        let transformed = self.to_matrix().inverse() * DVec2::new(point.x, point.y).extend(1.);
        Point::new(transformed.x, transformed.y)
    }

    /// Returns the axis-aligned bounding box of the transformed rectangle.
    pub fn apply_rect(&self, rect: Rectangle<f64, Local>) -> Rectangle<f64, Local> {
        self.bounding_rect(rect, |point| self.apply(point))
    }

    /// Returns the axis-aligned bounding box of the inverse image of `rect`.
    pub fn apply_inverse_rect(&self, rect: Rectangle<f64, Local>) -> Rectangle<f64, Local> {
        self.bounding_rect(rect, |point| self.apply_inverse(point))
    }

    /// Returns the equivalent 2D affine matrix.
    pub fn to_matrix(&self) -> DMat3 {
        let scale = DVec2::splat(self.factor);
        let focal = self.focal;
        let focal = DVec2::new(focal.x, focal.y);

        DMat3::from_translation(focal) * DMat3::from_scale(scale) * DMat3::from_translation(-focal)
    }

    fn bounding_rect(
        &self,
        rect: Rectangle<f64, Local>,
        map: impl Fn(Point<f64, Local>) -> Point<f64, Local>,
    ) -> Rectangle<f64, Local> {
        let bottom_right = rect.loc + rect.size.to_f64();
        Rectangle::bounding_box(
            [
                rect.loc,
                (bottom_right.x, rect.loc.y).into(),
                (rect.loc.x, bottom_right.y).into(),
                bottom_right,
            ]
            .into_iter()
            .map(map),
        )
    }
}

impl OutputViewCtx {
    /// Creates a minimal context from an output origin point.
    ///
    /// Only the origin is meaningful for Global ↔ Local translation, and the
    /// frame is unzoomed.
    pub fn from_origin(origin: Point<f64, Logical>) -> Self {
        Self::new(Rectangle::new(origin.assume_global(), (0., 0.).into()))
    }

    /// Creates a context with unzoomed frame state.
    ///
    /// The frame fields are overwritten once per presented frame.
    pub fn new(global_geo: Rectangle<f64, Global>) -> Self {
        Self {
            global_geo,
            viewport: ViewportTransform::identity(),
            filter: None,
            filter_changed: false,
            scale_cursor: true,
        }
    }

    /// This output's view with the frame state cleared, for native captures.
    pub fn unzoomed(&self) -> Self {
        Self {
            viewport: ViewportTransform::identity(),
            filter: None,
            filter_changed: false,
            scale_cursor: true,
            ..*self
        }
    }

    /// Converts a content-space global point to physical pixels.
    ///
    /// Mirrors [`PointExt::to_local`]: Global → Local via the output origin,
    /// then Local -> Physical via the caller's output scale. Generic over the
    /// coordinate so call sites keep their existing rounding behavior.
    pub fn to_physical<R: Coordinate>(
        &self,
        pos: Point<f64, Global>,
        scale: Scale<f64>,
    ) -> Point<R, Physical> {
        pos.to_local(self).to_physical_precise_round(scale)
    }

    pub fn for_output<W: SpaceElement + PartialEq>(
        global_space: &Space<W>,
        output: &Output,
    ) -> Option<Self> {
        let global_geo = global_space
            .output_geometry(output)?
            .to_f64()
            .assume_global();
        Some(Self::new(global_geo))
    }
}

/// Rounding convention shared by `ZoomElement::geometry` and screenshot export, so
/// the export crop aligns with the displayed geometry. Damage rounds looser.
pub(crate) fn round_rect(rect: Rectangle<f64, Physical>) -> Rectangle<i32, Physical> {
    let loc = rect.loc.to_i32_round();
    let bottom_right = (rect.loc + rect.size).to_i32_round();
    Rectangle::new(loc, (bottom_right - loc).to_size())
}

/// Maps a content-space physical rect through the viewport, unrounded.
pub fn transform_rect(
    viewport: &ViewportTransform,
    rect: Rectangle<f64, Physical>,
    scale: Scale<f64>,
) -> Rectangle<f64, Physical> {
    let local = rect.to_logical(scale).assume_local();
    viewport.apply_rect(local).to_physical(scale)
}

/// Content-space physical rect to its screen image, rounded for display.
pub fn map_rect(
    viewport: &ViewportTransform,
    content: Rectangle<i32, Physical>,
    scale: Scale<f64>,
) -> Rectangle<i32, Physical> {
    round_rect(transform_rect(viewport, content.to_f64(), scale))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use glam::DVec3;
    use smithay::utils::{Point, Rectangle};

    use super::{OutputViewCtx, ViewportTransform};
    use crate::utils::geometry::{
        Global, Local, PointGlobalExt, PointLocalExt, RectExt, RectGlobalExt, RectLocalExt,
    };

    fn transform() -> ViewportTransform {
        ViewportTransform::new((100., 80.).into(), 2.)
    }

    #[test]
    fn identity_is_identity() {
        let point = (20., 30.).into();
        assert_eq!(ViewportTransform::identity().apply(point), point);
        assert_eq!(ViewportTransform::identity().apply_inverse(point), point);
    }

    #[test]
    fn point_round_trip() {
        let transform = transform();
        let point = (140., 125.).into();
        let round_trip = transform.apply_inverse(transform.apply(point));
        assert_relative_eq!(round_trip.x, point.x);
        assert_relative_eq!(round_trip.y, point.y);
    }

    #[test]
    fn rectangle_round_trip() {
        let transform = transform();
        let rect: Rectangle<f64, Local> = Rectangle::new((50., 40.).into(), (100., 80.).into());
        assert_eq!(
            transform.apply_inverse_rect(transform.apply_rect(rect)),
            rect
        );
    }

    #[test]
    fn matrix_composition_matches_sequential_application() {
        let inner = ViewportTransform::new((100., 80.).into(), 1.5);
        let outer = ViewportTransform::new((700., 500.).into(), 2.25);
        let point: Point<f64, Local> = (350., 275.).into();
        let sequential = outer.apply(inner.apply(point));
        let matrix = outer.to_matrix() * inner.to_matrix();
        let composed = matrix * DVec3::new(point.x, point.y, 1.0);

        assert_relative_eq!(composed.x, sequential.x, epsilon = 1e-4);
        assert_relative_eq!(composed.y, sequential.y, epsilon = 1e-4);
    }

    #[test]
    fn rectangle_image_and_preimage_have_expected_bounds() {
        let transform = ViewportTransform::new((100., 80.).into(), 2.);
        let rect: Rectangle<f64, Local> = Rectangle::new((90., 70.).into(), (20., 30.).into());

        assert_eq!(
            transform.apply_rect(rect),
            Rectangle::new((80., 60.).into(), (40., 60.).into())
        );
        assert_eq!(
            transform.apply_inverse_rect(rect),
            Rectangle::new((95., 75.).into(), (10., 15.).into())
        );
    }

    #[test]
    fn identity_preserves_rectangles_exactly() {
        let rect: Rectangle<f64, Local> = Rectangle::new((-10., 20.).into(), (33.5, 44.25).into());
        let identity = ViewportTransform::identity();

        assert_eq!(identity.apply_rect(rect), rect);
        assert_eq!(identity.apply_inverse_rect(rect), rect);
    }

    #[test]
    fn global_local_viewport_round_trip() {
        // Cross-abstraction: Global → Local → Viewport → inverse Viewport → Global,
        // with nonzero and negative output origins.
        let viewport = ViewportTransform::new((100., 80.).into(), 2.);
        let cases = [
            // (origin, global point)
            ((40., 25.), (120., 90.)),
            ((-200., -100.), (-80., -30.)),
        ];

        for ((ox, oy), (gx, gy)) in cases {
            let ctx = OutputViewCtx::new(Rectangle::new((ox, oy).into(), (1920., 1080.).into()));
            let global: Point<f64, Global> = (gx, gy).into();

            let local = global.to_local(&ctx);
            let round_trip = viewport
                .apply_inverse(viewport.apply(local))
                .to_global(&ctx);
            assert_relative_eq!(round_trip.x, global.x, epsilon = 1e-6);
            assert_relative_eq!(round_trip.y, global.y, epsilon = 1e-6);

            // Rectangle variant: translate loc, preserve size across the pipeline.
            let global_rect: Rectangle<f64, Global> =
                Rectangle::new((gx, gy).into(), (800., 600.).into()).assume_global();
            let local_rect = global_rect.to_local(&ctx);
            let rt_rect = viewport
                .apply_inverse_rect(viewport.apply_rect(local_rect))
                .to_global(&ctx);
            assert_relative_eq!(rt_rect.loc.x, global_rect.loc.x, epsilon = 1e-6);
            assert_relative_eq!(rt_rect.loc.y, global_rect.loc.y, epsilon = 1e-6);
            assert_relative_eq!(rt_rect.size.w, global_rect.size.w, epsilon = 1e-6);
            assert_relative_eq!(rt_rect.size.h, global_rect.size.h, epsilon = 1e-6);
        }
    }
}
