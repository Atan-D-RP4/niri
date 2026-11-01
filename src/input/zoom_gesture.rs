use std::collections::HashMap;

use smithay::backend::input::TouchSlot;
use smithay::output::Output;
use smithay::utils::{Logical, Point};

use super::{GESTURE_ACTIVATION_THRESHOLD, ZOOM_TOUCH_PINCH_FINGERS};

/// Physical input ownership is independent of an output's zoom motion.
#[derive(Debug, Default)]
pub struct ZoomInputState {
    touchpad: TouchpadRoute,
    touchscreen: TouchscreenSequence,
}

#[derive(Debug, Clone, Default)]
pub enum TouchpadRoute {
    #[default]
    Idle,
    Forwarded,
    Consumed {
        output: Output,
    },
}

#[derive(Debug, Clone)]
pub enum ZoomGestureSource {
    TouchpadPinch,
    TouchscreenPinch { initial_distance: f64 },
    TouchscreenPan { last_midpoint: Point<f64, Logical> },
}

#[derive(Debug, Default)]
struct TouchscreenSequence {
    /// Global screen positions, not the inverse-transformed positions sent to clients.
    contacts: HashMap<TouchSlot, Point<f64, Logical>>,
    phase: TouchscreenPhase,
}

#[derive(Debug, Default)]
enum TouchscreenPhase {
    #[default]
    Eligible,
    Pending {
        output: Output,
        initial_distance: f64,
        initial_midpoint: Point<f64, Logical>,
        last_midpoint: Point<f64, Logical>,
    },
    Pinching {
        output: Output,
        initial_distance: f64,
    },
    Panning {
        output: Output,
        last_midpoint: Point<f64, Logical>,
    },
    Draining {
        retained_owner: Option<Output>,
    },
}

/// End zoom motion now; release routing ownership only when the contacts are empty.
pub struct TouchscreenEnd {
    pub output: Output,
    pub released: bool,
}

