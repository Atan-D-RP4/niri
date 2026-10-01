use glam::{DMat3, DVec2};
use smithay::backend::renderer::TextureFilter;
use smithay::desktop::space::SpaceElement;
use smithay::desktop::Space;
use smithay::output::Output;
use smithay::utils::{Coordinate, Logical, Physical, Point, Rectangle, Scale, Size};

use crate::utils::geometry::{
    Global, Local, PointExt, PointGlobalExt, PointLocalExt, RectExt, RectLocalExt,
};

/// Sampled transform of output-local geometry, still [`Local`].
/// Clamping and animation policy belong to the constructor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportTransform {
    pub focal: Point<f64, Local>,
    pub factor: f64,
}

/// Output geometry plus its presentation for the current frame.
/// Frame fields are sampled once per frame so all render paths agree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputViewCtx {
    pub global_geo: Rectangle<f64, Global>,
    pub viewport: ViewportTransform,
    pub filter: Option<TextureFilter>,
    /// Filter band flipped since last frame; forces full damage.
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

    /// Content-space local point to its screen-space image.
    /// Content space is layout; screen space is the presented frame.
    pub fn content_to_screen(&self, point: Point<f64, Local>) -> Point<f64, Local> {
        let transformed = self.to_matrix() * DVec2::new(point.x, point.y).extend(1.);
        Point::new(transformed.x, transformed.y)
    }

    /// Screen-space local point back to content space.
    pub fn screen_to_content(&self, point: Point<f64, Local>) -> Point<f64, Local> {
        let transformed = self.to_matrix().inverse() * DVec2::new(point.x, point.y).extend(1.);
        Point::new(transformed.x, transformed.y)
    }

    /// Returns the axis-aligned bounding box of the transformed rectangle.
    pub fn content_to_screen_rect(&self, rect: Rectangle<f64, Local>) -> Rectangle<f64, Local> {
        self.bounding_rect(rect, |point| self.content_to_screen(point))
    }

    /// Returns the axis-aligned bounding box of the rectangle transformed back into content space.
    pub fn screen_to_content_rect(&self, rect: Rectangle<f64, Local>) -> Rectangle<f64, Local> {
        self.bounding_rect(rect, |point| self.screen_to_content(point))
    }

    /// Visible viewport: the output rect mapped back through the zoom.
    pub fn visible_viewport(&self, output_size: Size<f64, Local>) -> Rectangle<f64, Local> {
        self.screen_to_content_rect(Rectangle::from_size(output_size))
    }

    /// Constrain a screen-space point to the visible viewport.
    /// Shrinks by epsilon so edge points stay strictly inside.
    pub fn constrain_to_visible_viewport(
        &self,
        pos: Point<f64, Local>,
        output_size: Size<f64, Local>,
    ) -> Point<f64, Local> {
        let viewport = self.visible_viewport(output_size);
        pos.constrain(Rectangle::new(
            viewport.loc,
            viewport.size - Size::from((f64::EPSILON, f64::EPSILON)),
        ))
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
    /// Minimal context from an output origin, unzoomed.
    pub fn from_origin(origin: Point<f64, Logical>) -> Self {
        Self::new(Rectangle::new(origin.assume_global(), (0., 0.).into()))
    }

    /// Context with unzoomed frame state.
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

    /// Content-space global point to physical pixels.
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

/// Maps a content-space physical rect through the viewport, unrounded.
pub fn transform_rect(
    viewport: &ViewportTransform,
    rect: Rectangle<f64, Physical>,
    scale: Scale<f64>,
) -> Rectangle<f64, Physical> {
    let local = rect.to_logical(scale).assume_local();
    viewport.content_to_screen_rect(local).to_physical(scale)
}

/// Content-space physical rect to its screen image, rounded for display.
pub fn map_rect(
    viewport: &ViewportTransform,
    content: Rectangle<i32, Physical>,
    scale: Scale<f64>,
) -> Rectangle<i32, Physical> {
    transform_rect(viewport, content.to_f64(), scale).to_i32_round()
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
        assert_eq!(
            ViewportTransform::identity().content_to_screen(point),
            point
        );
        assert_eq!(
            ViewportTransform::identity().screen_to_content(point),
            point
        );
    }

    #[test]
    fn point_round_trip() {
        let transform = transform();
        let point = (140., 125.).into();
        let round_trip = transform.screen_to_content(transform.content_to_screen(point));
        assert_relative_eq!(round_trip.x, point.x);
        assert_relative_eq!(round_trip.y, point.y);
    }

    #[test]
    fn rectangle_round_trip() {
        let transform = transform();
        let rect: Rectangle<f64, Local> = Rectangle::new((50., 40.).into(), (100., 80.).into());
        assert_eq!(
            transform.screen_to_content_rect(transform.content_to_screen_rect(rect)),
            rect
        );
    }

    #[test]
    fn matrix_composition_matches_sequential_application() {
        let inner = ViewportTransform::new((100., 80.).into(), 1.5);
        let outer = ViewportTransform::new((700., 500.).into(), 2.25);
        let point: Point<f64, Local> = (350., 275.).into();
        let sequential = outer.content_to_screen(inner.content_to_screen(point));
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
            transform.content_to_screen_rect(rect),
            Rectangle::new((80., 60.).into(), (40., 60.).into())
        );
        assert_eq!(
            transform.screen_to_content_rect(rect),
            Rectangle::new((95., 75.).into(), (10., 15.).into())
        );
    }

    #[test]
    fn identity_preserves_rectangles_exactly() {
        let rect: Rectangle<f64, Local> = Rectangle::new((-10., 20.).into(), (33.5, 44.25).into());
        let identity = ViewportTransform::identity();

        assert_eq!(identity.content_to_screen_rect(rect), rect);
        assert_eq!(identity.screen_to_content_rect(rect), rect);
    }

    #[test]
    fn global_local_viewport_round_trip() {
        // Global to Local through the viewport and back.
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
                .screen_to_content(viewport.content_to_screen(local))
                .to_global(&ctx);
            assert_relative_eq!(round_trip.x, global.x, epsilon = 1e-6);
            assert_relative_eq!(round_trip.y, global.y, epsilon = 1e-6);

            // Rectangle variant of the same round trip.
            let global_rect: Rectangle<f64, Global> =
                Rectangle::new((gx, gy).into(), (800., 600.).into()).assume_global();
            let local_rect = global_rect.to_local(&ctx);
            let rt_rect = viewport
                .screen_to_content_rect(viewport.content_to_screen_rect(local_rect))
                .to_global(&ctx);
            assert_relative_eq!(rt_rect.loc.x, global_rect.loc.x, epsilon = 1e-6);
            assert_relative_eq!(rt_rect.loc.y, global_rect.loc.y, epsilon = 1e-6);
            assert_relative_eq!(rt_rect.size.w, global_rect.size.w, epsilon = 1e-6);
            assert_relative_eq!(rt_rect.size.h, global_rect.size.h, epsilon = 1e-6);
        }
    }
}
