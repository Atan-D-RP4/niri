//! Lua types module for complex type definitions and UserData implementations.
//!
//! This module defines complex types that can be used in Lua configuration,
//! providing UserData implementations for type safety and validation.

use mlua::prelude::*;
use mlua::{UserData, UserDataMethods};
use regex::Regex;

/// Animation configuration type with validation.
#[derive(Debug, Clone)]
pub struct LuaAnimation {
    pub name: String,
    pub duration_ms: i32,
    pub curve: String,
}

impl LuaAnimation {
    pub fn new(name: String, duration_ms: i32, curve: String) -> LuaResult<Self> {
        if duration_ms <= 0 || duration_ms > 5000 {
            return Err(mlua::Error::RuntimeError(format!(
                "Duration must be between 1 and 5000 ms, got {}",
                duration_ms
            )));
        }

        let valid_curves = ["linear", "ease_in_out_cubic", "ease_out_cubic"];
        if !valid_curves.contains(&curve.as_str()) {
            return Err(mlua::Error::RuntimeError(
                format!("Unknown animation curve '{}'. Valid curves: linear, ease_in_out_cubic, ease_out_cubic", curve),
            ));
        }

        Ok(LuaAnimation {
            name,
            duration_ms,
            curve,
        })
    }
}

impl UserData for LuaAnimation {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_name", |_, this, ()| Ok(this.name.clone()));
        methods.add_method("get_duration", |_, this, ()| Ok(this.duration_ms));
        methods.add_method("get_curve", |_, this, ()| Ok(this.curve.clone()));

        methods.add_method("with_duration", |_, this, ms: i32| {
            if ms <= 0 || ms > 5000 {
                return Err(mlua::Error::RuntimeError(format!(
                    "Duration must be between 1 and 5000 ms, got {}",
                    ms
                )));
            }
            let mut new_anim = this.clone();
            new_anim.duration_ms = ms;
            Ok(new_anim)
        });

        methods.add_method("with_curve", |_, this, curve: String| {
            let valid_curves = ["linear", "ease_in_out_cubic", "ease_out_cubic"];
            if !valid_curves.contains(&curve.as_str()) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Unknown curve: {}",
                    curve
                )));
            }
            let mut new_anim = this.clone();
            new_anim.curve = curve;
            Ok(new_anim)
        });
    }
}

/// Filter for matching windows by app_id and title.
#[derive(Debug, Clone)]
pub struct LuaFilter {
    pub match_app_id: Option<String>,
    pub match_title: Option<String>,
    pub regex_app_id: Option<Regex>,
    pub regex_title: Option<Regex>,
}

impl LuaFilter {
    pub fn new(
        match_app_id: Option<String>,
        match_title: Option<String>,
        regex_app_id: Option<String>,
        regex_title: Option<String>,
    ) -> LuaResult<Self> {
        if match_app_id.is_none()
            && match_title.is_none()
            && regex_app_id.is_none()
            && regex_title.is_none()
        {
            return Err(mlua::Error::RuntimeError(
                "Filter must have at least one condition".to_string(),
            ));
        }

        let regex_app_id = regex_app_id
            .map(|r| Regex::new(&r))
            .transpose()
            .map_err(|e| mlua::Error::RuntimeError(format!("Invalid regex for app_id: {}", e)))?;

        let regex_title = regex_title
            .map(|r| Regex::new(&r))
            .transpose()
            .map_err(|e| mlua::Error::RuntimeError(format!("Invalid regex for title: {}", e)))?;

        Ok(LuaFilter {
            match_app_id,
            match_title,
            regex_app_id,
            regex_title,
        })
    }

    pub fn matches(&self, app_id: &str, title: &str) -> bool {
        if let Some(ref regex) = self.regex_app_id {
            if !regex.is_match(app_id) {
                return false;
            }
        } else if let Some(ref match_app) = self.match_app_id {
            if app_id != match_app {
                return false;
            }
        }

        if let Some(ref regex) = self.regex_title {
            if !regex.is_match(title) {
                return false;
            }
        } else if let Some(ref match_title) = self.match_title {
            if !title.contains(match_title) {
                return false;
            }
        }

        true
    }
}

