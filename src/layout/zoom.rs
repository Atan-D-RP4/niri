use std::time::Duration;

use niri_config::animations::{ZoomFocalPanAnim, ZoomLevelChangeAnim};
use niri_config::{Zoom, ZoomMovementMode};
use smithay::output::Output;
use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::animation::{Animation, Clock};
use crate::niri::OutputState;
use crate::rubber_band::RubberBand;
use crate::utils::output_size;
use crate::utils::view::OutputViewport;

const ZOOM_CHANGE_EPSILON: f64 = 1e-4;
const ZOOM_ONEDGE_FAR_ANCHOR_INSET: f64 = 1e-4;

#[derive(Debug)]
enum ZoomTransition {
    Level(LevelAnimation),
    Pan(FocalPan),
    Gesture(ZoomGesture),
    GestureReturn(GestureReturn),
}

impl ZoomTransition {
    fn level_animation(&self) -> Option<&Animation> {
        match self {
            Self::Level(level) => Some(&level.animation),
            Self::GestureReturn(gesture) => Some(&gesture.animation),
            _ => None,
        }
    }
}

/// Ordinary level animation owns tracking; an independent pan cannot coexist with it.
#[derive(Debug)]
struct LevelAnimation {
    animation: Animation,
    tracking: TrackedCorrection,
}

/// Active cumulative pinch input. Unlike workspace swipes, scale is supplied cumulatively
/// and release has no fling, so neither a SwipeTracker nor another animation is needed here.
#[derive(Debug)]
struct ZoomGesture {
    start_level: f64,
    focal: GestureFocal,
}

impl OutputState {
    pub fn zoom(&self) -> &OutputZoomState {
        &self.zoom
    }

    /// The obligatory compositor boundary for all zoom actions, including deferred callbacks.
    pub(crate) fn update_zoom(&mut self, action: ZoomAction) -> bool {
        let old_view = self.zoom.view();
        let old_size = self.zoom.output_size;
        let resize = matches!(&action, ZoomAction::Resize(_));
        let (result, policy) = self.zoom.update(action);
        if resize || old_size != self.zoom.output_size {
            // Geometry changes are outside the stable-output contract.
            self.pending_zoom_pointer = None;
        } else if old_view != self.zoom.view() {
            if policy == PointerEffect::PreserveScreen {
                self.pending_zoom_pointer.get_or_insert(old_view);
            } else {
                self.pending_zoom_pointer = None;
            }
        }
        result
    }
}

impl ZoomGesture {
    /// Input supplies cumulative scale, so every sample is a fresh, nonmutating computation.
    fn sample_level(&self, scale: f64, sensitivity: f64, max_level: f64) -> Option<f64> {
        if !scale.is_finite() || scale <= 0. || !sensitivity.is_finite() || sensitivity < 0. {
            return None;
        }
        let log = sensitivity * scale.ln();
        if !log.is_finite() {
            return None;
        }
        let level = RubberBand {
            stiffness: 0.5,
            limit: 0.05,
        }
        .clamp(0., max_level.ln(), self.start_level.ln() + log)
        .exp()
        .max(1.);
        if !level.is_finite() {
            return None;
        }
        Some(level)
    }
}

/// Cancellation or rubber-band settling continues to use the movement mode's focal policy.
#[derive(Debug)]
struct GestureReturn {
    animation: Animation,
    focal: GestureFocal,
}

/// Movement-mode anchor plus an independent touchscreen translation.
#[derive(Debug, Clone, Copy)]
struct GestureFocal {
    cursor_anchor: Point<f64, Logical>,
    continuity_offset: Point<f64, Logical>,
    screen_pan: Point<f64, Logical>,
}

impl GestureFocal {
    fn reset_for_resize(&mut self, bounds: Rectangle<f64, Logical>) {
        self.cursor_anchor = self.cursor_anchor.constrain(bounds);
        self.continuity_offset = Point::from((0., 0.));
        self.screen_pan = Point::from((0., 0.));
    }
}

/// Independent focal pan; its pointer policy is resolved when the pan is created.
#[derive(Debug)]
struct FocalPan {
    animation: PointAnimation,
    pointer_effect: PointerEffect,
}

/// Every displayed-geometry commit declares whether the pointer stays at its screen position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointerEffect {
    KeepContent,
    PreserveScreen,
}

/// Closed, ephemeral requests at the compositor/zoom boundary.
pub(crate) enum ZoomAction {
    Advance,
    Resize(Size<f64, Logical>),
    Configure(Zoom, ZoomLevelChangeAnim, ZoomFocalPanAnim),
    PointerMove(Point<f64, Logical>),
    PointerHandoff(Point<f64, Logical>, ZoomLevelChangeAnim),
    SetLevel(f64, ZoomLevelChangeAnim),
    ToggleLock(ZoomLevelChangeAnim, ZoomFocalPanAnim),
    BeginGesture,
    UpdateGesture(f64, f64, Point<f64, Logical>),
    EndGesture(bool, ZoomLevelChangeAnim),
    Interrupt,
}

/// Transition-local focal continuity, driven by the level curve rather than another animation.
#[derive(Debug, Clone, Copy)]
struct TrackedCorrection {
    offset: Point<f64, Logical>,
    start_progress: f64,
}

impl TrackedCorrection {
    fn weight(self, progress: f64, level_done: bool) -> f64 {
        if level_done {
            return 0.;
        }
        if progress == self.start_progress {
            return 1.;
        }
        if progress >= 1. {
            return 0.;
        }
        let remaining = 1. - self.start_progress;
        if remaining <= ZOOM_CHANGE_EPSILON {
            // No unstable division in the last fraction of the level curve: preserve the
            // reseed sample above, then finish with ordinary geometry.
            return 0.;
        }
        (1. - progress) / remaining
    }
}

