use std::time::Duration;

use niri_config::ZoomMovementMode;
use smithay::output::Output;
use smithay::utils::{Point, Rectangle, Size};

use crate::animation::{Animation, Clock};
use crate::input::swipe_tracker::SwipeTracker;
use crate::utils::geometry::{Global, Local, PointLocalExt, SizeExt};
use crate::utils::view::{OutputViewCtx, ViewportTransform};

/// Per-output zoom state. Layout writes these every animation tick;
/// external consumers read via `Layout`'s public API.
///
/// Level and focal transitions are stored independently — they share a clock
/// and config for synchronization but have separate lifecycles.
#[derive(Debug, Clone)]
pub struct OutputZoomState {
    pub level: f64,
    pub focal: Point<f64, Local>,
    pub locked: bool,
    pub level_transition: ZoomLevelTransition,
    pub focal_animation: Option<ZoomFocalAnimation>,
}

impl OutputZoomState {
    pub fn new_for_output(output: &Output) -> Self {
        let mode_size = output.current_mode().map_or((0, 0).into(), |m| m.size);
        let scale = output.current_scale().fractional_scale();
        let logical_size = mode_size.to_f64().to_logical(scale);
        Self {
            level: 1.0,
            focal: Point::from((logical_size.w / 2.0, logical_size.h / 2.0)),
            locked: false,
            level_transition: ZoomLevelTransition::Idle,
            focal_animation: None,
        }
    }

    /// True when any transition (level or focal) is active and not yet done.
    pub fn transitioning(&self) -> bool {
        !matches!(self.level_transition, ZoomLevelTransition::Idle)
            || self.focal_animation.is_some()
    }

    /// Returns true when any `Animating` transition is active (not `Gesturing`).
    ///
    /// Used by `are_animations_ongoing()` to avoid driving the render loop
    /// during gestures — gesture updates already call `queue_redraw()`
    /// explicitly, so the VBlank-driven redraw loop is unnecessary and
    /// creates a render storm that can starve input processing.
    pub fn is_animating(&self) -> bool {
        matches!(self.level_transition, ZoomLevelTransition::Animating(_))
            || self.focal_animation.is_some()
    }

    /// Compute the current level from the active animation state.
    // `now` is threaded through for the gesture-driven paths that
    // Integrating Zoom 1 wires up; the idle check reads the clock directly.
    #[allow(clippy::let_and_return)]
    fn current_level(&self, now: Duration) -> f64 {
        match &self.level_transition {
            ZoomLevelTransition::Animating(a) => a.value_at(now),
            ZoomLevelTransition::Gesturing(g) => g.current_level,
            ZoomLevelTransition::Idle => self.level,
        }
    }

    /// Compute the current focal point from the active animation state.
    // See `current_level`: `now` is consumed by Integrating Zoom 1.
    #[allow(clippy::let_and_return)]
    fn current_focal(&self, now: Duration) -> Point<f64, Local> {
        let level = self.current_level(now);

        match &self.focal_animation {
            Some(a) => a.value_at(now),
            None => {
                // When no focal animation is active, compute focal from the
                // active level transition's tracking context.
                match &self.level_transition {
                    ZoomLevelTransition::Animating(a) => {
                        a.tracking.compute_focal(level, self.focal)
                    }
                    ZoomLevelTransition::Gesturing(g) => g.compute_focal_or(level, g.current_focal),
                    ZoomLevelTransition::Idle => self.focal,
                }
            }
        }
    }

    /// Sweep completed transitions and commit final values to resting state.
    ///
    /// Called from `Layout::advance_animations` on the same tick as all other
    /// animation sweeps. When an animation completes, its final level/focal
    /// are stored as the resting state for the `Idle` variant.
    pub fn advance_animations(&mut self, now: Duration) {
        self.level = self.current_level(now);
        self.focal = self.current_focal(now);
        self.level_transition.sweep_at(now);
        if let Some(a) = &self.focal_animation {
            if a.x_anim.is_done() && a.y_anim.is_done() {
                self.focal_animation = None;
            }
        }
    }

    /// Update cursor position on active transitions for focal tracking.
    pub fn set_cursor_pos(&mut self, pos: Point<f64, Local>) {
        match &mut self.level_transition {
            ZoomLevelTransition::Animating(a) => a.set_cursor_pos(pos),
            ZoomLevelTransition::Gesturing(g) => g.set_cursor_pos(pos),
            ZoomLevelTransition::Idle => {}
        }
    }

