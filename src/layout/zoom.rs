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
}
