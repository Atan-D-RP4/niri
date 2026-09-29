use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Transform};

use crate::backend::tty::{TtyFrame, TtyRenderer, TtyRendererError};
use crate::utils::view::OutputViewport;

#[derive(Debug)]
pub struct ViewElement<E> {
    element: E,
    viewport: OutputViewport,
    scale: Scale<f64>,
}

impl<E: Element> ViewElement<E> {
    pub fn from_element(element: E, viewport: OutputViewport, scale: Scale<f64>) -> Self {
        Self {
            element,
            viewport,
            scale,
        }
    }

    fn apply_to_rect(&self, rect: Rectangle<f64, Physical>) -> Rectangle<f64, Physical> {
        let local = rect.to_logical(self.scale);
        let transformed = self.viewport.content_to_screen_rect(local);
        transformed.to_physical(self.scale)
    }

    fn transform_relative_rect(
        &self,
        inner_geometry: Rectangle<f64, Physical>,
        transformed_origin: Point<f64, Physical>,
        rect: Rectangle<f64, Physical>,
    ) -> Rectangle<f64, Physical> {
        let mut absolute = rect;
        absolute.loc += inner_geometry.loc;
        let mut transformed = self.apply_to_rect(absolute);
        transformed.loc -= transformed_origin;
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
        let geometry = self.apply_to_rect(self.element.geometry(scale).to_f64());

        let top_left = geometry.loc.to_i32_round();
        let bottom_right = (geometry.loc + geometry.size).to_i32_round();
        Rectangle::new(top_left, (bottom_right - top_left).to_size())
    }

