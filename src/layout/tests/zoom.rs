use std::time::Duration;

use niri_config::animations::{Animation, Curve, EasingParams, Kind};
use niri_config::{Config, ZoomIncrementType, ZoomMovementMode};
use smithay::output::{Mode, Output, PhysicalProperties, Scale as OutputScale, Subpixel};
use smithay::utils::{Point, Rectangle, Scale, Size, Transform};

use super::*;
use crate::layout::zoom::{FocalTrackingContext, ZoomFocalAnimation, ZoomLevelAnimation};
use crate::layout::{Layout, LayoutElement};
use crate::utils::view::{OutputViewCtx, ViewportTransform};

impl<W: LayoutElement> Layout<W> {
    pub fn toggle_zoom_lock(&mut self, output: &Output) {
        if let Some(state) = self.zoom_states.get_mut(output) {
            state.locked = !state.locked;
        }
    }

    #[cfg(test)]
    pub fn zoom_set_state_for_test(
        &mut self,
        output: &Output,
        level: f64,
        focal: Point<f64, Local>,
        level_transition: zoom::ZoomLevelTransition,
        focal_animation: Option<zoom::ZoomFocalAnimation>,
    ) {
        if let Some(state) = self.zoom_states.get_mut(output) {
            state.level = level;
            state.focal = focal;
            state.level_transition = level_transition;
            state.focal_animation = focal_animation;
        }
    }
}

fn complete_animations(layout: &mut Layout<TestWindow>) {
    layout.clock.set_complete_instantly(true);
    // Advance past the animation start time so value_at() doesn't short-circuit
    // to `from` on the `at <= self.start_time` check before reaching
    // should_complete_instantly.
    let next_time = layout.clock.now_unadjusted() + Duration::from_secs(1);
    layout.clock.set_unadjusted(next_time);
    layout.advance_animations();
    layout.clock.set_complete_instantly(false);
}

fn make_output(name: &str, w: i32, h: i32) -> Output {
    let output = Output::new(
        name.to_string(),
        PhysicalProperties {
            size: Size::from((w, h)),
            subpixel: Subpixel::Unknown,
            make: String::new(),
            model: String::new(),
            serial_number: String::new(),
        },
    );
    output.change_current_state(
        Some(Mode {
            size: Size::from((w, h)),
            refresh: 60000,
        }),
        None,
        None,
        None,
    );
    output.user_data().insert_if_missing(|| OutputName {
        connector: name.to_string(),
        make: None,
        model: None,
        serial: None,
    });
    output
}

fn make_transformed_output(name: &str, w: i32, h: i32, transform: Transform, scale: f64) -> Output {
    let output = make_output(name, w, h);
    output.change_current_state(
        Some(Mode {
            size: Size::from((w, h)),
            refresh: 60000,
        }),
        Some(transform),
        Some(OutputScale::Fractional(scale)),
        None,
    );
    output
}

fn current_viewport(layout: &Layout<TestWindow>, output: &Output) -> ViewportTransform {
    let now = layout.clock.now();
    layout
        .zoom_state_for_output(output)
        .unwrap()
        .viewport_transform(now)
}

/// Lock preserves focal when level changes; unlock restores cursor tracking.
#[test]
fn locked_zoom_level_change_preserves_focal() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_set_level(
        &output,
        2.0,
        Point::from((0.0, 0.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);
    layout.toggle_zoom_lock(&output);
    complete_animations(&mut layout);
    let focal_before = layout.zoom_state_for_output(&output).unwrap().focal;

    layout.zoom_set_level(
        &output,
        5.0,
        Point::from((0.0, 0.0)),
        ZoomMovementMode::CursorFollow,
        true,
    );
    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 5.0).abs() < 1e-6);
    assert!((state.focal.x - focal_before.x).abs() < 1e-6);
    assert!((state.focal.y - focal_before.y).abs() < 1e-6);
    assert!(state.locked);
}

#[test]
fn layout_zoom_store_is_removed_on_remove_output() {
    let mut layout = Layout::<TestWindow>::default();
    let output1 = make_output("o1", 1920, 1080);
    let output2 = make_output("o2", 1280, 720);
    layout.add_output(output1.clone(), None);
    layout.add_output(output2.clone(), None);

    assert!(layout.zoom_state_for_output(&output1).is_some());
    assert!(layout.zoom_state_for_output(&output2).is_some());

    layout.remove_output(&output2);
    assert!(layout.zoom_state_for_output(&output2).is_none());
    assert!(layout.zoom_state_for_output(&output1).is_some());

    layout.remove_output(&output1);
    assert!(layout.zoom_state_for_output(&output1).is_none());
}