impl UserData for LuaFilter {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("matches", |_, this, (app_id, title): (String, String)| {
            Ok(this.matches(&app_id, &title))
        });

        methods.add_method("get_app_id", |_, this, ()| Ok(this.match_app_id.clone()));
        methods.add_method("get_title", |_, this, ()| Ok(this.match_title.clone()));
    }
}

/// Window rule type with filter and actions.
#[derive(Debug, Clone)]
pub struct LuaWindowRule {
    pub filter: LuaFilter,
    pub floating: Option<bool>,
    pub fullscreen: Option<bool>,
    pub tile: Option<bool>,
}

impl LuaWindowRule {
    pub fn new(filter: LuaFilter) -> Self {
        LuaWindowRule {
            filter,
            floating: None,
            fullscreen: None,
            tile: None,
        }
    }

    pub fn matches(&self, app_id: &str, title: &str) -> bool {
        self.filter.matches(app_id, title)
    }
}

impl UserData for LuaWindowRule {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("matches", |_, this, (app_id, title): (String, String)| {
            Ok(this.matches(&app_id, &title))
        });

        methods.add_method("get_floating", |_, this, ()| Ok(this.floating));
        methods.add_method("get_fullscreen", |_, this, ()| Ok(this.fullscreen));
        methods.add_method("get_tile", |_, this, ()| Ok(this.tile));

        methods.add_method("with_floating", |_, this, floating: bool| {
            let mut new_rule = this.clone();
            new_rule.floating = Some(floating);
            Ok(new_rule)
        });

        methods.add_method("with_fullscreen", |_, this, fullscreen: bool| {
            let mut new_rule = this.clone();
            new_rule.fullscreen = Some(fullscreen);
            Ok(new_rule)
        });

        methods.add_method("with_tile", |_, this, tile: bool| {
            let mut new_rule = this.clone();
            new_rule.tile = Some(tile);
            Ok(new_rule)
        });
    }
}

/// Gesture type for gesture configuration.
#[derive(Debug, Clone)]
pub struct LuaGesture {
    pub gesture_type: String,
    pub fingers: u32,
    pub direction: Option<String>,
    pub action: String,
}

impl LuaGesture {
    pub fn new(gesture_type: String, fingers: u32, action: String) -> LuaResult<Self> {
        let valid_types = ["swipe", "pinch", "hold"];
        if !valid_types.contains(&gesture_type.as_str()) {
            return Err(mlua::Error::RuntimeError(format!(
                "Unknown gesture type: {}. Valid types: swipe, pinch, hold",
                gesture_type
            )));
        }

        if fingers == 0 || fingers > 10 {
            return Err(mlua::Error::RuntimeError(format!(
                "Fingers must be between 1 and 10, got {}",
                fingers
            )));
        }

        Ok(LuaGesture {
            gesture_type,
            fingers,
            direction: None,
            action,
        })
    }
}

impl UserData for LuaGesture {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_type", |_, this, ()| Ok(this.gesture_type.clone()));
        methods.add_method("get_fingers", |_, this, ()| Ok(this.fingers));
        methods.add_method("get_direction", |_, this, ()| Ok(this.direction.clone()));
        methods.add_method("get_action", |_, this, ()| Ok(this.action.clone()));

        methods.add_method("with_direction", |_, this, direction: String| {
            let valid_directions = ["up", "down", "left", "right"];
            if !valid_directions.contains(&direction.as_str()) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Invalid direction: {}",
                    direction
                )));
            }
            let mut new_gesture = this.clone();
            new_gesture.direction = Some(direction);
            Ok(new_gesture)
        });

        methods.add_method("with_action", |_, this, action: String| {
            let mut new_gesture = this.clone();
            new_gesture.action = action;
            Ok(new_gesture)
        });
    }
}