/// Displayed values are committed on ticks (or gesture input), never sampled by view().
/// Animation presets are supplied by callers; the cloned clock shares Niri's clock.
#[derive(Debug)]
pub struct OutputZoomState {
    displayed: OutputViewport,
    locked: bool,
    max_level: f64,
    // Mouse-only, output-local content. Synthetic seat reprojection only records it.
    cursor: Point<f64, Logical>,
    output_size: Size<f64, Logical>,
    clock: Clock,
    transition: Option<ZoomTransition>,

    pub movement_mode: ZoomMovementMode,
}

impl OutputZoomState {
    pub fn new(output: &Output, clock: Clock, zoom: Zoom) -> Self {
        let output_size = output_size(output);
        let center = Point::from((output_size.w / 2., output_size.h / 2.));
        Self {
            displayed: OutputViewport::new(center, 1.),
            locked: false,
            max_level: valid_max(zoom.max_zoom),
            movement_mode: zoom.movement_mode,
            cursor: center,
            output_size,
            clock,
            transition: None,
        }
    }

    pub fn view(&self) -> OutputViewport {
        self.displayed
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    fn update(&mut self, action: ZoomAction) -> (bool, PointerEffect) {
        self.apply_action(action)
    }

    fn apply_action(&mut self, action: ZoomAction) -> (bool, PointerEffect) {
        let mut policy = PointerEffect::KeepContent;
        let result = match action {
            ZoomAction::Advance => {
                policy = self.advance_animations();
                true
            }
            ZoomAction::Resize(size) => {
                policy = PointerEffect::PreserveScreen;
                self.set_output_size(size)
            }
            ZoomAction::Configure(zoom, level, focal) => self.set_config(&zoom, &level, &focal),
            ZoomAction::PointerMove(position) => {
                self.set_cursor(position);
                true
            }
            ZoomAction::PointerHandoff(position, config) => {
                self.handoff_pointer(position, config);
                true
            }
            ZoomAction::SetLevel(level, config) => {
                self.set_level(level, config);
                true
            }
            ZoomAction::ToggleLock(level, focal) => {
                self.toggle_lock(level, focal);
                true
            }
            ZoomAction::BeginGesture => self.begin_gesture(),
            ZoomAction::UpdateGesture(scale, sensitivity, pan_delta) => {
                let accepted = self.update_gesture(scale, sensitivity, pan_delta);
                if accepted {
                    policy = self.gesture_pointer_effect();
                }
                accepted
            }
            ZoomAction::EndGesture(cancelled, config) => self.end_gesture(cancelled, config),
            ZoomAction::Interrupt => {
                self.interrupt_pointer_driver();
                true
            }
        };
        (result, policy)
    }

    pub fn is_gesturing(&self) -> bool {
        matches!(self.transition, Some(ZoomTransition::Gesture(_)))
    }

    pub fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }

    pub fn are_animations_ongoing(&self) -> bool {
        matches!(
            self.transition,
            Some(
                ZoomTransition::Level(_)
                    | ZoomTransition::Pan(_)
                    | ZoomTransition::GestureReturn(_)
            )
        )
    }

    /// The only displayed-geometry writer after construction; no queue or seat callbacks.
    fn commit_displayed(&mut self, next: OutputViewport, size: Size<f64, Logical>) -> bool {
        if !next.factor.is_finite()
            || next.factor < 1.
            || !finite_point(next.focal)
            || !size.w.is_finite()
            || !size.h.is_finite()
            || size.w < 0.
            || size.h < 0.
        {
            return false;
        }
        let bounds = Rectangle::from_size(size);
        let next = OutputViewport {
            focal: next.focal.constrain(bounds),
            factor: next.factor,
        };
        self.displayed = next;
        self.output_size = size;
        true
    }

    pub(crate) fn record_cursor(&mut self, pos: Point<f64, Logical>) {
        if finite_point(pos) {
            self.cursor = pos.constrain(Rectangle::from_size(self.output_size));
        }
    }

    /// Choose the hidden cursor's mode target, or its saved screen point at a Centered edge.
    pub(crate) fn pointer_reappearance_target(
        &self,
        actual: Point<f64, Logical>,
        saved_screen: Option<Point<f64, Logical>>,
    ) -> Option<Point<f64, Logical>> {
        if self.locked {
            return None;
        }
        if !matches!(
            self.movement_mode,
            ZoomMovementMode::CursorFollow | ZoomMovementMode::Centered
        ) {
            return None;
        }
        if self.movement_mode == ZoomMovementMode::Centered && self.centered_view_touches_edge() {
            if let Some(screen) = saved_screen {
                return Some(self.view().screen_to_content(screen));
            }
        }
        self.pointer_handoff_target(actual)
    }

    fn centered_view_touches_edge(&self) -> bool {
        if self.displayed.factor <= 1. + ZOOM_CHANGE_EPSILON {
            return false;
        }
        let viewport = self.view().visible_viewport(self.output_size);
        let epsilon = 1e-6;
        viewport.loc.x <= epsilon
            || viewport.loc.y <= epsilon
            || viewport.loc.x + viewport.size.w >= self.output_size.w - epsilon
            || viewport.loc.y + viewport.size.h >= self.output_size.h - epsilon
    }

