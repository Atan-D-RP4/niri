use std::time::Duration;

use niri_config::animations::{ZoomFocalPanAnim, ZoomLevelChangeAnim};
use niri_config::{Zoom, ZoomMovementMode};
use smithay::output::Output;
use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::animation::{Animation, Clock};
use crate::utils::output_size;
use crate::utils::view::OutputViewport;

const ZOOM_CHANGE_EPSILON: f64 = 1e-4;

#[derive(Debug)]
pub struct OutputZoomState {
    level: f64,
    focal: Point<f64, Logical>,
    pub locked: bool,

    // The maximum zoom level, clamped to this value.
    pub max_level: f64,
    pub movement_mode: ZoomMovementMode,
    // Content-local cursor anchor, pointer-only: touch never writes zoom state.
    cursor: Point<f64, Logical>,
    output_size: Size<f64, Logical>,

    clock: Clock,
    level_transition: Option<Animation>,
    focal_transition: Option<PointAnimation>,
}

impl OutputZoomState {
    pub fn new(output: &Output, clock: Clock, zoom: Zoom) -> Self {
        let output_size = output_size(output);
        let output_center = Point::from((output_size.w / 2., output_size.h / 2.));
        Self {
            level: 1.0,
            focal: output_center,
            locked: false,
            movement_mode: zoom.movement_mode,
            // Config default, can be overridden by the user in config.
            max_level: zoom.max_zoom,
            cursor: output_center,
            output_size,
            clock,
            level_transition: None,
            focal_transition: None,
        }
    }

    pub fn view(&self) -> OutputViewport {
        OutputViewport::new(self.focal, self.level)
    }

    pub fn advance_level_transition(&mut self) {
        let Some(ref anim) = self.level_transition else {
            return;
        };
        let new_level = if anim.is_done() {
            let level = anim.to();
            self.level_transition = None;
            level
        } else {
            anim.value_at(self.clock.now())
        };
        // Compute anchor at old level, then apply new level with the same anchor to avoid
        // drift.
        let anchor = self.tracked_anchor();
        self.level = new_level;
        self.apply_focal(self.focal_for(anchor));
    }

    pub fn advance_focal_transition(&mut self) {
        let Some(ref anim) = self.focal_transition else {
            return;
        };
        let new_focal = if anim.is_done() {
            let focal = anim.to;
            self.focal_transition = None;
            focal
        } else {
            anim.value_at(self.clock.now())
        };
        self.apply_focal(new_focal);
    }

    pub fn advance_animations(&mut self) {
        self.advance_level_transition();
        self.advance_focal_transition();
    }

    pub fn are_animations_ongoing(&self) -> bool {
        self.level_transition.is_some() || self.focal_transition.is_some()
    }

    pub fn set_config(
        &mut self,
        zoom: &Zoom,
        level_anim: &ZoomLevelChangeAnim,
        focal_anim: &ZoomFocalPanAnim,
    ) -> bool {
        let old_view = self.view();

        self.max_level = zoom.max_zoom;
        if self.movement_mode != zoom.movement_mode {
            self.set_movement_mode(zoom.movement_mode, *focal_anim);
        } else if self.focal_transition.is_none() {
            let anchor = self.tracked_anchor();
            self.apply_focal(self.focal_for(anchor));
        }
        self.set_level(self.level, *level_anim);

        old_view != self.view() || self.are_animations_ongoing()
    }

    pub fn set_output_size(&mut self, size: Size<f64, Logical>) {
        if self.output_size == size {
            return;
        }
        self.output_size = size;
        if self.focal_transition.is_none() {
            let anchor = self.tracked_anchor();
            self.apply_focal(self.focal_for(anchor));
        }
    }

    pub fn set_level(&mut self, level: f64, config: ZoomLevelChangeAnim) {
        let target_level = level.clamp(1.0, self.max_level);
        if self.locked || (self.level - target_level).abs() <= ZOOM_CHANGE_EPSILON {
            return;
        }

        self.focal_transition = None;
        self.level_transition = Some(Animation::new(
            self.clock.clone(),
            self.level,
            target_level,
            0.,
            config.0,
        ));
    }

    pub fn set_cursor(&mut self, pos: Point<f64, Logical>) {
        self.cursor = pos;
        if self.level <= 1.0 || self.focal_transition.is_some() {
            return;
        }
        let anchor = self.tracked_anchor();
        self.apply_focal(self.focal_for(anchor));
    }