#[test]
fn zoom_levels_are_independent_per_output() {
    let mut layout = Layout::<TestWindow>::default();
    let output1 = make_output("o1", 1920, 1080);
    let output2 = make_output("o2", 1280, 720);
    layout.add_output(output1.clone(), None);
    layout.add_output(output2.clone(), None);

    layout.zoom_set_level(
        &output1,
        2.0,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);

    assert!((layout.zoom_state_for_output(&output1).unwrap().level - 2.0).abs() < 1e-6);
    assert!((layout.zoom_state_for_output(&output2).unwrap().level - 1.0).abs() < 1e-6);
}

#[test]
fn removing_one_output_does_not_change_other_output_zoom() {
    let mut layout = Layout::<TestWindow>::default();
    let output1 = make_output("o1", 1920, 1080);
    let output2 = make_output("o2", 1280, 720);
    layout.add_output(output1.clone(), None);
    layout.add_output(output2.clone(), None);

    layout.zoom_set_level(
        &output1,
        2.0,
        Point::from((50.0, 50.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);
    let before = layout.zoom_state_for_output(&output1).unwrap().level;

    layout.remove_output(&output2);
    let after = layout.zoom_state_for_output(&output1).unwrap().level;

    assert!((after - before).abs() < 1e-6);
}

#[test]
fn centered_zoom_level_change_animates_when_target_is_edge_constrained() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor_local = Point::from((10.0, 10.0));
    let output_size = Size::from((1920.0, 1080.0));

    layout.zoom_set_level(
        &output,
        2.0,
        cursor_local,
        ZoomMovementMode::Centered,
        false,
    );

    assert!(layout
        .zoom_state_for_output(&output)
        .unwrap()
        .transitioning());

    complete_animations(&mut layout);
    let state = layout.zoom_state_for_output(&output).unwrap();
    let expected_focal = FocalTrackingContext::focal_for_cursor(
        cursor_local,
        2.0,
        output_size,
        &ZoomMovementMode::Centered,
    );

    assert!(!state.transitioning());
    assert!((state.level - 2.0).abs() < 1e-6);
    assert!((state.focal.x - expected_focal.x).abs() < 1e-6);
    assert!((state.focal.y - expected_focal.y).abs() < 1e-6);
}

#[test]
fn zoom_snapshot_reports_consistent_level_focal_locked() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 1.0).abs() < 1e-6);
    assert!((state.focal.x - 960.0).abs() < 1e-3);
    assert!((state.focal.y - 540.0).abs() < 1e-3);
    assert!(!state.locked);

    layout.zoom_set_level(
        &output,
        2.0,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 2.0).abs() < 1e-6);
    assert!((state.focal.x - 100.0).abs() < 1e-3);
    assert!((state.focal.y - 100.0).abs() < 1e-3);
    assert!(!state.locked);

    layout.toggle_zoom_lock(&output);
    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 2.0).abs() < 1e-6);
    assert!(state.locked);
}

#[test]
fn on_edge_set_zoom_level_creates_animating_transition() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor_local = Point::from((500.0, 400.0));

    layout.zoom_set_level(&output, 2.0, cursor_local, ZoomMovementMode::OnEdge, false);

    complete_animations(&mut layout);
    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 2.0).abs() < 1e-6);
    assert!(
        (state.focal.x - 500.0).abs() < 1.0,
        "OnEdge focal.x should be near cursor.x=500, got {}",
        state.focal.x,
    );
}

#[test]
fn on_edge_gesture_focal_uses_anchor_when_cursor_within_viewport() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor_local = Point::from((500.0, 400.0));
    let output_size = Size::from((1920.0, 1080.0));

    layout.zoom_gesture_begin(
        &output,
        Some(cursor_local),
        Some(output_size),
        Some(ZoomMovementMode::OnEdge),
    );

    let _ = layout.zoom_gesture_update(
        &output,
        1.0,
        1.0,
        Duration::from_millis(16),
        Some(cursor_local),
        Some(output_size),
    );

    let result = layout.zoom_gesture_update(
        &output,
        2.0,
        1.0,
        Duration::from_millis(32),
        Some(cursor_local),
        Some(output_size),
    );
    assert!(result.is_some(), "gesture update should succeed");

    let viewport = current_viewport(&layout, &output);
    assert!(
        viewport.factor > 1.0,
        "gesture level should increase above 1.0, got {}",
        viewport.factor,
    );
    assert!(
        (viewport.focal.x - cursor_local.x).abs() < (960.0 - cursor_local.x).abs(),
        "OnEdge focal should track cursor (focal.x={}, cursor.x={})",
        viewport.focal.x,
        cursor_local.x,
    );
    assert!(
        (viewport.focal.y - cursor_local.y).abs() < (540.0 - cursor_local.y).abs(),
        "OnEdge focal should track cursor (focal.y={}, cursor.y={})",
        viewport.focal.y,
        cursor_local.y,
    );
    assert!(
        viewport.focal.x >= 0.0 && viewport.focal.x <= 1920.0,
        "focal.x {} out of bounds",
        viewport.focal.x
    );
    assert!(
        viewport.focal.y >= 0.0 && viewport.focal.y <= 1080.0,
        "focal.y {} out of bounds",
        viewport.focal.y
    );
}

