use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Coordinate, Physical, Point, Rectangle, Scale, Size, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::utils::view::OutputViewport;

/// Renders an element through an output view transform.
///
/// This Element's damage and opaque regions are relative to the inner element, that is, to the
/// position the element is drawn at.
#[derive(Debug)]
pub struct ViewElement<E> {
    element: E,
    scale: Scale<f64>,
    /// The view to render the element through.
    ///
    /// The view maps content space geometry to screen space by scaling the element by
    /// the viewport factor about the viewport focal point.
    viewport: OutputViewport,
    /// Overrides the position of the element. The size still comes from the view.
    ///
    /// This is used for the cursor, which is positioned by the pointer position
    /// rather than by the view.
    location: Option<Point<i32, Physical>>,
}

impl<E: Element> ViewElement<E> {
    pub fn from_element(
        element: E,
        viewport: OutputViewport,
        scale: Scale<f64>,
        location: Option<Point<i32, Physical>>,
    ) -> Self {
        Self {
            element,
            viewport,
            scale,
            location,
        }
    }

    /// Maps a rectangle from content space to screen space.
    fn apply_to_rect<C: Coordinate>(&self, rect: Rectangle<C, Physical>) -> Rectangle<C, Physical> {
        let rect = rect.to_f64();
        let transformed = self
            .viewport
            .content_to_screen_rect(rect.to_logical(self.scale))
            .to_physical(self.scale);
        let transformed_loc = Point::from((
            C::from_f64(transformed.loc.x.round()),
            C::from_f64(transformed.loc.y.round()),
        ));
        let transaction_size = Size::from((
            C::from_f64(transformed.size.w.round()),
            C::from_f64(transformed.size.h.round()),
        ));
        Rectangle::new(transformed_loc, transaction_size)
    }

    /// Maps a region relative to the wrapped element into a region relative to this
    /// element: made absolute in content space, mapped through the view, then made
    /// relative to the element's origin again.
    fn transform_relative_rect<C: Coordinate>(
        &self,
        inner_geometry: Rectangle<C, Physical>,
        rect: Rectangle<C, Physical>,
    ) -> Rectangle<C, Physical> {
        let mut absolute = rect;
        absolute.loc += inner_geometry.loc;
        let mut transformed = self.apply_to_rect(absolute);
        transformed.loc -= self.apply_to_rect(inner_geometry).loc;
        transformed
    }
}

impl<E: Element> Element for ViewElement<E> {
    fn id(&self) -> &Id {
        self.element.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.element.current_commit()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.element.src()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let inner_geometry = self.element.geometry(scale);
        let mut mapped = self.apply_to_rect(inner_geometry);
        if let Some(location) = self.location {
            mapped.loc = location;
        }
        mapped
    }

    fn transform(&self) -> Transform {
        self.element.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        // The damage is rebased by the element's transformed origin, but the inner rects
        // are rooted at the raw element geometry.
        let inner_geometry = self.element.geometry(scale);

        self.element
            .damage_since(scale, commit)
            .iter()
            .map(|rect| self.transform_relative_rect(inner_geometry, *rect))
            .collect()
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        // The opaque regions are rebased by the element's transformed origin, but the inner rects
        // are rooted at the raw element geometry.
        let inner_geometry = self.element.geometry(scale);

        self.element
            .opaque_regions(scale)
            .iter()
            .map(|rect| self.transform_relative_rect(inner_geometry, *rect))
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

impl<E: RenderElement<GlesRenderer>> RenderElement<GlesRenderer> for ViewElement<E> {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        self.element
            .draw(frame, src, dst, damage, opaque_regions, cache)
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
        self.element.capture_framebuffer(frame, src, dst, cache)
    }
}

impl<'render, E: RenderElement<TtyRenderer<'render>>> RenderElement<TtyRenderer<'render>>
    for ViewElement<E>
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
        self.element
            .draw(frame, src, dst, damage, opaque_regions, cache)
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
        self.element.capture_framebuffer(frame, src, dst, cache)
    }
}