    fn transform(&self) -> Transform {
        self.element.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let inner_geometry = self.element.geometry(scale).to_f64();
        let transformed_origin = self.apply_to_rect(inner_geometry).loc;

        self.element
            .damage_since(scale, commit)
            .iter()
            .map(|rect| {
                let transformed =
                    self.transform_relative_rect(inner_geometry, transformed_origin, rect.to_f64());
                // Round up since damage must cover at least the true extent of the transformed
                // rectangle.
                transformed.to_i32_up()
            })
            .collect()
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let inner_geometry = self.element.geometry(scale).to_f64();
        let transformed_origin = self.apply_to_rect(inner_geometry).loc;
        self.element
            .opaque_regions(scale)
            .iter()
            .map(|rect| {
                let transformed =
                    self.transform_relative_rect(inner_geometry, transformed_origin, rect.to_f64());
                // Round to nearest since opaque regions are a hint, not a guarantee.
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

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use smithay::backend::renderer::element::{Element, Id};
    use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
    use smithay::utils::{Buffer, Logical, Physical, Point, Rectangle, Scale};

    use super::ViewElement;
    use crate::utils::view::OutputViewport;

    struct TestElement {
        id: Id,
        geometry: Rectangle<i32, Physical>,
        damage: Vec<Rectangle<i32, Physical>>,
        opaque: Vec<Rectangle<i32, Physical>>,
    }

    impl TestElement {
        fn new(
            geometry: Rectangle<i32, Physical>,
            damage: Vec<Rectangle<i32, Physical>>,
            opaque: Vec<Rectangle<i32, Physical>>,
        ) -> Self {
            Self {
                id: Id::new(),
                geometry,
                damage,
                opaque,
            }
        }
    }

    impl Element for TestElement {
        fn id(&self) -> &Id {
            &self.id
        }

        fn current_commit(&self) -> CommitCounter {
            CommitCounter::default()
        }

        fn src(&self) -> Rectangle<f64, Buffer> {
            Rectangle::new((0., 0.).into(), (1., 1.).into())
        }

        fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
            self.geometry
        }

        fn damage_since(
            &self,
            _scale: Scale<f64>,
            _commit: Option<CommitCounter>,
        ) -> DamageSet<i32, Physical> {
            DamageSet::from_slice(&self.damage)
        }

        fn opaque_regions(&self, _scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
            OpaqueRegions::from_slice(&self.opaque)
        }
    }

    fn wrap(
        geometry: Rectangle<i32, Physical>,
        damage: Vec<Rectangle<i32, Physical>>,
        opaque: Vec<Rectangle<i32, Physical>>,
        viewport: OutputViewport,
        scale: Scale<f64>,
    ) -> ViewElement<TestElement> {
        ViewElement::from_element(TestElement::new(geometry, damage, opaque), viewport, scale)
    }

    #[test]
    fn identity_viewport_leaves_everything_unchanged() {
        let scale = Scale::from(1.);
        let geometry: Rectangle<i32, Physical> = Rectangle::new((30, 40).into(), (200, 100).into());
        let damage: Vec<Rectangle<i32, Physical>> = vec![
            Rectangle::new((0, 0).into(), (50, 50).into()),
            Rectangle::new((10, 10).into(), (20, 20).into()),
        ];
        let opaque: Vec<Rectangle<i32, Physical>> =
            vec![Rectangle::new((0, 0).into(), (200, 100).into())];

        let view = wrap(
            geometry,
            damage.clone(),
            opaque.clone(),
            OutputViewport::identity(),
            scale,
        );

        // The commit promises no visible behavior change without zoom.
        assert_eq!(view.geometry(scale), geometry);
        assert_eq!(&view.damage_since(scale, None)[..], &damage[..]);
        assert_eq!(&view.opaque_regions(scale)[..], &opaque[..]);
    }

    #[test]
    fn scaled_viewport_maps_geometry() {
        // Viewport doubles about (100, 80): T(p) = 2p - (100, 80).
        // (90, 70) + (20, 30) -> (80, 60) + (40, 60).
        let scale = Scale::from(1.);
        let viewport = OutputViewport::new((100., 80.).into(), 2.);
        let view = wrap(
            Rectangle::new((90, 70).into(), (20, 30).into()),
            vec![],
            vec![],
            viewport,
            scale,
        );

        assert_eq!(
            view.geometry(scale),
            Rectangle::new((80, 60).into(), (40, 60).into())
        );
    }

    #[test]
    fn mapping_round_trips_through_inverse() {
        let scale = Scale::from(2.);
        let viewport = OutputViewport::new((100., 80.).into(), 2.);
        let view = wrap(
            Rectangle::new((50, 40).into(), (100, 80).into()),
            vec![],
            vec![],
            viewport,
            scale,
        );

        // Rect-level round-trip through the element's own mapping:
        // physical -> logical -> viewport -> physical, then back.
        let rect: Rectangle<f64, Physical> = Rectangle::new((140., 125.).into(), (30., 20.).into());
        let mapped = view.apply_to_rect(rect);
        let back = viewport
            .screen_to_content_rect(mapped.to_logical(scale))
            .to_physical(scale);

        assert_relative_eq!(back.loc.x, rect.loc.x, epsilon = 1e-9);
        assert_relative_eq!(back.loc.y, rect.loc.y, epsilon = 1e-9);
        assert_relative_eq!(back.size.w, rect.size.w, epsilon = 1e-9);
        assert_relative_eq!(back.size.h, rect.size.h, epsilon = 1e-9);
    }

    #[test]
    fn damage_rebasing_with_offset_element() {
        // Viewport doubles about the origin: T(p) = 2p.
        let scale = Scale::from(1.);
        let viewport = OutputViewport::new((0., 0.).into(), 2.);

        // Element sits away from the origin; damage is element-relative.
        let damage: Vec<Rectangle<i32, Physical>> =
            vec![Rectangle::new((0, 0).into(), (50, 40).into())];
        let view = wrap(
            Rectangle::new((100, 50).into(), (200, 100).into()),
            damage,
            vec![],
            viewport,
            scale,
        );

        // Absolute damage (100, 50) + (50, 40) doubles to
        // (200, 100) + (100, 80); the transformed element origin is
        // 2 * (100, 50) = (200, 100), so rebasing back gives
        // (0, 0) + (100, 80).
        //
        // Regression guard: forgetting to add the inner loc yields
        // (-200, -100), forgetting to subtract the transformed origin
        // yields (200, 100).
        assert_eq!(
            &view.damage_since(scale, None)[..],
            &[Rectangle::new((0, 0).into(), (100, 80).into())]
        );
    }

    #[test]
    fn damage_rounds_up_and_opaque_rounds_to_nearest() {
        // 1.5x about the origin maps the element-relative (1, 1) + (2, 2)
        // to loc (1.5, 1.5), extremities (1.5, 1.5)-(4.5, 4.5), with the
        // element at the origin so no rebasing shift applies.
        let scale = Scale::from(1.);
        let viewport = OutputViewport::new((0., 0.).into(), 1.5);
        let rect: Vec<Rectangle<i32, Physical>> =
            vec![Rectangle::new((1, 1).into(), (2, 2).into())];
        let view = wrap(
            Rectangle::new((0, 0).into(), (100, 100).into()),
            rect.clone(),
            rect,
            viewport,
            scale,
        );

        let damage = view.damage_since(scale, None);
        let opaque = view.opaque_regions(scale);

        // Damage expands to cover: floor(1.5) = 1, ceil(4.5) = 5.
        assert_eq!(&damage[..], &[Rectangle::new((1, 1).into(), (4, 4).into())]);
        // Opaque rounds to nearest: round(1.5) = 2, round(3) = 3.
        assert_eq!(&opaque[..], &[Rectangle::new((2, 2).into(), (3, 3).into())]);
        assert_ne!(&damage[..], &opaque[..]);

        // Damage covers at least the true fractional extent.
        let outer = damage[0];
        assert!(outer.loc.x <= 1 && outer.loc.y <= 1);
        assert!(outer.loc.x + outer.size.w >= 5 && outer.loc.y + outer.size.h >= 5);
    }

    #[test]
    fn fractional_factor_with_non_unit_scale_stays_within_a_pixel() {
        // Viewport works in logical space: physical goes through
        // physical -> logical -> viewport -> physical.
        let scale = Scale::from(2.);
        let viewport = OutputViewport::new((37., 41.).into(), 1.3);
        let geometry: Rectangle<i32, Physical> = Rectangle::new((10, 20).into(), (100, 60).into());
        let view = wrap(geometry, vec![], vec![], viewport, scale);

        let original = geometry.to_f64();
        let mapped = view.apply_to_rect(original);
        // Non-trivial zoom: the mapping must actually move geometry.
        assert_ne!(mapped.loc.x, original.loc.x);
        assert_ne!(mapped.loc.y, original.loc.y);

        let back = viewport
            .screen_to_content_rect(mapped.to_logical(scale))
            .to_physical(scale);

        // Inverse round-trip lands within a physical pixel.
        assert!((back.loc.x - original.loc.x).abs() <= 1.);
        assert!((back.loc.y - original.loc.y).abs() <= 1.);
        assert!((back.size.w - original.size.w).abs() <= 1.);
        assert!((back.size.h - original.size.h).abs() <= 1.);
    }

    #[test]
    fn anchoring_a_screen_position_round_trips() {
        // Anchor in content space, mapped back to screen: a no-op, which is what keeps the
        // cursor on the pointer in every movement mode.
        let viewport = OutputViewport::new((300., 200.).into(), 2.5);
        let screen: Point<f64, Logical> = (640., 480.).into();
        let back = viewport.content_to_screen(viewport.screen_to_content(screen));

        assert_relative_eq!(back.x, screen.x, epsilon = 1e-9);
        assert_relative_eq!(back.y, screen.y, epsilon = 1e-9);
    }
}