#[test]
fn on_edge_gesture_tracks_cursor_pos_within_viewport() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor_local = Point::from((500.0, 400.0));
    let output_size = Size::from((1920.0, 1080.0));

    layout.zoom_gesture_begin(
        &output,
        Some(cursor_local),
        Some(output_size),
        Some(ZoomMovementMode::OnEdge),
    );

    let new_cursor = Point::from((700.0, 500.0));
    let result = layout.zoom_gesture_update(
        &output,
        2.0,
        1.0,
        Duration::from_millis(16),
        Some(new_cursor),
        Some(output_size),
    );
    assert!(result.is_some());

    let viewport = current_viewport(&layout, &output);
    assert!(
        viewport.focal.x >= 0.0 && viewport.focal.x <= 1920.0,
        "focal.x {} out of bounds",
        viewport.focal.x
    );
    assert!(
        viewport.focal.y >= 0.0 && viewport.focal.y <= 1080.0,
        "focal.y {} out of bounds",
        viewport.focal.y
    );
    assert!(
        (viewport.focal.x - new_cursor.x).abs() < (960.0 - new_cursor.x).abs(),
        "focal.x {} should be closer to cursor.x={} than to center",
        viewport.focal.x,
        new_cursor.x,
    );
}

#[test]
fn update_zoom_movement_mode_recomputes_on_edge_anchor() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor_local = Point::from((500.0, 400.0));

    layout.zoom_set_level(
        &output,
        2.0,
        cursor_local,
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);
    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.focal.x - 500.0).abs() < 1.0);
    assert!((state.focal.y - 400.0).abs() < 1.0);

    layout
        .zoom_state_for_output_mut(&output)
        .unwrap()
        .update_movement_mode(ZoomMovementMode::OnEdge);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        (state.focal.x - 500.0).abs() < 2.0,
        "After mode change to OnEdge, focal.x should be near cursor.x=500, got {}",
        state.focal.x,
    );
}

#[test]
fn update_zoom_movement_mode_noop_when_no_transition() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout
        .zoom_state_for_output_mut(&output)
        .unwrap()
        .update_movement_mode(ZoomMovementMode::OnEdge);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 1.0).abs() < 1e-6);
}

#[test]
fn off_true_zoom_level_change_skips_transition() {
    let mut config = Config::default();
    config.animations.zoom_level_change.0.off = true;
    let mut layout = Layout::<TestWindow>::new(Clock::with_time(Duration::ZERO), &config);
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_set_level(
        &output,
        2.0,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );

    // Animation::new() with config.off=true sets from=to, so value_at
    // returns to immediately, and the transition is_done_at returns true,
    // so it gets cleared once advance_animations runs.
    complete_animations(&mut layout);
    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        !state.transitioning(),
        "off=true should not leave a pending transition"
    );
    assert!(
        (state.level - 2.0).abs() < 1e-6,
        "off=true should snap to target level immediately"
    );
}

#[test]
fn zero_duration_zoom_level_change_skips_transition() {
    use niri_config::animations::{Animation as AnimConf, Curve as C, EasingParams, Kind as K};

    let mut config = Config::default();
    config.animations.zoom_level_change.0 = AnimConf {
        off: false,
        kind: K::Easing(EasingParams {
            duration_ms: 0,
            curve: C::Linear,
        }),
    };
    let mut layout = Layout::<TestWindow>::new(Clock::with_time(Duration::ZERO), &config);
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_set_level(
        &output,
        2.0,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );

    complete_animations(&mut layout);
    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        !state.transitioning(),
        "zero-duration animation should not leave a pending transition"
    );
    assert!(
        (state.level - 2.0).abs() < 1e-6,
        "zero-duration should snap to target level immediately"
    );
}