    /// Update the movement mode on the active level transition's tracking
    /// context. The OnEdge anchor is recomputed for the new mode so that
    /// subsequent focal computations use the correct mode.
    ///
    /// Does nothing when no level transition is active — the movement mode
    /// is read fresh from config by `update_zoom_base_focal` in that case.
    pub fn update_movement_mode(&mut self, mode: ZoomMovementMode) {
        match &mut self.level_transition {
            ZoomLevelTransition::Animating(a) => {
                let level = a.anim.value();
                // Compute focal from the tracking context rather than
                // using self.focal directly — self.focal may be stale
                // (it's not updated until advance_animations).
                let focal = a.tracking.compute_focal(level, self.focal);
                a.set_movement_mode(mode, level, focal);
            }
            ZoomLevelTransition::Gesturing(g) => {
                g.set_movement_mode(mode, g.current_level, g.current_focal);
            }
            ZoomLevelTransition::Idle => {}
        }
    }

    /// Canonical viewport transform for the current animated zoom state.
    ///
    /// This is the single construction path for [`ViewportTransform`]. No
    /// consumer should reconstruct the transform from raw animation fields.
    pub fn viewport_transform(&self, now: Duration) -> ViewportTransform {
        let focal = self.current_focal(now);
        let level = self.current_level(now);
        ViewportTransform::new(focal, level)
    }