    /// Invert the current mode against committed geometry, using the actual seat, not its cache.
    /// This is only a proposed output-local content position; State owns legal seat dispatch.
    pub(crate) fn pointer_handoff_target(
        &self,
        actual: Point<f64, Logical>,
    ) -> Option<Point<f64, Logical>> {
        if self.locked || self.displayed.factor <= 1. || !finite_point(actual) {
            return None;
        }
        if self.movement_mode == ZoomMovementMode::Centered && self.centered_view_touches_edge() {
            // Like OnEdge, retain the seat position so pre-input handoff cannot recenter it.
            return Some(actual);
        }
        // Ordinary tracked level animations already own a compatible pointer. In particular,
        // its displayed focal may lag behind a real cursor move until the next animation tick;
        // do not mistake that frame-local lag for a stale touch anchor and warp the move away.
        if let Some(ZoomTransition::Level(level)) = &self.transition {
            if !different(level.tracking.offset, (0., 0.).into()) {
                return None;
            }
        }
        let compatible = !different(
            self.focal_for_cursor_at(
                self.displayed.factor,
                actual,
                self.tracked_anchor(Some(actual)),
            ),
            self.displayed.focal,
        );
        Some(match self.movement_mode {
            ZoomMovementMode::CursorFollow => self.displayed.focal,
            ZoomMovementMode::Centered if compatible => actual,
            // OnEdge derives its anchor by clamping the real cursor to the visible viewport.
            // Keep the seat where it is so pre-input handoff cannot change the legacy edge anchor.
            ZoomMovementMode::OnEdge => actual,
            _ => self.displayed.screen_to_content(Point::from((
                self.output_size.w / 2.,
                self.output_size.h / 2.,
            ))),
        })
    }

    /// Record an already-dispatched compatible seat position, never write the displayed view.
    /// Scalar animations retain their exact curve/target/timeline; touch focal ownership ends.
    fn handoff_pointer(&mut self, actual: Point<f64, Logical>, config: ZoomLevelChangeAnim) {
        if !finite_point(actual) {
            return;
        }
        self.record_cursor(actual);
        let transition = self.transition.take();
        self.transition = match transition {
            Some(ZoomTransition::Level(mut level)) => {
                // OnEdge already has zero correction and uses a fresh live anchor each tick.
                // Keep its initial/reseed guard rather than inventing another start sample.
                if self.movement_mode != ZoomMovementMode::OnEdge {
                    level.tracking = self.tracking_for(&level.animation);
                }
                Some(ZoomTransition::Level(level))
            }
            Some(ZoomTransition::GestureReturn(gesture)) => {
                let tracking = self.tracking_for(&gesture.animation);
                Some(ZoomTransition::Level(LevelAnimation {
                    animation: gesture.animation,
                    tracking,
                }))
            }
            Some(ZoomTransition::Gesture(_)) => {
                // Commit the current legal scalar, or settle only its rubber-band excess.
                if self.displayed.factor > self.max_level {
                    let animation = Animation::new(
                        self.clock.clone(),
                        self.displayed.factor,
                        self.max_level,
                        0.,
                        config.0,
                    );
                    let tracking = self.tracking_for(&animation);
                    Some(ZoomTransition::Level(LevelAnimation {
                        animation,
                        tracking,
                    }))
                } else {
                    None
                }
            }
            Some(ZoomTransition::Pan(_)) | None => None,
        };
    }

    fn advance_animations(&mut self) -> PointerEffect {
        let old = self.view();
        // OnEdge captures the OLD displayed geometry on every tick, not the start geometry.
        let anchor = self.tracked_anchor(None);
        let mut next = old;
        let mut done = false;
        let mut pointer_effect = PointerEffect::KeepContent;
        match &self.transition {
            Some(ZoomTransition::Level(level)) => {
                done = level.animation.is_done();
                next.factor = self.sample_level_animation(&level.animation, done);
                if !self.locked {
                    let correction = level.tracking;
                    let progress = level_progress_at(&level.animation, next.factor);
                    let focal = self.focal_for_at(next.factor, anchor);
                    if self.movement_mode == ZoomMovementMode::OnEdge {
                        // Match the established live-anchor behavior: OnEdge recomputes on every
                        // tick, even when the level sample itself has not advanced.
                        next.focal = focal;
                    } else {
                        next.focal =
                            focal + correction.offset.upscale(correction.weight(progress, done));
                    }
                }
            }
            Some(ZoomTransition::Pan(pan)) if !self.locked => {
                pointer_effect = pan.pointer_effect;
                done = pan.animation.is_done();
                next.focal = if done {
                    pan.animation.to
                } else {
                    pan.animation.value_at(self.clock.now())
                };
            }
            Some(ZoomTransition::Gesture(gesture)) => {
                pointer_effect = self.advance_gesture_focal(&mut next, &gesture.focal);
            }
            Some(ZoomTransition::GestureReturn(gesture)) => {
                done = gesture.animation.is_done();
                next.factor = self.sample_level_animation(&gesture.animation, done);
                pointer_effect = self.advance_gesture_focal(&mut next, &gesture.focal);
            }
            _ => (),
        }
        // Commit the final tracked/gesture focal and its pointer policy BEFORE owner sweep.
        self.commit_displayed(next, self.output_size);
        if done {
            self.transition = None;
        }
        pointer_effect
    }

    /// Live gesture updates and their coordinated return use the movement mode's focal policy.
    fn advance_gesture_focal(
        &self,
        next: &mut OutputViewport,
        focal: &GestureFocal,
    ) -> PointerEffect {
        if !self.locked {
            next.focal = self.gesture_focal_at(next.factor, focal);
        }
        self.gesture_pointer_effect()
    }