#[test]
fn zoom_gesture_on_one_output_does_not_affect_other() {
    let mut layout = Layout::<TestWindow>::default();
    let output1 = make_output("o1", 1920, 1080);
    let output2 = make_output("o2", 1920, 1080);
    layout.add_output(output1.clone(), None);
    layout.add_output(output2.clone(), None);
    let output_size = Size::from((1920.0, 1080.0));
    let cursor_local = Point::from((500.0, 400.0));

    let initial_level2 = layout.zoom_state_for_output(&output2).unwrap().level;

    layout.zoom_gesture_begin(
        &output1,
        Some(cursor_local),
        Some(output_size),
        Some(ZoomMovementMode::CursorFollow),
    );

    let _ = layout.zoom_gesture_update(
        &output1,
        2.0,
        1.0,
        Duration::from_millis(16),
        Some(cursor_local),
        Some(output_size),
    );

    let level2_during = layout.zoom_state_for_output(&output2).unwrap().level;
    assert!(
        (level2_during - initial_level2).abs() < 1e-6,
        "output 2 level should not change during output 1 gesture"
    );

    layout.zoom_gesture_end(&output1, false);

    let level2_after = layout.zoom_state_for_output(&output2).unwrap().level;
    assert!(
        (level2_after - initial_level2).abs() < 1e-6,
        "output 2 level should not change after output 1 gesture ends"
    );
}

#[test]
fn zoom_gesture_cursor_moves_between_outputs() {
    let mut layout = Layout::<TestWindow>::default();
    let output1 = make_output("o1", 1920, 1080);
    let output2 = make_output("o2", 1920, 1080);
    layout.add_output(output1.clone(), None);
    layout.add_output(output2.clone(), None);
    let output_size = Size::from((1920.0, 1080.0));

    let cursor_on_output1 = Point::from((500.0, 400.0));
    let cursor_on_output2 = Point::from((2500.0, 500.0));

    let initial_level2 = layout.zoom_state_for_output(&output2).unwrap().level;

    layout.zoom_gesture_begin(
        &output1,
        Some(cursor_on_output1),
        Some(output_size),
        Some(ZoomMovementMode::CursorFollow),
    );

    let _ = layout.zoom_gesture_update(
        &output1,
        2.0,
        1.0,
        Duration::from_millis(16),
        Some(cursor_on_output2),
        Some(output_size),
    );
    let _ = layout.zoom_gesture_update(
        &output1,
        4.0,
        1.0,
        Duration::from_millis(32),
        Some(cursor_on_output2),
        Some(output_size),
    );

    let level1 = current_viewport(&layout, &output1).factor;
    assert!(
        level1 > 1.0,
        "output 1 level should increase during pinch gesture"
    );

    let level2_during = layout.zoom_state_for_output(&output2).unwrap().level;
    assert!(
        (level2_during - initial_level2).abs() < 1e-6,
        "output 2 level should not change when cursor moves to output 2 during output 1 gesture"
    );

    layout.zoom_gesture_end(&output1, false);
    complete_animations(&mut layout);

    let level2_after = layout.zoom_state_for_output(&output2).unwrap().level;
    assert!(
        (level2_after - initial_level2).abs() < 1e-6,
        "output 2 level should not change after gesture ends"
    );
}

#[test]
fn zoom_gesture_update_accepts_cursor_local_and_updates_focal() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let output_size = Size::from((1920.0, 1080.0));

    layout.zoom_gesture_begin(
        &output,
        Some(Point::from((100.0, 100.0))),
        Some(output_size),
        Some(ZoomMovementMode::CursorFollow),
    );

    let cursor_local = Point::from((500.0, 500.0));
    let result = layout.zoom_gesture_update(
        &output,
        2.0,
        1.0,
        Duration::from_millis(16),
        Some(cursor_local),
        Some(output_size),
    );
    assert!(result.is_some());

    let focal = current_viewport(&layout, &output).focal;
    assert!(
        (focal.x - 500.0).abs() < 1e-6,
        "CursorFollow focal.x {} != 500.0",
        focal.x
    );
    assert!(
        (focal.y - 500.0).abs() < 1e-6,
        "CursorFollow focal.y {} != 500.0",
        focal.y
    );
}

#[test]
fn zoom_gesture_end_maintains_level_with_no_animation() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let output_size = Size::from((1920.0, 1080.0));
    let cursor_local = Point::from((500.0, 400.0));

    layout.zoom_gesture_begin(
        &output,
        Some(cursor_local),
        Some(output_size),
        Some(ZoomMovementMode::CursorFollow),
    );

    assert!(layout
        .zoom_gesture_update(
            &output,
            1.0,
            1.0,
            Duration::from_millis(16),
            Some(cursor_local),
            Some(output_size),
        )
        .is_some());
    assert!(layout
        .zoom_gesture_update(
            &output,
            2.0,
            1.0,
            Duration::from_millis(32),
            Some(cursor_local),
            Some(output_size),
        )
        .is_some());

    assert_eq!(layout.zoom_gesture_end(&output, false), Some(true));
    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(!state.transitioning());
    assert!((state.level - 2.0).abs() < 1e-6);
    assert!((state.focal.x - 500.0).abs() < 1e-6);
    assert!((state.focal.y - 400.0).abs() < 1e-6);
}

