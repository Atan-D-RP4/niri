#![allow(clippy::wrong_self_convention)]

use smithay::utils::{Coordinate, Logical, Physical, Point, Rectangle, Scale, Size};

use crate::utils::view::OutputViewCtx;

/// Compositor-wide frame: positions absolute across all outputs, in logical units.
///
/// Global is the frame of cross-output concerns: hit-testing entry points such as
/// `output_under`, cursor and pointer-grab state, and surface-origin bookkeeping.
/// Translation to and from [`Local`] is a pure origin shift through [`OutputViewCtx`];
/// output rotation and scale never participate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Global;

/// Per-output frame: positions relative to an output's origin, in logical units.
///
/// Local is the frame of output-relative geometry: render locations, tile and layer positions,
/// and the `XrayPos` accumulation. The frame marks scope, not lifetime: values persist in layout
/// data (e.g. `TileData.logical_pos`) as well as flowing through a single render pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Local;

/// A position or offset relative to a surface origin.
///
/// Unlike [`Global`] and [`Local`], this is not a compositor frame at all: values are constructed
/// and consumed within a single pass — pointer offsets within the focused surface on the input
/// side, popup placement on the render side. It exists to keep surface-relative values from being
/// mistaken for output-relative or global positions.
///
/// Do not add further frame markers (e.g. WorkspaceLocal, BackdropLocal) without a
/// call-site-driven reason: screen/content/surface distinctions stay nominal (see
/// `screen_to_content`) until a second consumer demonstrably confuses them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceLocal;

pub(crate) trait PointExt<C: Coordinate> {
    /// Relabels a bare Logical point as Global without translating it.
    fn assume_global(self) -> Point<C, Global>;
    /// Relabels a bare Logical point as Local without translating it.
    fn assume_local(self) -> Point<C, Local>;
    /// Relabels a bare Logical point as SurfaceLocal without translating it.
    fn assume_surface_local(self) -> Point<C, SurfaceLocal>;
}

pub trait PointLocalExt<C: Coordinate> {
    /// Relabels a Local point for a Logical-only API; this does not translate it.
    fn as_logical(self) -> Point<C, Logical>;
    /// Translates an output-local point into the global frame using the output origin.
    fn to_global(self, ctx: &OutputViewCtx) -> Point<C, Global>;
    /// Converts a Local point into Physical coordinates using the given scale.
    fn to_physical(self, scale: impl Into<Scale<C>>) -> Point<C, Physical>;
    /// Converts a Local point into Physical coordinates using the given scale, rounding precisely
    /// to the nearest integer.
    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Point<R, Physical>;
}

#[allow(dead_code)]
pub(crate) trait PointGlobalExt<C: Coordinate> {
    /// Relabels a Global point for a Logical-only API; this does not translate it.
    fn as_logical(self) -> Point<C, Logical>;
    /// Translates a global point into output-local coordinates using the output origin.
    fn to_local(self, ctx: &OutputViewCtx) -> Point<C, Local>;
    /// Computes a surface-relative offset from this global point and a surface origin.
    fn surface_offset(self, surface_origin: Point<C, Global>) -> Point<C, SurfaceLocal>;
    /// Places a surface-relative logical position into the global frame through this origin.
    fn surface_position(self, surface_location: Point<C, Logical>) -> Point<C, Global>;
    /// Converts a Global point into Physical coordinates using the given scale.
    fn to_physical(self, scale: impl Into<Scale<C>>) -> Point<C, Physical>;
    /// Converts a Global point into Physical coordinates using the given scale, rounding precisely
    /// to the nearest integer.
    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Point<R, Physical>;
}

pub(crate) trait PointSurfaceLocalExt<C: Coordinate> {
    /// Relabels a SurfaceLocal point for a Logical-only API; this does not translate it.
    fn as_logical(self) -> Point<C, Logical>;
    /// Translates a surface-local point into output-local coordinates using the surface origin.
    fn to_local(self, surface_origin: Point<C, Local>) -> Point<C, Local>;
}