    pub fn set_movement_mode(&mut self, mode: ZoomMovementMode, config: ZoomFocalPanAnim) {
        if self.movement_mode == mode {
            return;
        }

        self.movement_mode = mode;
        self.focal_transition = None;

        if self.locked || self.level_transition.is_some() {
            return;
        }

        let target = self.focal_for(self.tracked_anchor());
        let delta = target - self.focal;
        if delta.x.abs() < ZOOM_CHANGE_EPSILON && delta.y.abs() < ZOOM_CHANGE_EPSILON {
            return;
        }

        self.focal_transition = Some(PointAnimation::new(
            self.clock.clone(),
            self.focal,
            target,
            config.0,
        ));
    }

    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
        if self.locked {
            self.focal_transition = None;
        } else {
            // A locked zoom keeps its level when a config change lowers the max zoom;
            // re-clamp it before the zoom resumes.
            self.level = self.level.clamp(1.0, self.max_level);
            let anchor = self.tracked_anchor();
            self.apply_focal(self.focal_for(anchor));
        }
    }

    fn apply_focal(&mut self, focal: Point<f64, Logical>) {
        if self.locked {
            return;
        }

        let focal_delta = focal - self.focal;
        if focal_delta.x.abs() < ZOOM_CHANGE_EPSILON && focal_delta.y.abs() < ZOOM_CHANGE_EPSILON {
            return;
        }

        self.focal = focal;
    }

    fn tracked_anchor(&self) -> Option<Point<f64, Logical>> {
        if !matches!(self.movement_mode, ZoomMovementMode::OnEdge) {
            return None;
        }

        let viewport =
            OutputViewport::new(self.focal, self.level).visible_viewport(self.output_size);
        let constrained = self.cursor.constrain(viewport);
        let delta = constrained - viewport.loc;
        Some(Point::from((
            delta.x / viewport.size.w,
            delta.y / viewport.size.h,
        )))
    }

    fn focal_for(&self, anchor: Option<Point<f64, Logical>>) -> Point<f64, Logical> {
        let anchor = if matches!(self.movement_mode, ZoomMovementMode::OnEdge) {
            anchor.unwrap_or_else(|| Point::from((0.5, 0.5)))
        } else {
            Point::from((0.5, 0.5))
        };

        let output_size = self.output_size;

        match self.movement_mode {
            ZoomMovementMode::CursorFollow => self.cursor,
            ZoomMovementMode::Centered | ZoomMovementMode::OnEdge => {
                if self.level <= 1.0 {
                    return self.cursor;
                }

                let viewport = OutputViewport::new(self.cursor, self.level);
                let viewport_size = viewport
                    .screen_to_content_rect(Rectangle::from_size(output_size))
                    .size;
                let anchor_offset =
                    Point::from((viewport_size.w * anchor.x, viewport_size.h * anchor.y));
                let viewport_loc = self.cursor - anchor_offset;
                let scale_factor = self.level / (self.level - 1.0).max(0.0001);

                viewport_loc
                    .upscale(scale_factor)
                    .constrain(Rectangle::from_size(output_size))
            }
        }
    }

    #[cfg(test)]
    fn verify_invariants(&self) {
        // The level is only ever assigned from set_level() (clamped to [1, max_level]) or
        // interpolated between those endpoints.
        assert!(self.level >= 1.0, "level must be at least 1");

        // While a level transition runs, the level stays between its endpoints; otherwise
        // it keeps its last value. A locked zoom may sit above the max level: a config
        // change cannot move a frozen level, and unlocking re-clamps it.
        if let Some(anim) = &self.level_transition {
            let (from, to) = (anim.from(), anim.to());
            assert!(
                (from.min(to)..=from.max(to)).contains(&self.level),
                "level {} is not between the level transition endpoints [{from}, {to}]",
                self.level,
            );
        } else {
            assert!(
                self.level <= self.max_level || self.locked,
                "level {} exceeds max_level {} while the zoom is unlocked",
                self.level,
                self.max_level,
            );
        }

        // Starting a level transition cancels a pending focal pan, and a focal pan is not
        // started while a level transition is running.
        assert!(
            !(self.level_transition.is_some() && self.focal_transition.is_some()),
            "level and focal transitions cannot be active at the same time",
        );

        // While a focal pan runs, the focal stays on the segment between its endpoints.
        if let Some(anim) = &self.focal_transition {
            for (value, from, to) in [
                (self.focal.x, anim.from.x, anim.to.x),
                (self.focal.y, anim.from.y, anim.to.y),
            ] {
                assert!(
                    (from.min(to)..=from.max(to)).contains(&value),
                    "focal coordinate {value} is off the pan segment [{from}, {to}]",
                );
            }
        }

        // The focal point always stays on the output, including at its edge: focal_for()
        // clamps with constrain(), which allows the edge exactly.
        assert!(
            self.focal.x >= 0.
                && self.focal.x <= self.output_size.w
                && self.focal.y >= 0.
                && self.focal.y <= self.output_size.h,
            "focal {:?} must stay within the output of size {:?}",
            self.focal,
            self.output_size,
        );
    }
}