#[test]
fn zoom_gesture_cancel_animates_back_to_start_level() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let output_size = Size::from((1920.0, 1080.0));
    let cursor_local = Point::from((500.0, 400.0));

    layout.zoom_gesture_begin(
        &output,
        Some(cursor_local),
        Some(output_size),
        Some(ZoomMovementMode::CursorFollow),
    );

    assert!(layout
        .zoom_gesture_update(
            &output,
            1.0,
            1.0,
            Duration::from_millis(16),
            Some(cursor_local),
            Some(output_size),
        )
        .is_some());
    assert!(layout
        .zoom_gesture_update(
            &output,
            2.0,
            1.0,
            Duration::from_millis(32),
            Some(cursor_local),
            Some(output_size),
        )
        .is_some());

    assert_eq!(layout.zoom_gesture_end(&output, true), Some(true));
    let state_before = layout.zoom_state_for_output(&output).unwrap();
    assert!(state_before.transitioning());
    assert!((state_before.focal.x - 500.0).abs() < 1e-6);
    assert!((state_before.focal.y - 400.0).abs() < 1e-6);

    complete_animations(&mut layout);
    let state_after = layout.zoom_state_for_output(&output).unwrap();
    assert!((state_after.level - 1.0).abs() < 1e-6);
    assert!(!state_after.transitioning());
    assert!((state_after.focal.x - 500.0).abs() < 1e-6);
    assert!((state_after.focal.y - 400.0).abs() < 1e-6);
}

proptest! {
    /// Invariant: viewport_global output is within valid bounds for various
    /// zoom levels and focal points.
    #[test]
    fn zoom_state_viewport_bounds(
        level in 1.0f64..=5.0f64,
        focal_x in 0.0f64..1920.0f64,
        focal_y in 0.0f64..1080.0f64,
    ) {
        let state = crate::layout::zoom::OutputZoomState {
            level,
            focal: Point::from((focal_x, focal_y)),
            locked: false,
            level_transition: ZoomLevelTransition::Idle,
            focal_animation: None,
        };
        let output_view_ctx = OutputViewCtx {
            global_geo: Rectangle::from_size(Size::from((1920.0f64, 1080.0f64))),
            local_geo: Rectangle::from_size(Size::from((1920.0f64, 1080.0f64))),
            output_transform: Transform::Normal,
            scale: Scale::from(1.0f64),
        };
        let viewport = state.viewport_global(&output_view_ctx, Duration::ZERO);

        prop_assert!(viewport.size.w > 0.0, "viewport width must be positive");
        prop_assert!(viewport.size.h > 0.0, "viewport height must be positive");
        prop_assert!(
            viewport.size.w <= 1920.0 + 1e-9,
            "viewport width {} exceeds output width 1920",
            viewport.size.w,
        );
        prop_assert!(
            viewport.size.h <= 1080.0 + 1e-9,
            "viewport height {} exceeds output height 1080",
            viewport.size.h,
        );
    }
}

#[test]
fn focal_output_size_applies_output_transform() {
    let normal = make_transformed_output("normal", 1920, 1080, Transform::Normal, 1.0);
    let rotate_90 = make_transformed_output("rotate-90", 1920, 1080, Transform::_90, 1.0);
    let rotate_180 = make_transformed_output("rotate-180", 1920, 1080, Transform::_180, 1.0);
    let flipped = make_transformed_output("flipped", 1920, 1080, Transform::Flipped, 1.0);
    let rotate_scaled = make_transformed_output("rotate-scaled", 1920, 1080, Transform::_90, 1.5);

    assert_eq!(
        Layout::<TestWindow>::output_size_for_focal(&normal),
        (1920., 1080.).into()
    );
    assert_eq!(
        Layout::<TestWindow>::output_size_for_focal(&rotate_90),
        (1080., 1920.).into()
    );
    assert_eq!(
        Layout::<TestWindow>::output_size_for_focal(&rotate_180),
        (1920., 1080.).into()
    );
    assert_eq!(
        Layout::<TestWindow>::output_size_for_focal(&flipped),
        (1920., 1080.).into()
    );
    assert_eq!(
        Layout::<TestWindow>::output_size_for_focal(&rotate_scaled),
        (720., 1280.).into(),
    );
}