    /// Viewport rectangle in the global coordinate frame for the current
    /// animated zoom state.
    ///
    /// Computes the viewport in Local space via [`ViewportTransform`], then
    /// translates to Global by adding the output origin. This does NOT apply
    /// the output transform (rotation/reflection) — that is a render-stage
    /// concern per the geometry pipeline.
    pub fn viewport_global(
        &self,
        view_ctx: &OutputViewCtx,
        now: Duration,
    ) -> Rectangle<f64, Global> {
        let vt = self.viewport_transform(now);
        let output_local = Rectangle::from_size(view_ctx.local_geo.size);
        let viewport_local = vt.apply_inverse_rect(output_local);
        // Local→Global is a pure translation of location by the output origin.
        // intentionally not part of the viewport rectangle calculation.
        let global_loc = viewport_local.loc.to_global(view_ctx);
        Rectangle::new(global_loc, viewport_local.size.assume_global())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FocalTrackingContext {
    cursor_pos: Option<Point<f64, Local>>,
    output_size: Option<Size<f64, Local>>,
    movement_mode: Option<ZoomMovementMode>,
    on_edge_cursor_anchor: Option<Point<f64, Local>>,
}

impl FocalTrackingContext {
    pub fn should_use_dynamic_focal_tracking(
        &self,
        target_level: f64,
        locked: bool,
        level_changed: bool,
    ) -> bool {
        level_changed
            && !locked
            && target_level > 1.0
            && self.cursor_pos.is_some()
            && self.output_size.is_some()
            && self.movement_mode.is_some()
    }

    pub fn compute_focal(&self, level: f64, fallback: Point<f64, Local>) -> Point<f64, Local> {
        let (Some(cursor), Some(size), Some(mode)) = (
            self.cursor_pos,
            self.output_size,
            self.movement_mode.as_ref(),
        ) else {
            return fallback;
        };

        if matches!(mode, ZoomMovementMode::OnEdge) {
            if let Some(anchor) = self.on_edge_cursor_anchor {
                return Self::focal_for_on_edge_anchor(cursor, level, size, anchor);
            }
        }

        Self::focal_for_cursor(cursor, level, size, mode)
    }

    /// Computes the focal point that places `cursor` within the viewport at
    /// the given zoom level and movement mode.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn focal_for_cursor(
        cursor: Point<f64, Local>,
        level: f64,
        output_size: Size<f64, Local>,
        mode: &ZoomMovementMode,
    ) -> Point<f64, Local> {
        if level <= 1.0 {
            return cursor;
        }

        match mode {
            ZoomMovementMode::CursorFollow => cursor,
            // OnEdge uses the same static focal as Centered; it only differs
            // in how it updates the focal as the cursor moves relative to the
            // viewport.
            ZoomMovementMode::Centered | ZoomMovementMode::OnEdge => {
                let vt = ViewportTransform::new(cursor, level);
                let output_rect = Rectangle::from_size(output_size);
                let viewport = vt.apply_inverse_rect(output_rect);
                let scale_factor = level / (level - 1.0).max(0.001);

                viewport
                    .loc
                    .upscale(scale_factor)
                    .constrain(Rectangle::from_size(
                        output_size - Size::from((f64::EPSILON, f64::EPSILON)),
                    ))
            }
        }
    }

    /// Computes the focal point from an OnEdge cursor anchor, restoring the
    /// cursor's relative viewport position after a zoom level change.
    fn focal_for_on_edge_anchor(
        cursor: Point<f64, Local>,
        level: f64,
        output_size: Size<f64, Local>,
        anchor: Point<f64, Local>,
    ) -> Point<f64, Local> {
        if level <= 1.0 {
            return cursor;
        }

        let viewport_size = output_size.downscale(level);
        let anchor_offset = Point::from((viewport_size.w * anchor.x, viewport_size.h * anchor.y));
        let viewport_loc: Point<f64, Local> = cursor - anchor_offset;
        let scale_factor = level / (level - 1.0).max(0.001);

        viewport_loc
            .upscale(scale_factor)
            .constrain(Rectangle::from_size(
                output_size - Size::from((f64::EPSILON, f64::EPSILON)),
            ))
    }

    pub fn compute_on_edge_anchor(
        &self,
        current_level: f64,
        current_focal: Point<f64, Local>,
    ) -> Option<Point<f64, Local>> {
        let (Some(cursor), Some(output_size)) = (self.cursor_pos, self.output_size) else {
            return None;
        };

        if !matches!(self.movement_mode.as_ref(), Some(ZoomMovementMode::OnEdge)) {
            return None;
        }

        // Compute cursor anchor ratio within the viewport via ViewportTransform.
        let vt = ViewportTransform::new(current_focal, current_level);
        let output_rect = Rectangle::from_size(output_size);
        let viewport = vt.apply_inverse_rect(output_rect);
        let constrained = cursor.constrain(Rectangle::new(
            viewport.loc,
            viewport.size - Size::from((f64::EPSILON, f64::EPSILON)),
        ));
        let delta = constrained - viewport.loc;
        let anchor_x = if viewport.size.w.abs() < f64::EPSILON {
            0.5
        } else {
            delta.x / viewport.size.w
        };
        let anchor_y = if viewport.size.h.abs() < f64::EPSILON {
            0.5
        } else {
            delta.y / viewport.size.h
        };
        Some((anchor_x, anchor_y).into())
    }

    pub fn set_cursor_pos(&mut self, pos: Point<f64, Local>) {
        self.cursor_pos = Some(pos);
    }

    pub fn set_output_size(&mut self, size: Size<f64, Local>) {
        self.output_size = Some(size);
    }

    /// Update the movement mode. If the new mode is OnEdge, the cursor anchor
    /// is recomputed from the current cursor/level/focal so subsequent
    /// `compute_focal()` calls use the new mode's logic.
    ///
    /// `current_level` and `current_focal` are the zoom state at the time of
    /// the mode change — used to compute the OnEdge anchor.
    pub fn set_movement_mode(
        &mut self,
        mode: ZoomMovementMode,
        current_level: f64,
        current_focal: Point<f64, Local>,
    ) {
        self.movement_mode = Some(mode);
        self.on_edge_cursor_anchor = self.compute_on_edge_anchor(current_level, current_focal);
    }
}

#[derive(Debug, Clone)]
pub struct ZoomLevelAnimation {
    pub(super) anim: Animation,
    pub(super) tracking: FocalTrackingContext,
}

impl ZoomLevelAnimation {
    pub fn new(clock: Clock, from: f64, to: f64, config: niri_config::Animation) -> Self {
        Self {
            anim: Animation::new(clock, from, to, 0.0, config),
            tracking: FocalTrackingContext::default(),
        }
    }

    pub fn with_tracking_context(
        mut self,
        cursor_pos: Option<Point<f64, Local>>,
        output_size: Option<Size<f64, Local>>,
        movement_mode: Option<ZoomMovementMode>,
        current_level: f64,
        current_focal: Point<f64, Local>,
    ) -> Self {
        self.tracking.cursor_pos = cursor_pos;
        self.tracking.output_size = output_size;
        self.tracking.movement_mode = movement_mode;
        self.tracking.on_edge_cursor_anchor = self
            .tracking
            .compute_on_edge_anchor(current_level, current_focal);
        self
    }

    pub fn set_cursor_pos(&mut self, pos: Point<f64, Local>) {
        self.tracking.set_cursor_pos(pos);
    }