    fn gesture_pointer_effect(&self) -> PointerEffect {
        match self.movement_mode {
            ZoomMovementMode::Centered => PointerEffect::KeepContent,
            ZoomMovementMode::CursorFollow | ZoomMovementMode::OnEdge => {
                PointerEffect::PreserveScreen
            }
        }
    }

    fn sample_level_animation(&self, animation: &Animation, done: bool) -> f64 {
        if done {
            animation.to()
        } else {
            animation.value_at(self.clock.now()).clamp(
                animation.from().min(animation.to()),
                animation.from().max(animation.to()),
            )
        }
        .max(1.)
    }

    fn set_config(
        &mut self,
        zoom: &Zoom,
        level_anim: &ZoomLevelChangeAnim,
        focal_anim: &ZoomFocalPanAnim,
    ) -> bool {
        let old = self.view();
        self.max_level = valid_max(zoom.max_zoom);
        // Reconcile the destination, not just the currently displayed animation sample.
        let target = self
            .transition
            .as_ref()
            .and_then(ZoomTransition::level_animation)
            .map_or(self.displayed.factor, Animation::to);
        if self.is_gesturing() && !self.locked {
            self.end_gesture(false, *level_anim);
        }
        if !self.locked && target > self.max_level {
            self.set_level(target, *level_anim);
        }
        self.set_movement_mode(zoom.movement_mode, *focal_anim);
        old != self.view() || self.is_transitioning()
    }

    fn set_output_size(&mut self, size: Size<f64, Logical>) -> bool {
        if self.output_size == size {
            return false;
        }
        if !self.commit_displayed(self.displayed, size) {
            return false;
        }
        let bounds = Rectangle::from_size(size);
        self.cursor = self.cursor.constrain(bounds);
        match &mut self.transition {
            Some(ZoomTransition::Pan(pan)) => {
                pan.animation.from = pan.animation.from.constrain(bounds);
                pan.animation.to = pan.animation.to.constrain(bounds);
            }
            Some(ZoomTransition::Gesture(gesture)) => gesture.focal.reset_for_resize(bounds),
            Some(ZoomTransition::GestureReturn(gesture)) => gesture.focal.reset_for_resize(bounds),
            _ => (),
        }
        true
    }

    fn set_level(&mut self, level: f64, config: ZoomLevelChangeAnim) {
        if self.locked || !level.is_finite() {
            return;
        }
        let target = level.clamp(1., self.max_level);
        // Even an equal-level request interrupts a gesture/pan without changing this frame.
        self.transition = None;
        if (self.displayed.factor - target).abs() <= ZOOM_CHANGE_EPSILON {
            return;
        }
        let animation = Animation::new(
            self.clock.clone(),
            self.displayed.factor,
            target,
            0.,
            config.0,
        );
        let tracking = self.tracking_for(&animation);
        self.transition = Some(ZoomTransition::Level(LevelAnimation {
            animation,
            tracking,
        }));
    }

    fn tracking_for(&self, animation: &Animation) -> TrackedCorrection {
        let offset = match self.movement_mode {
            ZoomMovementMode::OnEdge => Point::from((0., 0.)),
            _ => {
                self.displayed.focal
                    - self.focal_for_at(self.displayed.factor, self.tracked_anchor(None))
            }
        };
        TrackedCorrection {
            offset,
            start_progress: level_progress_at(animation, self.displayed.factor),
        }
    }

    fn reseed_tracked(&mut self) {
        if let Some(ZoomTransition::Level(level)) = &self.transition {
            let tracking = self.tracking_for(&level.animation);
            if let Some(ZoomTransition::Level(level)) = &mut self.transition {
                level.tracking = tracking;
            }
        }
    }

    /// Only real pointer movement calls this. Reappearance at identical coordinates is a no-op.
    fn set_cursor(&mut self, pos: Point<f64, Logical>) {
        if !finite_point(pos) || pos == self.cursor {
            return;
        }
        self.record_cursor(pos);
        if self.locked || self.displayed.factor <= 1. {
            return;
        }
        if self.owns_level() {
            if self.movement_mode == ZoomMovementMode::OnEdge
                && matches!(&self.transition, Some(ZoomTransition::Level(_)))
            {
                // OnEdge updates focal from the current viewport as the cursor moves, even while
                // a level animation owns the zoom factor. Preserve that established behavior.
                let next = OutputViewport {
                    focal: self.focal_for_at(self.displayed.factor, self.tracked_anchor(None)),
                    factor: self.displayed.factor,
                };
                self.commit_displayed(next, self.output_size);
                self.reseed_tracked();
            }
            return;
        }
        let next = OutputViewport {
            focal: self.focal_for_at(self.displayed.factor, self.tracked_anchor(None)),
            factor: self.displayed.factor,
        };
        self.transition = None;
        self.commit_displayed(next, self.output_size);
    }

    fn owns_level(&self) -> bool {
        matches!(
            self.transition,
            Some(
                ZoomTransition::Level(_)
                    | ZoomTransition::Gesture(_)
                    | ZoomTransition::GestureReturn(_)
            )
        )
    }

    fn unlock_target(&self) -> Point<f64, Logical> {
        if self.movement_mode == ZoomMovementMode::CursorFollow {
            self.constrain_focal(self.view().content_to_screen(self.cursor))
        } else {
            self.focal_for_at(self.displayed.factor, self.tracked_anchor(None))
        }
    }

