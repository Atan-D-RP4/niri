use glam::{Mat3, Vec2};
use smithay::desktop::space::SpaceElement;
use smithay::desktop::Space;
use smithay::output::Output;
use smithay::utils::{Coordinate, Logical, Physical, Point, Rectangle, Scale, Transform};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputViewCtx {
    pub global_geo: Rectangle<f64, Global>,
    pub local_geo: Rectangle<f64, Local>,
    pub output_transform: Transform,
    pub scale: Scale<f64>,
}

impl ViewportTransform {
    /// Transformation that leaves local geometry unchanged.
    pub fn identity() -> Self {
        Self::new((0., 0.).into(), 1.)
    }

    /// Creates a transform around `focal` with a positive scale factor.
    pub fn new(focal: Point<f64, Local>, factor: f64) -> Self {
        assert!(factor.is_finite() && factor > 0.);
        Self { focal, factor }
    }

    /// Applies this transform to a local point.
    pub fn apply(&self, point: Point<f64, Local>) -> Point<f64, Local> {
        let transformed = self.to_matrix() * Vec2::new(point.x as f32, point.y as f32).extend(1.);
        Point::new(transformed.x as f64, transformed.y as f64).assume_local()
    }

    /// Applies the inverse transform to a local point.
    pub fn apply_inverse(&self, point: Point<f64, Local>) -> Point<f64, Local> {
        let transformed =
            self.to_matrix().inverse() * Vec2::new(point.x as f32, point.y as f32).extend(1.);
        Point::new(transformed.x as f64, transformed.y as f64).assume_local()
    }

    /// Tip-glued cursor placement shared by the live pointer and the preview.
    ///
    /// Returns the physical position of the cursor hotspot and a new transform
    /// with the focal point in local coordinates. The hotspot is scaled by the
    /// graphic scale, which is independent of the output scale.
    pub(crate) fn place_cursor(
        self,
        focal: Point<f64, Physical>,
        display: Point<f64, Local>,
        hotspot: Point<i32, Physical>,
        graphic_scale: f64,
        scale: Scale<f64>,
    ) -> (Point<f64, Physical>, Self) {
        let focal_local: Point<f64, Local> = focal.to_logical(scale).assume_local();
        let target_rounded: Point<i32, Physical> =
            self.apply(display).to_physical_precise_round(scale);
        let hotspot_scaled = hotspot.to_f64().upscale(graphic_scale).to_i32_round();
        (
            (target_rounded - hotspot_scaled).to_f64(),
            Self::new(focal_local, graphic_scale),
        )
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
    pub fn to_matrix(&self) -> Mat3 {
        let scale = Vec2::splat(self.factor as f32);
        let focal = self.focal;
        let focal = Vec2::new(focal.x as f32, focal.y as f32);

        Mat3::from_translation(focal) * Mat3::from_scale(scale) * Mat3::from_translation(-focal)
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
    /// The output's position in global logical space.
    ///
    /// This is the translation offset for Global ↔ Local conversion via the
    /// geometry extension traits (`PointLocalExt::to_global`,
    /// `PointGlobalExt::to_local`). Use this instead of separately computing
    /// output geometry from `global_space.output_geometry()`.
    pub fn output_origin(&self) -> Point<f64, Logical> {
        self.global_geo.loc.as_logical()
    }

    /// Physical → Local: divide by [`Self::scale`] only.
    ///
    /// [`Self::output_transform`] is intentionally ignored: element geometry
    /// is already in presented orientation, and rotation composes after the
    /// viewport, keeping it axis-aligned.
    #[inline]
    pub(crate) fn physical_rect_to_local(
        &self,
        rect: Rectangle<f64, Physical>,
    ) -> Rectangle<f64, Local> {
        rect.to_logical(self.scale).assume_local()
    }
    /// Local → Physical: inverse of [`Self::physical_rect_to_local`], scale only.
    #[inline]
    pub(crate) fn local_rect_to_physical(
        &self,
        rect: Rectangle<f64, Local>,
    ) -> Rectangle<f64, Physical> {
        rect.to_physical(self.scale)
    }

    /// Converts a Local logical point into output-local Physical coordinates.
    ///
    /// Point-level counterpart of [`Self::local_rect_to_physical`]: exact
    /// unit conversion only, no pixel snapping. Use
    /// [`Self::local_point_to_physical_precise_round`] for render placement.
    #[inline]
    pub(crate) fn local_point_to_physical(&self, point: Point<f64, Local>) -> Point<f64, Physical> {
        point.to_physical(self.scale)
    }

    /// Converts a Local logical point into Physical pixels with rounding.
    ///
    /// Point-level counterpart of the `as_logical().to_physical_precise_round()`
    /// chains at render boundaries. Like Smithay's method of the same name,
    /// but sourced from the output scale with the frame carried in the types.
    #[inline]
    pub(crate) fn to_physical_precise_round<N: Coordinate>(
        self,
        point: Point<f64, Local>,
    ) -> Point<N, Physical> {
        point.to_physical_precise_round(self.scale)
    }

    /// Creates a minimal context from an output origin point.
    ///
    /// Only the origin is meaningful for Global ↔ Local translation;
    /// transform and scale are set to defaults (Normal, 1.0).
    pub fn from_origin(origin: Point<f64, Logical>) -> Self {
        Self::new(
            Rectangle::new(origin.assume_global(), (0., 0.).into()),
            Rectangle::new((0., 0.).into(), (0., 0.).into()).assume_local(),
            Transform::Normal,
            Scale::from(1.0),
        )
    }

    /// Converts output-local logical coordinates into compositor-global logical coordinates.
    ///
    /// The output transform is applied before output scale and global placement.
    pub fn new(
        global_geo: Rectangle<f64, Global>,
        local_geo: Rectangle<f64, Local>,
        output_transform: Transform,
        scale: Scale<f64>,
    ) -> Self {
        Self {
            global_geo,
            local_geo,
            output_transform,
            scale,
        }
    }

    pub fn for_output<W: SpaceElement + PartialEq>(
        global_space: &Space<W>,
        output: &Output,
    ) -> Option<Self> {
        let global_geo = global_space
            .output_geometry(output)?
            .to_f64()
            .assume_global();
        let mode = output.current_mode()?;
        let scale = output.current_scale().fractional_scale();
        // Local size is in the presented orientation: apply the output
        // transform before the scale conversion, matching
        // `Layout::output_size_for_focal`.
        let mode_size = output.current_transform().transform_size(mode.size);
        let logical_size = mode_size.to_f64().to_logical(scale);
        let local_geo = Rectangle::from_size(logical_size).assume_local();
        let transform = output.current_transform();
        Some(Self::new(
            global_geo,
            local_geo,
            transform,
            Scale::from(scale),
        ))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use glam::Vec3;
    use smithay::utils::{Point, Rectangle};

    use super::ViewportTransform;
    use crate::utils::geometry::Local;

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
        let composed = matrix * Vec3::new(point.x as f32, point.y as f32, 1.0);

        assert_relative_eq!(composed.x as f64, sequential.x, epsilon = 1e-4);
        assert_relative_eq!(composed.y as f64, sequential.y, epsilon = 1e-4);
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
}