#[derive(Debug, Clone)]
pub enum TouchscreenGestureUpdate {
    Pinch {
        output: Output,
        initial_distance: f64,
    },
    Pan {
        output: Output,
        pan_delta: Point<f64, Logical>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchscreenActivation {
    Pending,
    Pinch,
    Pan,
}

impl ZoomInputState {
    pub fn touchpad_route(&self) -> TouchpadRoute {
        self.touchpad.clone()
    }

    /// The single admission point for consumed ownership across both sources.
    /// A failed touchscreen admission stays rejected until the contact sequence ends.
    pub fn begin_zoom_gesture(
        &mut self,
        output: &Output,
        source: ZoomGestureSource,
        begin_motion: impl FnOnce() -> bool,
    ) -> bool {
        let available = !self.has_consumed_owner();
        match source {
            ZoomGestureSource::TouchpadPinch => {
                if !matches!(self.touchpad, TouchpadRoute::Idle) {
                    return false;
                }
                if matches!(self.touchscreen.phase, TouchscreenPhase::Pending { .. }) {
                    self.drain_touchscreen();
                }
                if available && begin_motion() {
                    self.touchpad = TouchpadRoute::Consumed {
                        output: output.clone(),
                    };
                    true
                } else {
                    self.touchpad = TouchpadRoute::Forwarded;
                    false
                }
            }
            ZoomGestureSource::TouchscreenPinch { initial_distance } => self
                .begin_touchscreen_gesture(
                    output,
                    TouchscreenPhase::Pinching {
                        output: output.clone(),
                        initial_distance,
                    },
                    available,
                    begin_motion,
                ),
            ZoomGestureSource::TouchscreenPan { last_midpoint } => self.begin_touchscreen_gesture(
                output,
                TouchscreenPhase::Panning {
                    output: output.clone(),
                    last_midpoint,
                },
                available,
                begin_motion,
            ),
        }
    }

    fn begin_touchscreen_gesture(
        &mut self,
        output: &Output,
        phase: TouchscreenPhase,
        available: bool,
        begin_motion: impl FnOnce() -> bool,
    ) -> bool {
        if !matches!(
            &self.touchscreen.phase,
            TouchscreenPhase::Pending {
                output: candidate_output,
                ..
            } if candidate_output == output
        ) || self.touchscreen.contacts.len() != ZOOM_TOUCH_PINCH_FINGERS
        {
            return false;
        }
        if available && begin_motion() {
            self.touchscreen.phase = phase;
            true
        } else {
            self.drain_touchscreen();
            false
        }
    }

    pub fn forward_touchpad_begin(&mut self) {
        if matches!(self.touchpad, TouchpadRoute::Idle) {
            self.touchpad = TouchpadRoute::Forwarded;
        }
    }

    pub fn end_touchpad(&mut self) -> TouchpadRoute {
        std::mem::take(&mut self.touchpad)
    }

    pub fn has_consumed_owner(&self) -> bool {
        matches!(self.touchpad, TouchpadRoute::Consumed { .. })
            || self.touchscreen.owner().is_some()
    }

    pub fn touchscreen_pinch_active(&self) -> bool {
        matches!(self.touchscreen.phase, TouchscreenPhase::Pinching { .. })
    }

    pub fn touchscreen_contacts_empty(&self) -> bool {
        self.touchscreen.contacts.is_empty()
    }

    pub fn touchscreen_gesture_owned(&self) -> bool {
        self.touchscreen.owner().is_some()
    }

    /// Pair order is irrelevant: consumers use a symmetric distance/midpoint and check both.
    pub fn touchscreen_pair(&self) -> Option<[Point<f64, Logical>; 2]> {
        if matches!(self.touchscreen.phase, TouchscreenPhase::Draining { .. })
            || self.touchscreen.contacts.len() != ZOOM_TOUCH_PINCH_FINGERS
        {
            return None;
        }
        let mut positions = self.touchscreen.contacts.values().copied();
        Some([positions.next()?, positions.next()?])
    }

    pub fn update_touchscreen_gesture(
        &mut self,
        midpoint: Point<f64, Logical>,
    ) -> Option<TouchscreenGestureUpdate> {
        match &mut self.touchscreen.phase {
            TouchscreenPhase::Pinching {
                output,
                initial_distance,
            } => Some(TouchscreenGestureUpdate::Pinch {
                output: output.clone(),
                initial_distance: *initial_distance,
            }),
            TouchscreenPhase::Panning {
                output,
                last_midpoint,
            } => {
                let pan_delta = midpoint - *last_midpoint;
                *last_midpoint = midpoint;
                Some(TouchscreenGestureUpdate::Pan {
                    output: output.clone(),
                    pan_delta,
                })
            }
            _ => None,
        }
    }

    pub fn touchscreen_candidate(
        &self,
    ) -> Option<(Output, f64, Point<f64, Logical>, Point<f64, Logical>)> {
        match &self.touchscreen.phase {
            TouchscreenPhase::Pending {
                output,
                initial_distance,
                initial_midpoint,
                last_midpoint,
            } => Some((
                output.clone(),
                *initial_distance,
                *initial_midpoint,
                *last_midpoint,
            )),
            _ => None,
        }
    }

    pub fn begin_touchscreen_candidate(
        &mut self,
        output: &Output,
        initial_distance: f64,
        midpoint: Point<f64, Logical>,
    ) {
        if matches!(self.touchscreen.phase, TouchscreenPhase::Eligible)
            && self.touchscreen.contacts.len() == ZOOM_TOUCH_PINCH_FINGERS
            && initial_distance.is_finite()
            && midpoint.x.is_finite()
            && midpoint.y.is_finite()
        {
            self.touchscreen.phase = TouchscreenPhase::Pending {
                output: output.clone(),
                initial_distance,
                initial_midpoint: midpoint,
                last_midpoint: midpoint,
            };
        }
    }

    pub fn update_touchscreen_candidate(&mut self, midpoint: Point<f64, Logical>) {
        if let TouchscreenPhase::Pending { last_midpoint, .. } = &mut self.touchscreen.phase {
            *last_midpoint = midpoint;
        }
    }

    pub fn touch_down(
        &mut self,
        slot: TouchSlot,
        screen: Point<f64, Logical>,
    ) -> Option<TouchscreenEnd> {
        self.touchscreen.contacts.insert(slot, screen);
        if self.touchscreen.contacts.len() > ZOOM_TOUCH_PINCH_FINGERS {
            self.drain_touchscreen()
        } else {
            None
        }
    }

    pub fn touch_motion(&mut self, slot: TouchSlot, screen: Point<f64, Logical>) {
        if let Some(contact) = self.touchscreen.contacts.get_mut(&slot) {
            *contact = screen;
        }
    }

    pub fn touch_up(&mut self, slot: TouchSlot) -> Option<TouchscreenEnd> {
        self.touchscreen.contacts.remove(&slot);
        if self.touchscreen.contacts.is_empty() {
            self.reset_touchscreen()
        } else if self.touchscreen.contacts.len() < ZOOM_TOUCH_PINCH_FINGERS {
            self.drain_touchscreen()
        } else {
            None
        }
    }

    pub fn cancel_touchscreen(&mut self) -> Option<TouchscreenEnd> {
        self.touchscreen.contacts.clear();
        self.reset_touchscreen()
    }

    /// Rejection or interrupted motion cannot make remaining contacts eligible again.
    pub fn drain_touchscreen(&mut self) -> Option<TouchscreenEnd> {
        if self.touchscreen.contacts.is_empty() {
            return self.reset_touchscreen();
        }
        if matches!(self.touchscreen.phase, TouchscreenPhase::Draining { .. }) {
            return None;
        }
        let retained_owner = self.touchscreen.owner().cloned();
        self.touchscreen.phase = TouchscreenPhase::Draining {
            retained_owner: retained_owner.clone(),
        };
        retained_owner.map(|output| TouchscreenEnd {
            output,
            released: false,
        })
    }

    fn reset_touchscreen(&mut self) -> Option<TouchscreenEnd> {
        debug_assert!(self.touchscreen.contacts.is_empty());
        let owner = self.touchscreen.owner().cloned();
        self.touchscreen.phase = TouchscreenPhase::Eligible;
        owner.map(|output| TouchscreenEnd {
            output,
            released: true,
        })
    }
}

impl TouchscreenSequence {
    fn owner(&self) -> Option<&Output> {
        match &self.phase {
            TouchscreenPhase::Pinching { output, .. }
            | TouchscreenPhase::Panning { output, .. } => Some(output),
            TouchscreenPhase::Draining { retained_owner } => retained_owner.as_ref(),
            TouchscreenPhase::Eligible | TouchscreenPhase::Pending { .. } => None,
        }
    }
}

pub fn touchscreen_activation(
    initial_distance: f64,
    distance: f64,
    initial_midpoint: Point<f64, Logical>,
    midpoint: Point<f64, Logical>,
) -> TouchscreenActivation {
    let pinch = (distance - initial_distance).abs() >= GESTURE_ACTIVATION_THRESHOLD;
    let midpoint_delta = midpoint - initial_midpoint;
    let pan = midpoint_delta.x.hypot(midpoint_delta.y) >= GESTURE_ACTIVATION_THRESHOLD;

    if pinch {
        TouchscreenActivation::Pinch
    } else if pan {
        TouchscreenActivation::Pan
    } else {
        TouchscreenActivation::Pending
    }
}