pub(crate) trait RectExt<C: Coordinate> {
    /// Relabels a bare Logical rectangle as Global without translating it.
    fn assume_global(self) -> Rectangle<C, Global>;
    /// Relabels a bare Logical rectangle as Local without translating it.
    fn assume_local(self) -> Rectangle<C, Local>;
    /// Relabels a bare Logical rectangle as SurfaceLocal without translating it.
    fn assume_surface_local(self) -> Rectangle<C, SurfaceLocal>;
}

#[allow(dead_code)]
pub(crate) trait RectLocalExt<C: Coordinate> {
    /// Relabels a Local rectangle for a Logical-only API; this does not translate it.
    fn as_logical(self) -> Rectangle<C, Logical>;
    /// Translates an output-local rectangle into the global frame using the output origin.
    fn to_global(self, ctx: &OutputViewCtx) -> Rectangle<C, Global>;
    /// Converts a Local rectangle into Physical coordinates using the given scale.
    fn to_physical(self, scale: impl Into<Scale<C>>) -> Rectangle<C, Physical>;
    /// Converts a Local rectangle into Physical coordinates using the given scale, rounding
    /// precisely to the nearest integer.
    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Rectangle<R, Physical>;
}

#[allow(dead_code)]
pub(crate) trait RectGlobalExt<C: Coordinate> {
    /// Relabels a Global rectangle for a Logical-only API; this does not translate it.
    fn as_logical(self) -> Rectangle<C, Logical>;
    /// Translates a global rectangle into output-local coordinates using the output origin.
    fn to_local(self, ctx: &OutputViewCtx) -> Rectangle<C, Local>;
    /// Converts a Global rectangle into Physical coordinates using the given scale.
    fn to_physical(self, scale: impl Into<Scale<C>>) -> Rectangle<C, Physical>;
    /// Converts a Global rectangle into Physical coordinates using the given scale, rounding
    /// precisely to the nearest integer.
    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Rectangle<R, Physical>;
}

// `pub` for `niri-visual-tests` which builds `Tile` sizes
// through this trait; point/rect assertions stay crate-private.
pub trait SizeExt<C: Coordinate> {
    /// Relabels a Size as Logical without translating it.
    fn as_logical(self) -> Size<C, Logical>;
    /// Relabels a Size as Global without translating it.
    fn assume_global(self) -> Size<C, Global>;
    /// Relabels a Size as Local without translating it.
    fn assume_local(self) -> Size<C, Local>;
    /// Relabels a Size as SurfaceLocal without translating it.
    fn assume_surface_local(self) -> Size<C, SurfaceLocal>;
}

impl<C: Coordinate> PointExt<C> for Point<C, Logical> {
    fn assume_global(self) -> Point<C, Global> {
        (self.x, self.y).into()
    }

    fn assume_local(self) -> Point<C, Local> {
        (self.x, self.y).into()
    }

    fn assume_surface_local(self) -> Point<C, SurfaceLocal> {
        (self.x, self.y).into()
    }
}

impl<C: Coordinate> PointLocalExt<C> for Point<C, Local> {
    fn as_logical(self) -> Point<C, Logical> {
        (self.x, self.y).into()
    }

    fn to_global(self, ctx: &OutputViewCtx) -> Point<C, Global> {
        let origin = ctx.output_origin();
        let point = self.to_f64().as_logical() + origin;
        (C::from_f64(point.x), C::from_f64(point.y)).into()
    }

    fn to_physical(self, scale: impl Into<Scale<C>>) -> Point<C, Physical> {
        self.as_logical().to_physical(scale)
    }

    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Point<R, Physical> {
        self.as_logical().to_physical_precise_round(scale)
    }
}

impl<C: Coordinate> PointGlobalExt<C> for Point<C, Global> {
    fn as_logical(self) -> Point<C, Logical> {
        (self.x, self.y).into()
    }

    fn to_local(self, ctx: &OutputViewCtx) -> Point<C, Local> {
        let origin = ctx.output_origin();
        let point = self.to_f64().as_logical() - origin;
        (C::from_f64(point.x), C::from_f64(point.y)).into()
    }

    fn surface_offset(self, surface_origin: Point<C, Global>) -> Point<C, SurfaceLocal> {
        let offset = self.to_f64().as_logical() - surface_origin.to_f64().as_logical();
        (C::from_f64(offset.x), C::from_f64(offset.y)).into()
    }

