use glam::{Mat3, Vec2};
use smithay::desktop::space::SpaceElement;
use smithay::desktop::Space;
use smithay::output::Output;
use smithay::utils::{Logical, Point, Rectangle, Scale, Transform};

use crate::utils::geometry::{Global, Local, PointExt, PointGlobalExt, RectExt};

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

    pub fn focal(&self) -> Point<f64, Local> {
        self.focal
    }

    pub fn factor(&self) -> f64 {
        self.factor
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
        let focal = self.focal();
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

    /// Creates a minimal context from an output geometry rectangle.
    ///
    /// Only the output origin is meaningful for Global ↔ Local translation;
    /// transform and scale are set to defaults (Normal, 1.0).
    pub fn from_output_rect(rect: Rectangle<i32, Logical>) -> Self {
        Self::new(
            rect.to_f64().assume_global(),
            Rectangle::from_size(rect.size.to_f64()).assume_local(),
            Transform::Normal,
            Scale::from(1.0),
        )
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
        let logical_size = mode.size.to_f64().to_logical(scale);
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
    use smithay::utils::Rectangle;

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
}