    pub fn set_movement_mode(
        &mut self,
        mode: ZoomMovementMode,
        current_level: f64,
        current_focal: Point<f64, Local>,
    ) {
        self.tracking
            .set_movement_mode(mode, current_level, current_focal);
    }

    pub fn should_use_dynamic_focal_tracking(
        &self,
        target_level: f64,
        locked: bool,
        level_changed: bool,
    ) -> bool {
        self.tracking
            .should_use_dynamic_focal_tracking(target_level, locked, level_changed)
    }

    pub fn value_at(&self, now: Duration) -> f64 {
        self.anim.value_at(now)
    }
}

#[derive(Debug, Clone)]
pub struct ZoomLevelGesture {
    pub tracker: SwipeTracker,
    pub start_level: f64,
    pub current_level: f64,
    pub current_focal: Point<f64, Local>,
    pub last_log_scale: Option<f64>,
    tracking: FocalTrackingContext,
}

impl ZoomLevelGesture {
    pub fn new(
        start_level: f64,
        current_focal: Point<f64, Local>,
        cursor_pos: Option<Point<f64, Local>>,
        output_size: Option<Size<f64, Local>>,
        movement_mode: Option<ZoomMovementMode>,
    ) -> Self {
        let mut result = Self {
            tracker: SwipeTracker::new(),
            start_level,
            current_level: start_level,
            current_focal,
            last_log_scale: None,
            tracking: FocalTrackingContext {
                cursor_pos,
                output_size,
                movement_mode,
                on_edge_cursor_anchor: None,
            },
        };
        result.tracking.on_edge_cursor_anchor = result
            .tracking
            .compute_on_edge_anchor(start_level, current_focal);
        result
    }

    pub fn compute_focal_or(&self, level: f64, fallback: Point<f64, Local>) -> Point<f64, Local> {
        self.tracking.compute_focal(level, fallback)
    }

    pub fn cursor_pos(&self) -> Option<Point<f64, Local>> {
        self.tracking.cursor_pos
    }

    pub fn output_size(&self) -> Option<Size<f64, Local>> {
        self.tracking.output_size
    }

    pub fn movement_mode(&self) -> Option<&ZoomMovementMode> {
        self.tracking.movement_mode.as_ref()
    }

    pub fn set_cursor_pos(&mut self, pos: Point<f64, Local>) {
        self.tracking.set_cursor_pos(pos);
    }

    pub fn set_output_size(&mut self, size: Size<f64, Local>) {
        self.tracking.output_size = Some(size);
    }

    pub fn set_movement_mode(
        &mut self,
        mode: ZoomMovementMode,
        current_level: f64,
        current_focal: Point<f64, Local>,
    ) {
        self.tracking
            .set_movement_mode(mode, current_level, current_focal);
    }

    pub fn should_use_dynamic_focal_tracking(
        &self,
        target_level: f64,
        locked: bool,
        level_changed: bool,
    ) -> bool {
        self.tracking
            .should_use_dynamic_focal_tracking(target_level, locked, level_changed)
    }
}

#[derive(Debug, Clone)]
pub struct ZoomFocalAnimation {
    pub x_anim: Animation,
    pub y_anim: Animation,
}

impl ZoomFocalAnimation {
    pub fn new(
        clock: Clock,
        from: Point<f64, Local>,
        to: Point<f64, Local>,
        config: niri_config::Animation,
    ) -> Self {
        Self {
            x_anim: Animation::new(clock.clone(), from.x, to.x, 0.0, config),
            y_anim: Animation::new(clock, from.y, to.y, 0.0, config),
        }
    }

    pub fn value_at(&self, now: Duration) -> Point<f64, Local> {
        Point::from((self.x_anim.value_at(now), self.y_anim.value_at(now)))
    }
}

/// Level transition: idle, scripted animation, or user-driven gesture.
#[derive(Debug, Clone, Default)]
pub enum ZoomLevelTransition {
    #[default]
    Idle,
    Animating(ZoomLevelAnimation),
    Gesturing(ZoomLevelGesture),
}

impl ZoomLevelTransition {
    /// Clear completed `Animating` transitions.
    pub fn sweep_at(&mut self, _now: Duration) {
        if let Self::Animating(a) = self {
            if a.anim.is_done() {
                *self = Self::Idle;
            }
        }
    }

    pub fn take_gesture(&mut self) -> Option<ZoomLevelGesture> {
        match std::mem::take(self) {
            Self::Gesturing(g) => Some(g),
            other => {
                *self = other;
                None
            }
        }
    }