/// Helper function to create animation from Lua table
pub fn animation_from_table(_lua: &Lua, table: &mlua::Table) -> LuaResult<LuaAnimation> {
    let name = table.get::<String>("name")?;
    let duration_ms = table.get::<i32>("duration")?;
    let curve = table
        .get::<Option<String>>("curve")?
        .unwrap_or_else(|| "ease_in_out_cubic".to_string());

    LuaAnimation::new(name, duration_ms, curve)
}

/// Helper function to create filter from Lua table
pub fn filter_from_table(_lua: &Lua, table: &mlua::Table) -> LuaResult<LuaFilter> {
    let match_app_id = table.get::<Option<String>>("match_app_id")?;
    let match_title = table.get::<Option<String>>("match_title")?;
    let regex_app_id = table.get::<Option<String>>("regex_app_id")?;
    let regex_title = table.get::<Option<String>>("regex_title")?;

    LuaFilter::new(match_app_id, match_title, regex_app_id, regex_title)
}

/// Helper function to create window rule from Lua table
pub fn window_rule_from_table(lua: &Lua, table: &mlua::Table) -> LuaResult<LuaWindowRule> {
    let filter_table = table.get::<mlua::Table>("filter")?;
    let filter = filter_from_table(lua, &filter_table)?;

    let mut rule = LuaWindowRule::new(filter);

    if let Ok(floating) = table.get::<bool>("floating") {
        rule.floating = Some(floating);
    }
    if let Ok(fullscreen) = table.get::<bool>("fullscreen") {
        rule.fullscreen = Some(fullscreen);
    }
    if let Ok(tile) = table.get::<bool>("tile") {
        rule.tile = Some(tile);
    }

    Ok(rule)
}

/// Helper function to create gesture from Lua table
pub fn gesture_from_table(_lua: &Lua, table: &mlua::Table) -> LuaResult<LuaGesture> {
    let gesture_type = table.get::<String>("gesture_type")?;
    let fingers = table.get::<u32>("fingers")?;
    let action = table.get::<String>("action")?;

    let mut gesture = LuaGesture::new(gesture_type, fingers, action)?;

    if let Ok(direction) = table.get::<String>("direction") {
        gesture.direction = Some(direction);
    }

    Ok(gesture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_creation() {
        let anim = LuaAnimation::new("test".to_string(), 100, "linear".to_string());
        assert!(anim.is_ok());
        let anim = anim.unwrap();
        assert_eq!(anim.name, "test");
        assert_eq!(anim.duration_ms, 100);
    }

    #[test]
    fn animation_invalid_duration() {
        let anim = LuaAnimation::new("test".to_string(), 0, "linear".to_string());
        assert!(anim.is_err());

        let anim = LuaAnimation::new("test".to_string(), 6000, "linear".to_string());
        assert!(anim.is_err());
    }

    #[test]
    fn animation_invalid_curve() {
        let anim = LuaAnimation::new("test".to_string(), 100, "invalid_curve".to_string());
        assert!(anim.is_err());
    }

    #[test]
    fn filter_creation() {
        let filter = LuaFilter::new(Some("firefox".to_string()), None, None, None);
        assert!(filter.is_ok());
    }

    #[test]
    fn filter_no_conditions() {
        let filter = LuaFilter::new(None, None, None, None);
        assert!(filter.is_err());
    }

    #[test]
    fn filter_matches() {
        let filter = LuaFilter::new(Some("firefox".to_string()), None, None, None).unwrap();
        assert!(filter.matches("firefox", ""));
        assert!(!filter.matches("chrome", ""));
    }

    #[test]
    fn gesture_creation() {
        let gesture = LuaGesture::new("swipe".to_string(), 3, "focus_left".to_string());
        assert!(gesture.is_ok());
    }

    #[test]
    fn gesture_invalid_type() {
        let gesture = LuaGesture::new("invalid".to_string(), 3, "focus_left".to_string());
        assert!(gesture.is_err());
    }

    #[test]
    fn gesture_invalid_fingers() {
        let gesture = LuaGesture::new("swipe".to_string(), 0, "focus_left".to_string());
        assert!(gesture.is_err());

        let gesture = LuaGesture::new("swipe".to_string(), 15, "focus_left".to_string());
        assert!(gesture.is_err());
    }
}
