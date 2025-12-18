# Animation System Specification

## Overview

The niri-ui animation system integrates with niri's existing `Animation` and `Clock` types to provide smooth, consistent animations across all UI elements. It supports easing curves, spring physics, and deceleration-based animations.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Animation System                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌───────────────────┐                                             │
│   │      Clock        │  Shared time source, supports rate scaling  │
│   └─────────┬─────────┘                                             │
│             │                                                       │
│             ▼                                                       │
│   ┌───────────────────┐                                             │
│   │    Animation      │  Core animation with from/to/duration       │
│   │                   │  Supports: Easing, Spring, Deceleration     │
│   └─────────┬─────────┘                                             │
│             │                                                       │
│             ▼                                                       │
│   ┌───────────────────┐                                             │
│   │  AnimationState   │  State machine for UI visibility            │
│   │  (UI-specific)    │  Hidden → Showing → Shown → Hiding          │
│   └─────────┬─────────┘                                             │
│             │                                                       │
│             ▼                                                       │
│   ┌───────────────────┐                                             │
│   │   Widget/Window   │  Uses animation values for rendering        │
│   └───────────────────┘                                             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Types (from niri)

### Clock

The `Clock` type provides a shared time source that supports rate scaling for testing and debugging.

```rust
// From src/animation/clock.rs
pub struct Clock {
    // Internal implementation
}

impl Clock {
    /// Returns the current time.
    pub fn now(&self) -> Duration;
    
    /// Returns the animation rate multiplier.
    pub fn rate(&self) -> f64;
    
    /// Returns true if animations should complete instantly.
    pub fn should_complete_instantly(&self) -> bool;
}
```

### Animation

The core animation type supporting multiple animation kinds.

```rust
// From src/animation/mod.rs
pub struct Animation {
    from: f64,
    to: f64,
    initial_velocity: f64,
    is_off: bool,
    duration: Duration,
    clamped_duration: Duration,
    start_time: Duration,
    clock: Clock,
    kind: Kind,
}

enum Kind {
    Easing { curve: Curve },
    Spring(Spring),
    Deceleration { initial_velocity: f64, deceleration_rate: f64 },
}

pub enum Curve {
    Linear,
    EaseOutQuad,
    EaseOutCubic,
    EaseOutExpo,
    CubicBezier(CubicBezier),
}
```

### Key Animation Methods

```rust
impl Animation {
    /// Creates a new animation with config.
    pub fn new(
        clock: Clock,
        from: f64,
        to: f64,
        initial_velocity: f64,
        config: niri_config::Animation,
    ) -> Self;
    
    /// Creates an easing animation.
    pub fn ease(
        clock: Clock,
        from: f64,
        to: f64,
        initial_velocity: f64,
        duration_ms: u64,
        curve: Curve,
    ) -> Self;
    
    /// Creates a spring animation.
    pub fn spring(clock: Clock, spring: Spring) -> Self;
    
    /// Creates a deceleration animation (for kinetic scrolling).
    pub fn decelerate(
        clock: Clock,
        from: f64,
        initial_velocity: f64,
        deceleration_rate: f64,
        threshold: f64,
    ) -> Self;
    
    /// Returns true if the animation has completed.
    pub fn is_done(&self) -> bool;
    
    /// Returns the current animated value.
    pub fn value(&self) -> f64;
    
    /// Returns the value clamped to the target after reaching it.
    pub fn clamped_value(&self) -> f64;
    
    /// Returns the target value.
    pub fn to(&self) -> f64;
    
    /// Returns the starting value.
    pub fn from(&self) -> f64;
    
    /// Creates a restarted animation with new from/to values.
    pub fn restarted(&self, from: f64, to: f64, initial_velocity: f64) -> Self;
}
```

## UI Animation State Machine

For UI elements with show/hide animations, use a state machine pattern:

```rust
use std::time::Duration;
use crate::animation::{Animation, Clock};

/// Animation state for UI elements with show/hide transitions.
#[derive(Debug)]
pub enum AnimationState {
    /// Element is hidden and not rendering.
    Hidden,
    /// Element is animating into view.
    Showing(Animation),
    /// Element is fully visible, optionally with auto-hide timer.
    Shown {
        /// Time when auto-hide should start, if any.
        hide_at: Option<Duration>,
    },
    /// Element is animating out of view.
    Hiding(Animation),
}

impl AnimationState {
    /// Returns true if the element should be rendered.
    pub fn is_visible(&self) -> bool {
        !matches!(self, AnimationState::Hidden)
    }
    
    /// Returns the current opacity (0.0 to 1.0).
    pub fn opacity(&self) -> f64 {
        match self {
            AnimationState::Hidden => 0.0,
            AnimationState::Showing(anim) => anim.clamped_value(),
            AnimationState::Shown { .. } => 1.0,
            AnimationState::Hiding(anim) => 1.0 - anim.clamped_value(),
        }
    }
    
    /// Updates the state based on current time.
    /// Returns true if a redraw is needed.
    pub fn tick(&mut self, clock: &Clock) -> bool {
        match self {
            AnimationState::Hidden => false,
            
            AnimationState::Showing(anim) => {
                if anim.is_done() {
                    *self = AnimationState::Shown { hide_at: None };
                }
                true // Always redraw during animation
            }
            
            AnimationState::Shown { hide_at } => {
                if let Some(time) = hide_at {
                    if clock.now() >= *time {
                        // Start hiding - caller should transition to Hiding
                        return true;
                    }
                }
                false
            }
            
            AnimationState::Hiding(anim) => {
                if anim.is_done() {
                    *self = AnimationState::Hidden;
                    return true; // Final redraw to clear
                }
                true
            }
        }
    }
}
```

## Animated Properties

For animating individual properties within widgets:

```rust
/// An animated property that smoothly transitions between values.
pub struct AnimatedProperty {
    animation: Option<Animation>,
    current: f64,
}

impl AnimatedProperty {
    pub fn new(initial: f64) -> Self {
        Self {
            animation: None,
            current: initial,
        }
    }
    
    /// Sets a new target value with animation.
    pub fn animate_to(&mut self, clock: Clock, target: f64, duration_ms: u64) {
        if (self.current - target).abs() < f64::EPSILON {
            return;
        }
        
        let anim = Animation::ease(
            clock,
            self.current,
            target,
            0.0,
            duration_ms,
            Curve::EaseOutCubic,
        );
        self.animation = Some(anim);
    }
    
    /// Sets a new target value instantly.
    pub fn set(&mut self, value: f64) {
        self.animation = None;
        self.current = value;
    }
    
    /// Returns the current animated value.
    pub fn value(&self) -> f64 {
        self.animation
            .as_ref()
            .map(|a| a.clamped_value())
            .unwrap_or(self.current)
    }
    
    /// Returns true if currently animating.
    pub fn is_animating(&self) -> bool {
        self.animation
            .as_ref()
            .map(|a| !a.is_done())
            .unwrap_or(false)
    }
    
    /// Updates internal state after animation completes.
    pub fn tick(&mut self) {
        if let Some(anim) = &self.animation {
            if anim.is_done() {
                self.current = anim.to();
                self.animation = None;
            }
        }
    }
}
```

## Lua Animation API

```lua
-- Animation configuration in Lua
local panel = niri.ui.panel({
    -- Animations are configured per-property or globally
    animations = {
        show = {
            kind = "spring",
            damping_ratio = 0.8,
            stiffness = 400,
            epsilon = 0.001,
        },
        hide = {
            kind = "easing",
            duration_ms = 200,
            curve = "ease-out-cubic",
        },
    },
})

-- Trigger show animation
panel:show()

-- Trigger hide animation  
panel:hide()

-- Animate a specific property
local label = niri.ui.label({ text = "Hello" })
label:animate("opacity", 0.5, { duration_ms = 300, curve = "ease-out-quad" })
label:animate("scale", 1.2, { kind = "spring", stiffness = 300 })
```

## Animation Curves Reference