#[test]
fn rotated_focal_tracking_stays_within_transformed_output() {
    let output = make_transformed_output("rotate-90", 1920, 1080, Transform::_90, 1.0);
    let size = Layout::<TestWindow>::output_size_for_focal(&output);
    let mut tracking = FocalTrackingContext::default();
    tracking.set_cursor_pos((0., 0.).into());
    tracking.set_output_size(size);
    tracking.set_movement_mode(ZoomMovementMode::Centered, 1.0, (0., 0.).into());

    let focal = tracking.compute_focal(2.0, (0., 0.).into());
    assert!(focal.x >= 0.0 && focal.x <= size.w);
    assert!(focal.y >= 0.0 && focal.y <= size.h);
}

#[test]
fn on_edge_focal_tracking_handles_all_rotated_output_corners() {
    let output = make_transformed_output("rotate-90", 1920, 1080, Transform::_90, 1.0);
    let size = Layout::<TestWindow>::output_size_for_focal(&output);
    let corners = [(0.0, 0.0), (size.w, 0.0), (0.0, size.h), (size.w, size.h)];

    for cursor in corners {
        let cursor = Point::from(cursor);
        let mut tracking = FocalTrackingContext::default();
        tracking.set_cursor_pos(cursor);
        tracking.set_output_size(size);
        tracking.set_movement_mode(
            ZoomMovementMode::OnEdge,
            2.0,
            (size.w / 2.0, size.h / 2.0).into(),
        );

        let focal = tracking.compute_focal(2.0, cursor);
        assert!(focal.x >= 0.0 && focal.x <= size.w, "focal x: {focal:?}");
        assert!(focal.y >= 0.0 && focal.y <= size.h, "focal y: {focal:?}");
    }
}

#[test]
fn composed_level_focal_animation_completes_to_targets() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let output_size = Size::from((1920.0, 1080.0));
    let cursor_local = Point::from((700.0, 400.0));
    let target_level = 2.0;
    let target_focal = FocalTrackingContext::focal_for_cursor(
        cursor_local,
        target_level,
        output_size,
        &ZoomMovementMode::Centered,
    );

    complete_animations(&mut layout);

    let clock = layout.clock.clone();
    let focal_init = Point::from((960.0, 540.0));

    let level_config = Animation {
        off: false,
        kind: Kind::Easing(EasingParams {
            duration_ms: 250,
            curve: Curve::EaseOutExpo,
        }),
    };
    let focal_config = Animation {
        off: false,
        kind: Kind::Easing(EasingParams {
            duration_ms: 250,
            curve: Curve::CubicBezier(0.05, 0.7, 0.1, 1.0),
        }),
    };

    let level_anim = ZoomLevelAnimation::new(clock.clone(), 1.0, target_level, level_config);
    let focal_anim = ZoomFocalAnimation::new(clock, focal_init, target_focal, focal_config);

    layout.zoom_set_state_for_test(
        &output,
        1.0,
        focal_init,
        ZoomLevelTransition::Animating(level_anim),
        Some(focal_anim),
    );

    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        (state.level - target_level).abs() < 1e-6,
        "level {} != {}",
        state.level,
        target_level,
    );
    assert!(
        (state.focal.x - target_focal.x).abs() < 1e-6,
        "focal.x {} != {}",
        state.focal.x,
        target_focal.x,
    );
    assert!(
        (state.focal.y - target_focal.y).abs() < 1e-6,
        "focal.y {} != {}",
        state.focal.y,
        target_focal.y,
    );
}

#[test]
fn zoom_transition_snapshot_values() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    let start_level = 1.0;
    let target_level = 2.0;
    let center = Point::from((960.0, 540.0));

    layout.zoom_set_level(
        &output,
        target_level,
        center,
        ZoomMovementMode::Centered,
        false,
    );

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        (state.level - start_level).abs() < 1e-6,
        "before animation: level should be {} (start), got {}",
        start_level,
        state.level,
    );
    assert!(
        (state.focal.x - 960.0).abs() < 1.0,
        "before animation: Centered focal.x should be at output center (960), got {}",
        state.focal.x,
    );

    complete_animations(&mut layout);
    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        (state.level - target_level).abs() < 1e-6,
        "after completion: level should be {} (target), got {}",
        target_level,
        state.level,
    );
    assert!(
        (state.focal.x - 960.0).abs() < 1.0,
        "after completion: Centered focal.x should stay at output center (960), got {}",
        state.focal.x,
    );
}