    pub fn gesture_mut(&mut self) -> Option<&mut ZoomLevelGesture> {
        match self {
            Self::Gesturing(g) => Some(g),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use niri_config::animations::{Animation as AnimationConfig, Curve, EasingParams, Kind};
    use niri_config::ZoomMovementMode;
    use smithay::output::{Mode, PhysicalProperties, Subpixel};
    use smithay::utils::{Point, Rectangle, Scale, Size, Transform};

    use super::*;
    use crate::utils::geometry::Local;

    fn output() -> Output {
        let output = Output::new(
            "test".to_string(),
            PhysicalProperties {
                size: Size::from((1920, 1080)),
                subpixel: Subpixel::Unknown,
                make: String::new(),
                model: String::new(),
                serial_number: String::new(),
            },
        );
        output.change_current_state(
            Some(Mode {
                size: Size::from((1920, 1080)),
                refresh: 60000,
            }),
            None,
            None,
            None,
        );
        output
    }

    fn state(level: f64, focal: Point<f64, Local>) -> OutputZoomState {
        OutputZoomState {
            level,
            focal,
            locked: false,
            level_transition: ZoomLevelTransition::Idle,
            focal_animation: None,
        }
    }

    fn animation_config() -> AnimationConfig {
        AnimationConfig {
            off: false,
            kind: Kind::Easing(EasingParams {
                duration_ms: 100,
                curve: Curve::Linear,
            }),
        }
    }

    #[test]
    fn new_for_output_starts_at_local_center() {
        let state = OutputZoomState::new_for_output(&output());

        assert_eq!(state.level, 1.0);
        assert_eq!(state.focal, (960.0, 540.0).into());
        assert!(!state.locked);
        assert!(!state.transitioning());
    }

    #[test]
    fn idle_viewport_transform_uses_resting_state() {
        let state = state(2.5, (300.0, 200.0).into());

        assert_eq!(
            state.viewport_transform(Duration::from_secs(1)),
            ViewportTransform::new((300.0, 200.0).into(), 2.5)
        );
    }

    #[test]
    fn gesture_viewport_transform_uses_live_values() {
        let mut gesture = ZoomLevelGesture::new(
            1.0,
            (960.0, 540.0).into(),
            Some((200.0, 100.0).into()),
            Some((1920.0, 1080.0).into()),
            Some(ZoomMovementMode::CursorFollow),
        );
        gesture.current_level = 3.0;
        gesture.current_focal = (200.0, 100.0).into();

        let state = OutputZoomState {
            level: 1.0,
            focal: (960.0, 540.0).into(),
            locked: false,
            level_transition: ZoomLevelTransition::Gesturing(gesture),
            focal_animation: None,
        };
        let viewport = state.viewport_transform(Duration::ZERO);

        assert_eq!(viewport.factor(), 3.0);
        assert_eq!(viewport.focal(), (200.0, 100.0).into());
    }

    #[test]
    fn gesture_is_transitioning_but_not_animating() {
        let gesture = ZoomLevelGesture::new(1.0, (960.0, 540.0).into(), None, None, None);
        let state = OutputZoomState {
            level: 1.0,
            focal: (960.0, 540.0).into(),
            locked: false,
            level_transition: ZoomLevelTransition::Gesturing(gesture),
            focal_animation: None,
        };

        assert!(state.transitioning());
        assert!(!state.is_animating());
    }

    #[test]
    fn focal_tracking_requires_complete_context() {
        let mut tracking = FocalTrackingContext::default();
        assert!(!tracking.should_use_dynamic_focal_tracking(2.0, false, true));

        tracking.set_cursor_pos((100.0, 100.0).into());
        tracking.set_output_size((1920.0, 1080.0).into());
        assert!(!tracking.should_use_dynamic_focal_tracking(2.0, false, true));

        tracking.set_movement_mode(ZoomMovementMode::CursorFollow, 1.0, (960.0, 540.0).into());
        assert!(tracking.should_use_dynamic_focal_tracking(2.0, false, true));
        assert!(!tracking.should_use_dynamic_focal_tracking(1.0, false, true));
        assert!(!tracking.should_use_dynamic_focal_tracking(2.0, true, true));
        assert!(!tracking.should_use_dynamic_focal_tracking(2.0, false, false));
    }

    #[test]
    fn focal_tracking_falls_back_without_cursor_context() {
        let tracking = FocalTrackingContext::default();
        let fallback: Point<f64, Local> = (321.0, 123.0).into();

        assert_eq!(tracking.compute_focal(2.0, fallback), fallback);
    }

    #[test]
    fn focal_for_cursor_is_cursor_follow_at_zoom() {
        let cursor = (321.0, 123.0).into();
        let size = (1920.0, 1080.0).into();

        assert_eq!(
            FocalTrackingContext::focal_for_cursor(
                cursor,
                2.0,
                size,
                &ZoomMovementMode::CursorFollow,
            ),
            cursor
        );
        assert_eq!(
            FocalTrackingContext::focal_for_cursor(
                cursor,
                1.0,
                size,
                &ZoomMovementMode::CursorFollow,
            ),
            cursor
        );
    }

    #[test]
    fn on_edge_anchor_is_only_present_for_on_edge_mode() {
        let mut tracking = FocalTrackingContext::default();
        tracking.set_cursor_pos((500.0, 400.0).into());
        tracking.set_output_size((1920.0, 1080.0).into());

        tracking.set_movement_mode(ZoomMovementMode::Centered, 2.0, (500.0, 400.0).into());
        assert_eq!(
            tracking.compute_on_edge_anchor(2.0, (500.0, 400.0).into()),
            None
        );

        tracking.set_movement_mode(ZoomMovementMode::OnEdge, 2.0, (500.0, 400.0).into());
        let anchor = tracking.compute_on_edge_anchor(2.0, (500.0, 400.0).into());
        assert!(anchor.is_some());
        let anchor = anchor.unwrap();
        assert!((0.0..=1.0).contains(&anchor.x));
        assert!((0.0..=1.0).contains(&anchor.y));
    }

    #[test]
    fn changing_movement_mode_recomputes_on_edge_tracking() {
        let mut tracking = FocalTrackingContext::default();
        tracking.set_cursor_pos((10.0, 20.0).into());
        tracking.set_output_size((1920.0, 1080.0).into());
        tracking.set_movement_mode(ZoomMovementMode::CursorFollow, 1.0, (960.0, 540.0).into());
        tracking.set_movement_mode(ZoomMovementMode::OnEdge, 2.0, (960.0, 540.0).into());

        let focal = tracking.compute_focal(2.0, (960.0, 540.0).into());
        assert!((0.0..=1920.0).contains(&focal.x));
        assert!((0.0..=1080.0).contains(&focal.y));
    }

    #[test]
    fn completed_level_animation_is_swept_to_idle() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut clock_for_animation = clock.clone();
        clock_for_animation.set_complete_instantly(true);
        let animation = ZoomLevelAnimation::new(clock, 1.0, 3.0, animation_config());
        let mut transition = ZoomLevelTransition::Animating(animation);

        transition.sweep_at(Duration::ZERO);

        assert!(matches!(transition, ZoomLevelTransition::Idle));
    }

    #[test]
    fn advance_animations_commits_level_and_focal_targets() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut clock_for_animation = clock.clone();
        clock_for_animation.set_complete_instantly(true);
        let level_animation = ZoomLevelAnimation::new(clock.clone(), 1.0, 3.0, animation_config());
        let focal_animation = ZoomFocalAnimation::new(
            clock,
            (960.0, 540.0).into(),
            (300.0, 200.0).into(),
            animation_config(),
        );
        let mut state = OutputZoomState {
            level: 1.0,
            focal: (960.0, 540.0).into(),
            locked: false,
            level_transition: ZoomLevelTransition::Animating(level_animation),
            focal_animation: Some(focal_animation),
        };

        state.advance_animations(Duration::from_millis(1));

        assert_eq!(state.level, 3.0);
        assert_eq!(state.focal, (300.0, 200.0).into());
        assert!(!state.transitioning());
    }

    #[test]
    fn viewport_global_translates_local_viewport_to_output_origin() {
        let state = state(2.0, (960.0, 540.0).into());
        let ctx = OutputViewCtx::new(
            Rectangle::new((100.0, 200.0).into(), (1920.0, 1080.0).into()),
            Rectangle::from_size((1920.0, 1080.0).into()),
            Transform::Normal,
            Scale::from(1.0),
        );

        let viewport = state.viewport_global(&ctx, Duration::ZERO);

        assert_eq!(viewport.loc, (580.0, 470.0).into());
        assert_eq!(viewport.size, (960.0, 540.0).into());
    }
}