    fn start_pan(
        &mut self,
        target: Point<f64, Logical>,
        pointer_effect: PointerEffect,
        config: ZoomFocalPanAnim,
    ) {
        self.transition = if different(target, self.displayed.focal) {
            Some(ZoomTransition::Pan(FocalPan {
                animation: PointAnimation::new(
                    self.clock.clone(),
                    self.displayed.focal,
                    self.constrain_focal(target),
                    config.0,
                ),
                pointer_effect,
            }))
        } else {
            None
        };
    }

    fn set_movement_mode(&mut self, mode: ZoomMovementMode, config: ZoomFocalPanAnim) {
        if self.movement_mode == mode {
            return;
        }
        self.movement_mode = mode;
        if self.locked {
            return;
        }
        if self.owns_level() {
            self.reseed_tracked();
            return;
        }
        self.start_pan(
            self.focal_for_at(self.displayed.factor, self.tracked_anchor(None)),
            PointerEffect::KeepContent,
            config,
        );
    }

    fn toggle_lock(&mut self, level_config: ZoomLevelChangeAnim, focal_config: ZoomFocalPanAnim) {
        if !self.locked {
            if self.is_gesturing() {
                self.end_gesture(false, level_config);
            }
            self.locked = true;
            // Existing level animation continues, but focal is frozen, including gesture settling.
            if !self.owns_level() {
                self.transition = None;
            }
        } else {
            self.locked = false;
            let target = self
                .transition
                .as_ref()
                .and_then(ZoomTransition::level_animation)
                .map(Animation::to);
            // Lock froze focal, not the legal animation's destination or elapsed timeline.
            if target.is_some_and(|target| target <= self.max_level) {
                self.reseed_tracked(); // Does not replace GestureReturn's retained anchor.
                return;
            }
            // Reconcile the active destination, even when the displayed level is still legal.
            self.set_level(target.unwrap_or(self.displayed.factor), level_config);
            if self.transition.is_none() {
                let pointer_effect = match self.movement_mode {
                    ZoomMovementMode::Centered => PointerEffect::KeepContent,
                    _ => PointerEffect::PreserveScreen,
                };
                self.start_pan(self.unlock_target(), pointer_effect, focal_config);
            }
        }
    }

    fn begin_gesture(&mut self) -> bool {
        if self.locked || self.is_gesturing() {
            return false;
        }
        // Preserve continuity if a pinch interrupts a level transition. At 1× the focal is
        // visually irrelevant, so don't carry its arbitrary displayed value into the gesture.
        let cursor_anchor = self.cursor;
        let mode_anchor = self.tracked_anchor(Some(cursor_anchor));
        let focal_target =
            self.focal_for_cursor_at(self.displayed.factor, cursor_anchor, mode_anchor);
        let continuity_offset = if self.displayed.factor <= 1. + ZOOM_CHANGE_EPSILON
            || self.movement_mode == ZoomMovementMode::OnEdge
        {
            Point::from((0., 0.))
        } else {
            self.displayed.focal - focal_target
        };
        self.transition = Some(ZoomTransition::Gesture(ZoomGesture {
            start_level: self.displayed.factor,
            focal: GestureFocal {
                cursor_anchor,
                continuity_offset,
                screen_pan: Point::from((0., 0.)),
            },
        }));
        true
    }

    /// Scale selects the mode focal; touchscreen translation adds an independent pan.
    fn update_gesture(
        &mut self,
        scale: f64,
        sensitivity: f64,
        pan_delta: Point<f64, Logical>,
    ) -> bool {
        if !finite_point(pan_delta) {
            return false;
        }
        let Some(ZoomTransition::Gesture(gesture)) = &self.transition else {
            return false;
        };
        let Some(level) = gesture.sample_level(scale, sensitivity, self.max_level) else {
            return false;
        };
        let mut focal = gesture.focal;
        focal.screen_pan += pan_delta;
        if level <= 1. + ZOOM_CHANGE_EPSILON {
            focal.screen_pan = Point::from((0., 0.));
        }
        let (base_focal, requested_focal) = self.gesture_focal_parts(level, &focal);
        let next = OutputViewport {
            focal: self.constrain_focal(requested_focal),
            factor: level,
        };
        if !self.commit_displayed(next, self.output_size) {
            return false;
        }
        if level > 1. + ZOOM_CHANGE_EPSILON && different(self.displayed.focal, requested_focal) {
            // Bounds clipped this pan. Rebase its screen-space delta so reversing the gesture
            // moves away from the edge immediately instead of paying back clipped motion.
            focal.screen_pan = (base_focal - self.displayed.focal).upscale(level - 1.);
        }
        if let Some(ZoomTransition::Gesture(gesture)) = &mut self.transition {
            gesture.focal = focal;
        }
        true
    }

    fn end_gesture(&mut self, cancelled: bool, level_config: ZoomLevelChangeAnim) -> bool {
        let (start_level, focal) = match self.transition.take() {
            Some(ZoomTransition::Gesture(gesture)) => (gesture.start_level, gesture.focal),
            other => {
                self.transition = other;
                return false;
            }
        };
        let target = if cancelled {
            start_level
        } else {
            self.displayed.factor
        }
        .clamp(1., self.max_level);
        if (target - self.displayed.factor).abs() > ZOOM_CHANGE_EPSILON {
            // Cancellation/rubber-band settling keeps using the movement mode's focal policy.
            self.transition = Some(ZoomTransition::GestureReturn(GestureReturn {
                animation: Animation::new(
                    self.clock.clone(),
                    self.displayed.factor,
                    target,
                    0.,
                    level_config.0,
                ),
                focal,
            }));
            return true;
        }
        true
    }