    fn surface_position(self, surface_location: Point<C, Logical>) -> Point<C, Global> {
        let position = self.to_f64().as_logical() + surface_location.to_f64();
        (C::from_f64(position.x), C::from_f64(position.y)).into()
    }

    fn to_physical(self, scale: impl Into<Scale<C>>) -> Point<C, Physical> {
        self.as_logical().to_physical(scale)
    }

    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Point<R, Physical> {
        self.as_logical().to_physical_precise_round(scale)
    }
}

impl<C: Coordinate> PointSurfaceLocalExt<C> for Point<C, SurfaceLocal> {
    fn as_logical(self) -> Point<C, Logical> {
        (self.x, self.y).into()
    }

    fn to_local(self, surface_origin: Point<C, Local>) -> Point<C, Local> {
        let offset = self.to_f64().as_logical();
        let point = surface_origin.to_f64().as_logical() + offset;
        (C::from_f64(point.x), C::from_f64(point.y)).into()
    }
}

impl<C: Coordinate> RectExt<C> for Rectangle<C, Logical> {
    fn assume_global(self) -> Rectangle<C, Global> {
        Rectangle::new(self.loc.assume_global(), self.size.assume_global())
    }

    fn assume_local(self) -> Rectangle<C, Local> {
        Rectangle::new(self.loc.assume_local(), self.size.assume_local())
    }

    fn assume_surface_local(self) -> Rectangle<C, SurfaceLocal> {
        Rectangle::new(
            self.loc.assume_surface_local(),
            self.size.assume_surface_local(),
        )
    }
}

impl<C: Coordinate> RectLocalExt<C> for Rectangle<C, Local> {
    fn as_logical(self) -> Rectangle<C, Logical> {
        Rectangle::new(self.loc.as_logical(), self.size.as_logical())
    }

    fn to_global(self, ctx: &OutputViewCtx) -> Rectangle<C, Global> {
        Rectangle::new(self.loc.to_global(ctx), self.size.assume_global())
    }

    fn to_physical(self, scale: impl Into<Scale<C>>) -> Rectangle<C, Physical> {
        self.as_logical().to_physical(scale)
    }

    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Rectangle<R, Physical> {
        self.as_logical().to_physical_precise_round(scale)
    }
}

impl<C: Coordinate> RectGlobalExt<C> for Rectangle<C, Global> {
    fn as_logical(self) -> Rectangle<C, Logical> {
        Rectangle::new(self.loc.as_logical(), self.size.as_logical())
    }

    fn to_local(self, ctx: &OutputViewCtx) -> Rectangle<C, Local> {
        Rectangle::new(self.loc.to_local(ctx), self.size.assume_local())
    }

    fn to_physical(self, scale: impl Into<Scale<C>>) -> Rectangle<C, Physical> {
        self.as_logical().to_physical(scale)
    }

    fn to_physical_precise_round<S: Coordinate, R: Coordinate>(
        self,
        scale: impl Into<Scale<S>>,
    ) -> Rectangle<R, Physical> {
        self.as_logical().to_physical_precise_round(scale)
    }
}

impl<C: Coordinate> SizeExt<C> for Size<C, Logical> {
    fn as_logical(self) -> Size<C, Logical> {
        self
    }

    fn assume_global(self) -> Size<C, Global> {
        (self.w, self.h).into()
    }

    fn assume_local(self) -> Size<C, Local> {
        (self.w, self.h).into()
    }

    fn assume_surface_local(self) -> Size<C, SurfaceLocal> {
        (self.w, self.h).into()
    }
}

impl<C: Coordinate> SizeExt<C> for Size<C, Global> {
    fn as_logical(self) -> Size<C, Logical> {
        (self.w, self.h).into()
    }

    fn assume_global(self) -> Size<C, Global> {
        self
    }

    fn assume_local(self) -> Size<C, Local> {
        (self.w, self.h).into()
    }

    fn assume_surface_local(self) -> Size<C, SurfaceLocal> {
        (self.w, self.h).into()
    }
}

impl<C: Coordinate> SizeExt<C> for Size<C, Local> {
    fn as_logical(self) -> Size<C, Logical> {
        (self.w, self.h).into()
    }

    fn assume_global(self) -> Size<C, Global> {
        (self.w, self.h).into()
    }