| Curve | Description | Use Case |
|-------|-------------|----------|
| `Linear` | Constant rate | Progress bars, timers |
| `EaseOutQuad` | Quadratic deceleration | General UI transitions |
| `EaseOutCubic` | Cubic deceleration | Smoother transitions |
| `EaseOutExpo` | Exponential deceleration | Dramatic reveals |
| `CubicBezier` | Custom bezier curve | Fine-tuned animations |
| `Spring` | Physics-based oscillation | Natural, bouncy feel |
| `Deceleration` | Velocity-based slowdown | Kinetic scrolling |

## Acceptance Criteria

### AC1: Easing Animation
```
GIVEN an Animation with curve=EaseOutCubic, from=0.0, to=1.0, duration=300ms
WHEN 150ms has elapsed
THEN value() returns approximately 0.875 (ease-out-cubic at 50%)
AND is_done() returns false
```

### AC2: Animation Completion
```
GIVEN an Animation with duration=200ms
WHEN 200ms or more has elapsed
THEN is_done() returns true
AND value() returns exactly the target value
```

### AC3: State Machine Transitions
```
GIVEN AnimationState::Hidden
WHEN show() is called
THEN state transitions to AnimationState::Showing(animation)
AND opacity() returns 0.0 initially
AND opacity() increases over time
AND state transitions to Shown when animation completes
```

### AC4: Spring Animation
```
GIVEN a Spring animation with damping_ratio=0.5, stiffness=300
WHEN the animation runs
THEN the value may overshoot the target
AND clamped_value() never exceeds the target
AND the animation eventually settles at the target
```

### AC5: Instant Completion
```
GIVEN a Clock with should_complete_instantly()=true
WHEN any Animation is created
THEN is_done() immediately returns true
AND value() returns the target value
```

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_clock() -> Clock {
        // Create a test clock that can be manually advanced
        Clock::new_for_test()
    }
    
    #[test]
    fn test_easing_animation() {
        let clock = test_clock();
        let anim = Animation::ease(clock.clone(), 0.0, 100.0, 0.0, 1000, Curve::Linear);
        
        // At start
        assert!(!anim.is_done());
        assert_eq!(anim.value(), 0.0);
        
        // Advance to middle
        clock.advance(Duration::from_millis(500));
        assert!((anim.value() - 50.0).abs() < 0.1);
        
        // Advance past end
        clock.advance(Duration::from_millis(600));
        assert!(anim.is_done());
        assert_eq!(anim.value(), 100.0);
    }
    
    #[test]
    fn test_animation_state_machine() {
        let clock = test_clock();
        let mut state = AnimationState::Hidden;
        
        assert!(!state.is_visible());
        assert_eq!(state.opacity(), 0.0);
        
        // Start showing
        let show_anim = Animation::ease(clock.clone(), 0.0, 1.0, 0.0, 200, Curve::EaseOutCubic);
        state = AnimationState::Showing(show_anim);
        
        assert!(state.is_visible());
        
        // Complete animation
        clock.advance(Duration::from_millis(250));
        state.tick(&clock);
        
        assert!(matches!(state, AnimationState::Shown { .. }));
        assert_eq!(state.opacity(), 1.0);
    }
    
    #[test]
    fn test_animated_property() {
        let clock = test_clock();
        let mut prop = AnimatedProperty::new(0.0);
        
        prop.animate_to(clock.clone(), 100.0, 500);
        assert!(prop.is_animating());
        
        clock.advance(Duration::from_millis(600));
        prop.tick();
        
        assert!(!prop.is_animating());
        assert_eq!(prop.value(), 100.0);
    }
}
```

### Integration Tests

- Test animations with real compositor clock
- Test animation interruption (starting new animation mid-flight)
- Test multiple simultaneous animations
- Test animation with config hot-reload

## Performance Considerations

1. **Avoid allocations**: Reuse Animation structs when possible via `restarted()`
2. **Clamped values**: Use `clamped_value()` for visual properties to prevent overshoot
3. **Early termination**: Check `is_done()` to skip unnecessary calculations
4. **Rate scaling**: Respect `Clock::rate()` for slow-motion debugging
5. **Batch updates**: Tick multiple animations in a single frame update

## References

- `src/animation/mod.rs` - Core Animation type
- `src/animation/clock.rs` - Clock type
- `src/animation/spring.rs` - Spring physics
- `src/animation/bezier.rs` - Cubic bezier curves