    /// Stop a pointer-affecting driver when a manipulation grab/constraint becomes active.
    /// No rebase is sent through such a grab; the displayed view is retained.
    fn interrupt_pointer_driver(&mut self) {
        if matches!(
            self.transition,
            Some(
                ZoomTransition::Pan(_)
                    | ZoomTransition::Gesture(_)
                    | ZoomTransition::GestureReturn(_)
            )
        ) {
            self.transition = None;
        }
    }

    fn gesture_focal_parts(
        &self,
        level: f64,
        focal: &GestureFocal,
    ) -> (Point<f64, Logical>, Point<f64, Logical>) {
        let anchor = self.tracked_anchor(Some(focal.cursor_anchor));
        let target = self.focal_for_cursor_at(level, focal.cursor_anchor, anchor);
        let base = self.constrain_focal(target + focal.continuity_offset);
        let pan = if level <= 1. + ZOOM_CHANGE_EPSILON {
            Point::from((0., 0.))
        } else {
            focal.screen_pan.downscale(1. - level)
        };
        (base, base + pan)
    }

    fn gesture_focal_at(&self, level: f64, focal: &GestureFocal) -> Point<f64, Logical> {
        let (_, requested) = self.gesture_focal_parts(level, focal);
        self.constrain_focal(requested)
    }

    fn constrain_focal(&self, focal: Point<f64, Logical>) -> Point<f64, Logical> {
        focal.constrain(Rectangle::from_size(self.output_size))
    }

    fn tracked_anchor(&self, cursor: Option<Point<f64, Logical>>) -> Option<Point<f64, Logical>> {
        if self.movement_mode != ZoomMovementMode::OnEdge {
            return None;
        }
        let cursor = cursor.unwrap_or(self.cursor);
        let viewport = self.view().visible_viewport(self.output_size);
        let delta = cursor.constrain(viewport) - viewport.loc;
        Some(Point::from((
            (delta.x / viewport.size.w.max(0.0001)).min(1. - ZOOM_ONEDGE_FAR_ANCHOR_INSET),
            (delta.y / viewport.size.h.max(0.0001)).min(1. - ZOOM_ONEDGE_FAR_ANCHOR_INSET),
        )))
    }

    fn focal_for_at(&self, level: f64, anchor: Option<Point<f64, Logical>>) -> Point<f64, Logical> {
        self.focal_for_cursor_at(level, self.cursor, anchor)
    }

    fn focal_for_cursor_at(
        &self,
        level: f64,
        cursor: Point<f64, Logical>,
        anchor: Option<Point<f64, Logical>>,
    ) -> Point<f64, Logical> {
        if self.movement_mode == ZoomMovementMode::CursorFollow || level <= 1. {
            return self.constrain_focal(cursor);
        }
        let anchor = if self.movement_mode == ZoomMovementMode::OnEdge {
            anchor.unwrap_or(Point::from((0.5, 0.5)))
        } else {
            Point::from((0.5, 0.5))
        };
        let loc = cursor
            - Point::from((
                self.output_size.w / level * anchor.x,
                self.output_size.h / level * anchor.y,
            ));
        self.constrain_focal(loc.upscale(level / (level - 1.).max(0.0001)))
    }
}

fn level_progress_at(animation: &Animation, level: f64) -> f64 {
    let delta = animation.to() - animation.from();
    if delta.abs() <= ZOOM_CHANGE_EPSILON {
        return 1.;
    }
    ((level - animation.from()) / delta).clamp(0., 1.)
}

fn valid_max(max: f64) -> f64 {
    if max.is_finite() {
        max.max(1.)
    } else {
        10.
    }
}
fn finite_point(p: Point<f64, Logical>) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn different(a: Point<f64, Logical>, b: Point<f64, Logical>) -> bool {
    (a.x - b.x).abs() > ZOOM_CHANGE_EPSILON || (a.y - b.y).abs() > ZOOM_CHANGE_EPSILON
}

/// One progress curve for both coordinates, bounded even for spring overshoot.
#[derive(Debug, Clone)]
struct PointAnimation {
    from: Point<f64, Logical>,
    to: Point<f64, Logical>,
    progress: Animation,
}