/// Animates between two points via a single normalized 0 -> 1 progress curve,
/// so both components advance in lockstep from one curve evaluation.
#[derive(Debug, Clone)]
pub struct PointAnimation {
    from: Point<f64, Logical>,
    to: Point<f64, Logical>,
    progress: Animation,
}

impl PointAnimation {
    pub fn new(
        clock: Clock,
        from: Point<f64, Logical>,
        to: Point<f64, Logical>,
        config: niri_config::Animation,
    ) -> Self {
        Self {
            from,
            to,
            progress: Animation::new(clock, 0., 1., 0., config),
        }
    }

    fn value_at(&self, now: Duration) -> Point<f64, Logical> {
        let t = self.progress.value_at(now);
        Point::from((
            self.from.x + (self.to.x - self.from.x) * t,
            self.from.y + (self.to.y - self.from.y) * t,
        ))
    }

    fn is_done(&self) -> bool {
        self.progress.is_done()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use approx::assert_relative_eq;
    use niri_config::animations::{Curve, EasingParams, Kind};
    use niri_config::{Zoom, ZoomMovementMode};
    use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
    use smithay::utils::Size;

    use super::*;

    fn test_output() -> Output {
        let output = Output::new(
            "test".into(),
            PhysicalProperties {
                size: Size::from((1000, 500)),
                subpixel: Subpixel::Unknown,
                make: String::new(),
                model: String::new(),
                serial_number: String::new(),
            },
        );
        output.change_current_state(
            Some(Mode {
                size: Size::from((1000, 500)),
                refresh: 60_000,
            }),
            None,
            None,
            None,
        );
        output
    }

    fn zoom_state(mode: ZoomMovementMode, clock: Clock) -> OutputZoomState {
        let state = OutputZoomState::new(
            &test_output(),
            clock,
            Zoom {
                movement_mode: mode,
                ..Default::default()
            },
        );
        state.verify_invariants();
        state
    }

    fn easing(duration_ms: u32) -> niri_config::Animation {
        niri_config::Animation {
            off: false,
            kind: Kind::Easing(EasingParams {
                duration_ms,
                curve: Curve::Linear,
            }),
        }
    }

    fn anim_off() -> niri_config::Animation {
        niri_config::Animation {
            off: true,
            kind: Kind::Easing(EasingParams {
                duration_ms: 0,
                curve: Curve::Linear,
            }),
        }
    }

    #[test]
    fn level_transition_derives_focal_from_anchor_at_new_level() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::OnEdge, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        assert_relative_eq!(state.level, 2.0);

        state.set_cursor(Point::from((600., 250.)));
        state.verify_invariants();

        state.set_level(3.0, ZoomLevelChangeAnim(easing(100)));
        state.verify_invariants();
        assert!(state.level_transition.is_some());

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();
        state.verify_invariants();

        assert!(state.level_transition.is_none());
        assert_relative_eq!(state.level, 3.0);
        assert_relative_eq!(state.focal.x, 550., epsilon = 1e-6);
        assert_relative_eq!(state.focal.y, 250., epsilon = 1e-6);
    }

    #[test]
    fn focal_pan_progresses_to_target() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::Centered, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        state.set_cursor(Point::from((700., 350.)));
        state.verify_invariants();
        assert_relative_eq!(state.focal.x, 900.);
        assert_relative_eq!(state.focal.y, 450.);

        state.set_movement_mode(
            ZoomMovementMode::CursorFollow,
            ZoomFocalPanAnim(easing(100)),
        );
        state.verify_invariants();
        assert!(state.focal_transition.is_some());
        assert_relative_eq!(state.focal.x, 900.);
        assert_relative_eq!(state.focal.y, 450.);