    fn assume_local(self) -> Size<C, Local> {
        self
    }

    fn assume_surface_local(self) -> Size<C, SurfaceLocal> {
        (self.w, self.h).into()
    }
}

impl<C: Coordinate> SizeExt<C> for Size<C, SurfaceLocal> {
    fn as_logical(self) -> Size<C, Logical> {
        (self.w, self.h).into()
    }

    fn assume_global(self) -> Size<C, Global> {
        (self.w, self.h).into()
    }

    fn assume_local(self) -> Size<C, Local> {
        (self.w, self.h).into()
    }

    fn assume_surface_local(self) -> Size<C, SurfaceLocal> {
        self
    }
}

#[cfg(test)]
mod tests {
    use smithay::utils::{Logical, Point, Rectangle};

    use super::{
        Global, Local, PointGlobalExt, PointLocalExt, PointSurfaceLocalExt, RectExt, RectGlobalExt,
        RectLocalExt, SizeExt,
    };
    use crate::utils::view::OutputViewCtx;

    #[test]
    fn local_to_physical_matches_logical_conversion() {
        use smithay::utils::{Physical, Scale};

        let point: Point<f64, Local> = (3., 4.).into();
        let scale = Scale::from(2.);
        assert_eq!(
            point.to_physical(scale),
            point.as_logical().to_physical(scale),
        );
        let rounded: Point<i32, Physical> = point.to_physical_precise_round(scale);
        let expected: Point<i32, Physical> = point.as_logical().to_physical_precise_round(scale);
        assert_eq!(rounded, expected);

        let rect = Rectangle::new(point, (10., 20.).into());
        assert_eq!(
            rect.to_physical(scale),
            rect.as_logical().to_physical(scale),
        );
        let rounded: Rectangle<i32, Physical> = rect.to_physical_precise_round(scale);
        let expected: Rectangle<i32, Physical> = rect.as_logical().to_physical_precise_round(scale);
        assert_eq!(rounded, expected);
    }

    #[test]
    fn surface_offsets_have_their_own_frame() {
        let point: Point<f64, Global> = (120., 90.).into();
        let origin: Point<f64, Global> = (40., 25.).into();

        let offset = point.surface_offset(origin);
        assert_eq!(offset.as_logical(), Point::<f64, Logical>::from((80., 65.)));
    }

    #[test]
    fn surface_position_round_trips_with_surface_offset() {
        let origin: Point<f64, Global> = (40., 25.).into();
        let location: Point<f64, Logical> = (80., 65.).into();

        let point = origin.surface_position(location);
        assert_eq!(point.surface_offset(origin).as_logical(), location);
    }

    #[test]
    fn global_local_point_conversion_round_trips() {
        let ctx = OutputViewCtx::new(
            Rectangle::new((40., 25.).into(), (1920., 1080.).into()),
            Rectangle::new((0., 0.).into(), (1920., 1080.).into()).assume_local(),
            Default::default(),
            (1., 1.).into(),
        );
        let point: Point<f64, Global> = (120., 90.).into();

        let local: Point<f64, Local> = point.to_local(&ctx);
        assert_eq!(local.to_global(&ctx), point);
    }

    #[test]
    fn global_local_rectangle_conversion_round_trips() {
        let ctx = OutputViewCtx::new(
            Rectangle::new((40., 25.).into(), (1920., 1080.).into()),
            Rectangle::new((0., 0.).into(), (1920., 1080.).into()).assume_local(),
            Default::default(),
            (1., 1.).into(),
        );
        let logical = Rectangle::new((120., 90.).into(), (800., 600.).into());
        let global = logical.assume_global();

        let local = global.to_local(&ctx);
        assert_eq!(local.loc, (80., 65.).into());
        assert_eq!(local.size, logical.size.assume_local());
        assert_eq!(local.to_global(&ctx), global);

        // And back the other way from a Local literal.
        let local = Rectangle::new((80., 65.).into(), (800., 600.).into());
        let global = local.to_global(&ctx);
        assert_eq!(global.loc, (120., 90.).into());
        assert_eq!(global.size, local.size.assume_global());
        assert_eq!(global.to_local(&ctx), local);
    }
}