impl PointAnimation {
    fn new(
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
        let t = self.progress.value_at(now).clamp(0., 1.);
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
    use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
    use smithay::utils::Size;

    use super::*;

    impl OutputZoomState {
        fn verify_invariants(&self) {
            let valid_size = |size: Size<f64, Logical>| {
                size.w.is_finite() && size.h.is_finite() && size.w >= 0. && size.h >= 0.
            };
            let valid_view = |view: OutputViewport, size: Size<f64, Logical>| {
                valid_size(size)
                    && view.factor.is_finite()
                    && view.factor >= 1.
                    && finite_point(view.focal)
                    && view.focal.constrain(Rectangle::from_size(size)) == view.focal
            };
            let valid_animation =
                |animation: &Animation| animation.from().is_finite() && animation.to().is_finite();

            assert!(self.max_level.is_finite() && self.max_level >= 1.);
            assert!(valid_view(self.displayed, self.output_size));
            let bounds = Rectangle::from_size(self.output_size);
            assert!(finite_point(self.cursor) && self.cursor.constrain(bounds) == self.cursor);

            match &self.transition {
                Some(ZoomTransition::Level(level)) => {
                    assert!(valid_animation(&level.animation));
                    assert!(finite_point(level.tracking.offset));
                    assert!((0. ..=1.).contains(&level.tracking.start_progress));
                }
                Some(ZoomTransition::Pan(pan)) => {
                    assert!(finite_point(pan.animation.from) && finite_point(pan.animation.to));
                }
                Some(ZoomTransition::Gesture(gesture)) => {
                    assert!(gesture.start_level.is_finite() && gesture.start_level >= 1.);
                    assert!(finite_point(gesture.focal.cursor_anchor));
                    assert!(finite_point(gesture.focal.continuity_offset));
                    assert!(finite_point(gesture.focal.screen_pan));
                }
                Some(ZoomTransition::GestureReturn(gesture)) => {
                    assert!(valid_animation(&gesture.animation));
                    assert!(finite_point(gesture.focal.cursor_anchor));
                    assert!(finite_point(gesture.focal.continuity_offset));
                    assert!(finite_point(gesture.focal.screen_pan));
                }
                None => (),
            }
        }
    }

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
        OutputZoomState::new(
            &test_output(),
            clock,
            Zoom {
                movement_mode: mode,
                ..Default::default()
            },
        )
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
        state.advance_animations();
        assert_relative_eq!(state.view().factor, 2.0);
        assert_relative_eq!(state.view().focal.x, 500., epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.y, 250., epsilon = 1e-6);

        state.set_cursor(Point::from((600., 250.)));

        state.set_level(3.0, ZoomLevelChangeAnim(easing(100)));
        assert!(state.is_transitioning());

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();

        assert!(!state.are_animations_ongoing());
        assert_relative_eq!(state.view().factor, 3.0);
        assert_relative_eq!(state.view().focal.x, 550., epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.y, 250., epsilon = 1e-6);
        state.verify_invariants();
    }

    #[test]
    fn pan_meets_target_and_level_change_replaces_pan() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::Centered, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.advance_animations();
        state.set_cursor(Point::from((700., 350.)));
        assert_relative_eq!(state.view().focal.x, 900.);
        assert_relative_eq!(state.view().focal.y, 450.);

        // Moving to CursorFollow starts a real pan from (900, 450) to the cursor (700, 350).
        state.set_movement_mode(
            ZoomMovementMode::CursorFollow,
            ZoomFocalPanAnim(easing(100)),
        );

        state.advance_animations();
        assert_relative_eq!(state.view().focal.x, 900.);
        assert_relative_eq!(state.view().focal.y, 450.);

        clock.set_unadjusted(Duration::from_millis(50));
        state.advance_animations();
        assert_relative_eq!(state.view().focal.x, 800., epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.y, 400., epsilon = 1e-6);

        // Starting a level change cancels the pan and replaces it with a tracked level
        // animation; a pan would still be running at t=100.
        state.set_level(3.0, ZoomLevelChangeAnim(easing(100)));

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();
        assert_relative_eq!(state.view().factor, 2.5, epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.x, 750., epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.y, 375., epsilon = 1e-6);

        clock.set_unadjusted(Duration::from_millis(150));
        state.advance_animations();

