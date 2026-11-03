use niri_config::ZoomMovementMode;
use smithay::output::Output;
use smithay::utils::{Logical, Point, Rectangle, Size};

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
}

impl OutputZoomState {
    pub fn new(output: &Output, zoom: niri_config::Zoom) -> Self {
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
        }
    }

    pub fn set_config(&mut self, zoom: &niri_config::Zoom) -> bool {
        let old_view = self.view();

        self.max_level = zoom.max_zoom;
        if self.movement_mode != zoom.movement_mode {
            self.set_movement_mode(zoom.movement_mode);
        } else {
            self.apply_focal(self.focal_for(
                self.level,
                self.cursor,
                self.tracked_anchor(self.focal, self.cursor, self.level),
            ));
        }
        self.set_level(self.level);

        old_view != self.view()
    }

    pub fn view(&self) -> OutputViewport {
        OutputViewport::new(self.focal, self.level)
    }

    pub fn set_output_size(&mut self, size: Size<f64, Logical>) {
        if self.output_size == size {
            return;
        }
        self.output_size = size;
        self.apply_focal(self.focal_for(
            self.level,
            self.cursor,
            self.tracked_anchor(self.focal, self.cursor, self.level),
        ));
    }

    pub fn set_level(&mut self, level: f64) {
        let target_level = level.clamp(1.0, self.max_level);
        if self.locked || (self.level - target_level).abs() <= ZOOM_CHANGE_EPSILON {
            return;
        }
        let anchor = self.tracked_anchor(self.focal, self.cursor, self.level);
        self.level = target_level;
        self.apply_focal(self.focal_for(self.level, self.cursor, anchor));
    }

    pub fn set_cursor(&mut self, pos: Point<f64, Logical>) {
        self.cursor = pos;
        if self.level <= 1.0 {
            return;
        }
        let anchor = self.tracked_anchor(self.focal, pos, self.level);
        let focal = self.focal_for(self.level, pos, anchor);
        self.apply_focal(focal);
    }

    pub fn set_movement_mode(&mut self, mode: ZoomMovementMode) {
        self.movement_mode = mode;
        let anchor = self.tracked_anchor(self.focal, self.cursor, self.level);
        self.apply_focal(self.focal_for(self.level, self.cursor, anchor));
    }

    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
        if !self.locked {
            let anchor = self.tracked_anchor(self.focal, self.cursor, self.level);
            self.apply_focal(self.focal_for(self.level, self.cursor, anchor));
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

    fn tracked_anchor(
        &self,
        focal: Point<f64, Logical>,
        cursor: Point<f64, Logical>,
        level: f64,
    ) -> Option<Point<f64, Logical>> {
        if !matches!(self.movement_mode, ZoomMovementMode::OnEdge) {
            return None;
        }

        let view = OutputViewport::new(focal, level);
        let viewport = view.visible_viewport(self.output_size);
        let constrained = cursor.constrain(viewport);
        let delta = constrained - viewport.loc;
        Some(Point::from((
            delta.x / viewport.size.w,
            delta.y / viewport.size.h,
        )))
    }

    fn focal_for(
        &self,
        level: f64,
        cursor: Point<f64, Logical>,
        anchor: Option<Point<f64, Logical>>,
    ) -> Point<f64, Logical> {
        let anchor = if matches!(self.movement_mode, ZoomMovementMode::OnEdge) {
            if let Some(anchor) = anchor {
                anchor
            } else {
                Point::from((0.5, 0.5))
            }
        } else {
            Point::from((0.5, 0.5))
        };

        match self.movement_mode {
            ZoomMovementMode::CursorFollow => cursor,
            ZoomMovementMode::Centered | ZoomMovementMode::OnEdge => {
                Self::focal_for_anchor(level, self.output_size, cursor, anchor)
            }
        }
    }

    fn focal_for_anchor(
        level: f64,
        output_size: Size<f64, Logical>,
        cursor: Point<f64, Logical>,
        anchor: Point<f64, Logical>,
    ) -> Point<f64, Logical> {
        if level <= 1.0 {
            return cursor;
        }

        let viewport = OutputViewport::new(cursor, level);
        let viewport_size = viewport
            .screen_to_content_rect(Rectangle::from_size(output_size))
            .size;
        let anchor_offset = Point::from((viewport_size.w * anchor.x, viewport_size.h * anchor.y));
        let viewport_loc = cursor - anchor_offset;
        let scale_factor = level / (level - 1.0).max(0.0001);

        viewport_loc
            .upscale(scale_factor)
            .constrain(Rectangle::from_size(output_size))
    }
}