proptest! {
    /// Invariant: focal computation returns points within output bounds for
    /// all movement modes.
    #[test]
    fn compute_focal_bounds(
        cursor_x in 0.0f64..1920.0f64,
        cursor_y in 0.0f64..1080.0f64,
        level in 1.0f64..=10.0f64,
        mode in prop_oneof![
            Just(ZoomMovementMode::CursorFollow),
            Just(ZoomMovementMode::Centered),
            Just(ZoomMovementMode::OnEdge),
        ],
    ) {
        let output_size = Size::from((1920.0f64, 1080.0f64));
        let cursor = Point::from((cursor_x, cursor_y));
        let focal = FocalTrackingContext::focal_for_cursor(cursor, level, output_size, &mode);

        prop_assert!(
            focal.x >= 0.0 && focal.x <= 1920.0,
            "focal.x {} out of [0, 1920] for mode {:?}", focal.x, mode
        );
        prop_assert!(
            focal.y >= 0.0 && focal.y <= 1080.0,
            "focal.y {} out of [0, 1080] for mode {:?}", focal.y, mode
        );
    }
}

#[test]
fn animation_interruption_restarts_to_new_target() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_set_level(
        &output,
        2.0,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    layout.zoom_set_level(
        &output,
        3.0,
        Point::from((200.0, 200.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );

    assert!(
        layout
            .zoom_state_for_output(&output)
            .unwrap()
            .transitioning(),
        "interrupted animation should still have a transition"
    );

    complete_animations(&mut layout);
    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        (state.level - 3.0).abs() < 1e-6,
        "final level should be 3.0, got {}",
        state.level
    );
}

#[test]
fn set_zoom_level_during_gesture_clears_it() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor = Point::from((500.0, 400.0));
    let output_size = Size::from((1920.0, 1080.0));

    layout.zoom_gesture_begin(
        &output,
        Some(cursor),
        Some(output_size),
        Some(ZoomMovementMode::CursorFollow),
    );
    let _ = layout.zoom_gesture_update(
        &output,
        1.0,
        1.0,
        Duration::from_millis(16),
        Some(cursor),
        Some(output_size),
    );
    let _ = layout.zoom_gesture_update(
        &output,
        2.0,
        1.0,
        Duration::from_millis(32),
        Some(cursor),
        Some(output_size),
    );

    layout.zoom_set_level(&output, 3.0, cursor, ZoomMovementMode::CursorFollow, false);

    assert_eq!(
        layout.zoom_gesture_end(&output, false),
        None,
        "set_zoom_level should clear the gesture",
    );

    assert!(
        layout
            .zoom_state_for_output(&output)
            .unwrap()
            .transitioning(),
        "set_zoom_level should create an animation"
    );

    complete_animations(&mut layout);
    assert!(
        (layout.zoom_state_for_output(&output).unwrap().level - 3.0).abs() < 1e-6,
        "final level should be 3.0",
    );
}

#[test]
fn toggle_zoom_lock_during_gesture_does_not_panic() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor = Point::from((500.0, 400.0));
    let output_size = Size::from((1920.0, 1080.0));

    layout.zoom_gesture_begin(
        &output,
        Some(cursor),
        Some(output_size),
        Some(ZoomMovementMode::CursorFollow),
    );

    let _ = layout.zoom_gesture_update(
        &output,
        2.0,
        1.0,
        Duration::from_millis(16),
        Some(cursor),
        Some(output_size),
    );

    layout.toggle_zoom_lock(&output);
    assert!(
        layout.zoom_state_for_output(&output).unwrap().locked,
        "lock should be toggled on"
    );

    let result = layout.zoom_gesture_end(&output, false);
    assert!(
        result.is_some(),
        "gesture end after lock toggle should succeed"
    );
}

#[test]
fn zoom_level_clamps_below_minimum() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_set_level(
        &output,
        0.5,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        (state.level - 1.0).abs() < 1e-6,
        "level {} should be clamped to min 1.0",
        state.level
    );
}

#[test]
fn zoom_level_clamps_above_maximum() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_set_level(
        &output,
        30.0,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!(
        (state.level - 10.0).abs() < 1e-6,
        "level {} should be clamped to max 10.0",
        state.level
    );
}