        state.advance_animations();
        state.verify_invariants();
        assert!(state.focal_transition.is_some());
        assert_relative_eq!(state.focal.x, 900.);
        assert_relative_eq!(state.focal.y, 450.);

        clock.set_unadjusted(Duration::from_millis(50));
        state.advance_animations();
        state.verify_invariants();
        assert!(state.focal_transition.is_some());
        assert_relative_eq!(state.focal.x, 800., epsilon = 1e-6);
        assert_relative_eq!(state.focal.y, 400., epsilon = 1e-6);

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();
        state.verify_invariants();
        assert!(state.focal_transition.is_none());
        assert!(!state.are_animations_ongoing());
        assert_relative_eq!(state.focal.x, 700., epsilon = f64::EPSILON);
        assert_relative_eq!(state.focal.y, 350., epsilon = f64::EPSILON);
    }

    #[test]
    fn mode_change_with_animations_off_commits_target() {
        let mut state = zoom_state(ZoomMovementMode::Centered, Clock::with_time(Duration::ZERO));

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        state.set_cursor(Point::from((700., 350.)));
        state.verify_invariants();

        state.set_movement_mode(ZoomMovementMode::CursorFollow, ZoomFocalPanAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        assert!(state.focal_transition.is_none());
        assert_relative_eq!(state.focal.x, 700., epsilon = f64::EPSILON);
        assert_relative_eq!(state.focal.y, 350., epsilon = f64::EPSILON);

        state.set_cursor(Point::from((900., 100.)));
        state.verify_invariants();
        state.set_movement_mode(ZoomMovementMode::Centered, ZoomFocalPanAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        assert!(state.focal_transition.is_none());
        assert_relative_eq!(state.focal.x, 1000., epsilon = 1e-6);
        assert_relative_eq!(state.focal.y, 0., epsilon = 1e-6);
    }

    #[test]
    fn locked_zoom_freezes_during_pan() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::Centered, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        state.set_cursor(Point::from((700., 350.)));
        state.verify_invariants();

        state.set_movement_mode(
            ZoomMovementMode::CursorFollow,
            ZoomFocalPanAnim(easing(100)),
        );
        state.verify_invariants();
        let frozen = state.view();

        state.toggle_lock();
        state.verify_invariants();
        assert!(!state.are_animations_ongoing());

        state.set_level(3.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        assert_relative_eq!(state.level, 2.0);
        assert!(!state.are_animations_ongoing());

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();
        state.verify_invariants();
        assert_eq!(state.view(), frozen);

        state.set_cursor(Point::from((200., 100.)));
        state.verify_invariants();
        assert_eq!(state.view(), frozen);

        state.set_movement_mode(ZoomMovementMode::Centered, ZoomFocalPanAnim(easing(100)));
        state.verify_invariants();
        assert!(state.focal_transition.is_none());
        assert_eq!(state.view(), frozen);
    }

    #[test]
    fn mode_change_during_level_transition_defers_focal_to_level_transition() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::OnEdge, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        state.set_cursor(Point::from((600., 250.)));
        state.verify_invariants();

        state.set_level(3.0, ZoomLevelChangeAnim(easing(100)));
        state.verify_invariants();
        let view_before = state.view();

        state.set_movement_mode(ZoomMovementMode::Centered, ZoomFocalPanAnim(easing(100)));
        state.verify_invariants();
        assert!(state.level_transition.is_some());
        assert!(state.focal_transition.is_none());
        assert_eq!(state.view(), view_before);

        clock.set_unadjusted(Duration::from_millis(50));
        state.advance_animations();
        state.verify_invariants();

        assert!(state.level_transition.is_some());
        assert!(state.focal_transition.is_none());
        assert_relative_eq!(state.level, 2.5, epsilon = 1e-6);
        assert_relative_eq!(state.focal.x, 2000. / 3., epsilon = 1e-6);
        assert_relative_eq!(state.focal.y, 250., epsilon = 1e-6);
    }

    #[test]
    fn level_change_cancels_focal_pan() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::Centered, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        state.set_cursor(Point::from((700., 350.)));
        state.verify_invariants();

        // CursorFollow starts a real pan: the Centered focal is (900, 450) and the
        // target is the cursor at (700, 350).
        state.set_movement_mode(
            ZoomMovementMode::CursorFollow,
            ZoomFocalPanAnim(easing(100)),
        );
        state.verify_invariants();
        assert!(state.focal_transition.is_some());
        assert_relative_eq!(state.focal.x, 900.);
        assert_relative_eq!(state.focal.y, 450.);

        state.set_level(3.0, ZoomLevelChangeAnim(easing(100)));
        state.verify_invariants();
        assert!(state.level_transition.is_some());
        assert!(state.focal_transition.is_none());
        assert_relative_eq!(state.focal.x, 900.);
        assert_relative_eq!(state.focal.y, 450.);

        // With the pan cancelled, the focal at t=50 is carried by the level transition
        // anchoring alone; a pending pan would have interpolated to (800, 400) here.
        clock.set_unadjusted(Duration::from_millis(50));
        state.advance_animations();
        state.verify_invariants();

        assert!(state.level_transition.is_some());
        assert!(state.focal_transition.is_none());
        assert_relative_eq!(state.level, 2.5, epsilon = 1e-6);
        assert_relative_eq!(state.focal.x, 700., epsilon = 1e-6);
        assert_relative_eq!(state.focal.y, 350., epsilon = 1e-6);

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();
        state.verify_invariants();

        assert!(state.level_transition.is_none());
        assert!(state.focal_transition.is_none());
        assert_relative_eq!(state.level, 3.0);
        assert_relative_eq!(state.focal.x, 700., epsilon = f64::EPSILON);
        assert_relative_eq!(state.focal.y, 350., epsilon = f64::EPSILON);
    }

    #[test]
    fn set_config_reports_scheduled_work() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::Centered, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        state.set_cursor(Point::from((700., 350.)));
        state.verify_invariants();

        let zoom = Zoom {
            movement_mode: ZoomMovementMode::CursorFollow,
            ..Default::default()
        };

        let changed = state.set_config(
            &zoom,
            &ZoomLevelChangeAnim(easing(100)),
            &ZoomFocalPanAnim(easing(100)),
        );
        state.verify_invariants();
        assert!(changed);
        assert!(state.focal_transition.is_some());

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();
        state.verify_invariants();
        assert!(!state.are_animations_ongoing());

        let changed = state.set_config(
            &zoom,
            &ZoomLevelChangeAnim(easing(100)),
            &ZoomFocalPanAnim(easing(100)),
        );
        state.verify_invariants();
        assert!(!changed);
    }

    #[test]
    fn set_level_clamps_to_max_level() {
        let mut state = OutputZoomState::new(
            &test_output(),
            Clock::with_time(Duration::ZERO),
            Zoom {
                movement_mode: ZoomMovementMode::Centered,
                max_zoom: 4.0,
                ..Default::default()
            },
        );

        state.set_level(100.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        assert_relative_eq!(state.level, 4.0);

        state.set_level(0.5, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        assert_relative_eq!(state.level, 1.0);
    }

    #[test]
    fn unlock_reclamps_level_after_max_zoom_decrease() {
        let mut state = OutputZoomState::new(
            &test_output(),
            Clock::with_time(Duration::ZERO),
            Zoom {
                movement_mode: ZoomMovementMode::Centered,
                max_zoom: 10.0,
                ..Default::default()
            },
        );
        state.verify_invariants();

        // Zoom in and lock: the frozen level survives config changes.
        state.set_level(8.0, ZoomLevelChangeAnim(anim_off()));
        state.verify_invariants();
        state.advance_animations();
        state.verify_invariants();
        assert_relative_eq!(state.level, 8.0);
        state.toggle_lock();
        state.verify_invariants();

        // Lowering the max zoom cannot move the locked level.
        let zoom = Zoom {
            movement_mode: ZoomMovementMode::Centered,
            max_zoom: 5.0,
            ..Default::default()
        };
        let changed = state.set_config(
            &zoom,
            &ZoomLevelChangeAnim(anim_off()),
            &ZoomFocalPanAnim(anim_off()),
        );
        state.verify_invariants();
        assert!(!changed);
        assert_relative_eq!(state.level, 8.0);

        // Unlocking re-clamps the level to the new max before the zoom resumes.
        state.toggle_lock();
        state.verify_invariants();
        assert_relative_eq!(state.level, 5.0);
        assert!(!state.are_animations_ongoing());
        assert_relative_eq!(state.focal.x, 500., epsilon = 1e-6);
        assert_relative_eq!(state.focal.y, 250., epsilon = 1e-6);
    }
}