        assert!(!state.are_animations_ongoing());
        assert_relative_eq!(state.view().factor, 3.0);
        assert_relative_eq!(state.view().focal.x, 700., epsilon = f64::EPSILON);
        assert_relative_eq!(state.view().focal.y, 350., epsilon = f64::EPSILON);
        state.verify_invariants();
    }

    #[test]
    fn mode_change_during_level_animation_reseeds_without_view_jump() {
        let mut clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::OnEdge, clock.clone());

        state.set_level(2.0, ZoomLevelChangeAnim(anim_off()));
        state.advance_animations();
        state.set_cursor(Point::from((600., 250.)));

        state.set_level(3.0, ZoomLevelChangeAnim(easing(100)));
        let view_before = state.view();

        // The level animation owns the mode change; the tracked correction is reseeded
        // so the displayed view does not jump.
        state.set_movement_mode(ZoomMovementMode::Centered, ZoomFocalPanAnim(easing(100)));
        assert!(state.is_transitioning());
        assert_eq!(state.view(), view_before);

        clock.set_unadjusted(Duration::from_millis(50));
        state.advance_animations();
        assert_relative_eq!(state.view().factor, 2.5, epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.x, 1700. / 3., epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.y, 250., epsilon = 1e-6);

        clock.set_unadjusted(Duration::from_millis(100));
        state.advance_animations();

        assert!(!state.are_animations_ongoing());
        assert_relative_eq!(state.view().factor, 3.0);
        assert_relative_eq!(state.view().focal.x, 650., epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.y, 250., epsilon = 1e-6);
        state.verify_invariants();
    }

    #[test]
    fn locked_zoom_freezes_and_unlock_reclamps_after_max_level_decrease() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut state = OutputZoomState::new(
            &test_output(),
            clock.clone(),
            Zoom {
                movement_mode: ZoomMovementMode::Centered,
                max_zoom: 10.0,
                ..Default::default()
            },
        );

        state.set_level(8.0, ZoomLevelChangeAnim(anim_off()));
        state.advance_animations();
        assert_relative_eq!(state.view().factor, 8.0);
        let frozen = state.view();

        state.toggle_lock(
            ZoomLevelChangeAnim(anim_off()),
            ZoomFocalPanAnim(anim_off()),
        );
        assert!(state.locked());
        assert!(!state.are_animations_ongoing());

        // While locked: level requests are ignored, the cursor never moves the view.
        state.set_level(3.0, ZoomLevelChangeAnim(anim_off()));
        state.set_cursor(Point::from((200., 100.)));
        state.advance_animations();
        assert_eq!(state.view(), frozen);

        // Lowering the max zoom cannot move a locked level above the new max.
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
        assert!(!changed);
        assert_eq!(state.view(), frozen);

        // Unlocking re-clamps the level to the new max before the zoom resumes.
        state.toggle_lock(
            ZoomLevelChangeAnim(anim_off()),
            ZoomFocalPanAnim(anim_off()),
        );
        assert!(state.are_animations_ongoing());

        state.advance_animations();
        assert!(!state.are_animations_ongoing());
        assert!(!state.locked());
        assert_relative_eq!(state.view().factor, 5.0);
        assert_relative_eq!(state.view().focal.x, 125., epsilon = 1e-6);
        assert_relative_eq!(state.view().focal.y, 62.5, epsilon = 1e-6);
        state.verify_invariants();
    }

    #[test]
    fn cursor_follow_gesture_uses_the_cursor_focal() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::CursorFollow, clock);
        let cursor = Point::from((400., 250.));
        state.set_cursor(cursor);
        assert!(state.update(ZoomAction::BeginGesture).0);
        assert!(
            state
                .update(ZoomAction::UpdateGesture(2.0, 1.0, Point::from((0., 0.))))
                .0
        );

        let screen = state.view().content_to_screen(cursor);
        assert_relative_eq!(screen.x, 400., epsilon = 1e-6);
        assert_relative_eq!(screen.y, 250., epsilon = 1e-6);
    }

    #[test]
    fn centered_gesture_uses_the_centered_focal() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::Centered, clock);
        let cursor = Point::from((400., 250.));
        state.set_cursor(cursor);
        assert!(state.update(ZoomAction::BeginGesture).0);
        assert!(
            state
                .update(ZoomAction::UpdateGesture(2.0, 1.0, Point::from((0., 0.))))
                .0
        );

        let screen = state.view().content_to_screen(cursor);
        assert_relative_eq!(screen.x, 500., epsilon = 1e-6);
        assert_relative_eq!(screen.y, 250., epsilon = 1e-6);
    }

    #[test]
    fn on_edge_gesture_keeps_cursor_at_visible_viewport_edge() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::OnEdge, clock);
        let cursor = Point::from((800., 250.));
        state.set_cursor(cursor);
        state.set_level(2., ZoomLevelChangeAnim(anim_off()));
        state.advance_animations();
        state.set_cursor(Point::from((900., 250.)));
        let cursor: Point<f64, Logical> = Point::from((900., 250.));

        assert!(state.update(ZoomAction::BeginGesture).0);
        assert!(
            state
                .update(ZoomAction::UpdateGesture(1.5, 1., Point::from((0., 0.))))
                .0
        );

        let screen = state.view().content_to_screen(cursor);
        // The far-edge OnEdge anchor keeps a tiny inset below 1.0, so the hotspot maps exactly
        // to `output_size / level` minus one anchored singular-pixel step, never past it.
        assert_relative_eq!(
            screen.x,
            1000. * (1. - ZOOM_ONEDGE_FAR_ANCHOR_INSET),
            epsilon = 1e-6
        );
        assert_relative_eq!(screen.y, 250., epsilon = 1e-6);
    }

    #[test]
    fn cancelled_gesture_returns_to_its_start_level() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::CursorFollow, clock);
        let cursor = Point::from((400., 250.));
        state.set_cursor(cursor);
        assert!(state.update(ZoomAction::BeginGesture).0);
        assert!(
            state
                .update(ZoomAction::UpdateGesture(2., 1., Point::from((0., 0.))))
                .0
        );
        assert!(
            state
                .update(ZoomAction::EndGesture(
                    true,
                    ZoomLevelChangeAnim(anim_off())
                ))
                .0
        );
        assert!(state.are_animations_ongoing());

        assert!(state.update(ZoomAction::Advance).0);
        assert!(!state.are_animations_ongoing());
        assert_relative_eq!(state.view().factor, 1.);
        let screen = state.view().content_to_screen(cursor);
        assert_relative_eq!(screen.x, 400., epsilon = 1e-6);
        assert_relative_eq!(screen.y, 250., epsilon = 1e-6);
    }

    #[test]
    fn touchscreen_pan_rebases_when_the_focal_hits_an_edge() {
        let clock = Clock::with_time(Duration::ZERO);
        let mut state = zoom_state(ZoomMovementMode::CursorFollow, clock);
        let cursor = Point::from((0., 250.));
        state.set_cursor(cursor);
        state.set_level(8., ZoomLevelChangeAnim(anim_off()));
        state.advance_animations();
        assert!(state.update(ZoomAction::BeginGesture).0);

        assert!(
            state
                .update(ZoomAction::UpdateGesture(1., 1., Point::from((300., 0.)),))
                .0
        );
        let before_midpoint: Point<f64, Logical> = Point::from((400., 250.));
        let before_reversing: Point<f64, Logical> = state.view().screen_to_content(before_midpoint);
        assert_eq!(state.view().focal.x, 0.);

        assert!(
            state
                .update(ZoomAction::UpdateGesture(1., 1., Point::from((-100., 0.)),))
                .0
        );
        let after_midpoint: Point<f64, Logical> = Point::from((300., 250.));
        let after_reversing: Point<f64, Logical> = state.view().screen_to_content(after_midpoint);
        assert_relative_eq!(after_reversing.x, before_reversing.x, epsilon = 1e-6);
        assert_relative_eq!(after_reversing.y, before_reversing.y, epsilon = 1e-6);
    }
}