#[test]
fn focal_only_animation_updates_state_focal() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_set_level(
        &output,
        2.0,
        Point::from((100.0, 100.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );
    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 2.0).abs() < 1e-6);
    let old_focal = state.focal;

    let clock = layout.clock.clone();
    let focal_init = old_focal;
    let focal_target = Point::from((100.0, 100.0));
    let focal_config = Animation {
        off: false,
        kind: Kind::Easing(EasingParams {
            duration_ms: 250,
            curve: Curve::EaseOutExpo,
        }),
    };
    let focal_anim = ZoomFocalAnimation::new(clock, focal_init, focal_target, focal_config);

    layout.zoom_set_state_for_test(
        &output,
        2.0,
        old_focal,
        ZoomLevelTransition::Idle,
        Some(focal_anim),
    );

    complete_animations(&mut layout);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert!((state.level - 2.0).abs() < 1e-6);
    assert!(
        (state.focal.x - focal_target.x).abs() < 1e-6,
        "focal.x {} should be {} after focal-only animation",
        state.focal.x,
        focal_target.x,
    );
    assert!(
        (state.focal.y - focal_target.y).abs() < 1e-6,
        "focal.y {} should be {} after focal-only animation",
        state.focal.y,
        focal_target.y,
    );

    assert!(matches!(state.level_transition, ZoomLevelTransition::Idle));
    assert!(state.focal_animation.is_none());
}

#[test]
fn set_zoom_lock_returns_previous_state() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);

    assert!(!layout.set_zoom_lock(&output, true));
    layout.add_output(output.clone(), None);

    assert!(!layout.set_zoom_lock(&output, true));
    assert!(layout.zoom_state_for_output(&output).unwrap().locked);
    assert!(layout.set_zoom_lock(&output, false));
    assert!(!layout.zoom_state_for_output(&output).unwrap().locked);
}

#[test]
fn zoom_in_and_out_use_linear_steps() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor = Point::from((500.0, 400.0));

    layout.zoom_in(&output, cursor);
    complete_animations(&mut layout);
    assert_eq!(layout.zoom_state_for_output(&output).unwrap().level, 2.0);

    layout.zoom_out(&output, cursor);
    complete_animations(&mut layout);
    assert_eq!(layout.zoom_state_for_output(&output).unwrap().level, 1.0);
}

#[test]
fn exponential_zoom_steps_double_and_halve() {
    let mut config = Config::default();
    config.zoom.increment_type = ZoomIncrementType::Exponential;
    let mut layout = Layout::<TestWindow>::new(Clock::with_time(Duration::ZERO), &config);
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor = Point::from((500.0, 400.0));

    layout.zoom_set_level(&output, 2.0, cursor, ZoomMovementMode::CursorFollow, false);
    complete_animations(&mut layout);
    layout.zoom_in(&output, cursor);
    complete_animations(&mut layout);
    assert_eq!(layout.zoom_state_for_output(&output).unwrap().level, 4.0);

    layout.zoom_out(&output, cursor);
    complete_animations(&mut layout);
    assert_eq!(layout.zoom_state_for_output(&output).unwrap().level, 2.0);
}

#[test]
fn zoom_out_from_minimum_is_a_noop() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_out(&output, Point::from((500.0, 400.0)));

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert_eq!(state.level, 1.0);
    assert!(!state.transitioning());
}

#[test]
fn zoom_in_at_maximum_is_a_noop() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor = Point::from((500.0, 400.0));

    layout.zoom_set_level(&output, 10.0, cursor, ZoomMovementMode::CursorFollow, false);
    complete_animations(&mut layout);
    layout.zoom_in(&output, cursor);

    let state = layout.zoom_state_for_output(&output).unwrap();
    assert_eq!(state.level, 10.0);
    assert!(!state.transitioning());
}

#[test]
fn setting_current_zoom_level_does_not_start_transition() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);
    let cursor = Point::from((500.0, 400.0));

    layout.zoom_set_level(&output, 2.0, cursor, ZoomMovementMode::CursorFollow, false);
    complete_animations(&mut layout);
    layout.zoom_set_level(&output, 2.0, cursor, ZoomMovementMode::CursorFollow, false);

    assert!(!layout
        .zoom_state_for_output(&output)
        .unwrap()
        .transitioning());
}

#[test]
fn zoom_controls_ignore_unknown_output() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("unknown", 1920, 1080);

    layout.zoom_in(&output, Point::from((1.0, 1.0)));
    layout.zoom_out(&output, Point::from((1.0, 1.0)));
    layout.zoom_set_level(
        &output,
        2.0,
        Point::from((1.0, 1.0)),
        ZoomMovementMode::CursorFollow,
        false,
    );

    assert!(!layout.set_zoom_lock(&output, true));
    assert!(layout.zoom_state_for_output(&output).is_none());
}

#[test]
fn mutable_zoom_state_accessor_updates_state() {
    let mut layout = Layout::<TestWindow>::default();
    let output = make_output("o1", 1920, 1080);
    layout.add_output(output.clone(), None);

    layout.zoom_state_for_output_mut(&output).unwrap().locked = true;

    assert!(layout.zoom_state_for_output(&output).unwrap().locked);
}
