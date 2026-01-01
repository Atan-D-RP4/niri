use std::path::PathBuf;

use mlua::prelude::{LuaError, LuaValue, *};
use niri_config::debug::PreviewRender;
use niri_config::input::{AccelProfile, ScrollMethod, TapButtonMap, TrackLayout};
use niri_config::utils::FloatOrInt;
use niri_config::CenterFocusedColumn;

use crate::property_registry::{PropertyRegistry, PropertyType};
use crate::traits::parse_color_string;

pub fn register_config_accessors(registry: &mut PropertyRegistry) {
    registry.update_accessor(
        "animations.off",
        |_lua, config| Ok(LuaValue::Boolean(config.animations.off)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.animations.off = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "animations.slowdown",
        |_lua, config| Ok(LuaValue::Number(config.animations.slowdown)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.animations.slowdown = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "prefer_no_csd",
        |_lua, config| Ok(LuaValue::Boolean(config.prefer_no_csd)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.prefer_no_csd = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "cursor.xcursor_theme",
        |_lua, config| {
            Ok(LuaValue::String(
                _lua.create_string(&config.cursor.xcursor_theme)?,
            ))
        },
        |_lua, config, value| {
            let s = String::from_lua(value, _lua)?;
            config.cursor.xcursor_theme = s;
            Ok(())
        },
    );

    registry.update_accessor(
        "cursor.xcursor_size",
        |_lua, config| Ok(LuaValue::Integer(config.cursor.xcursor_size as i64)),
        |_lua, config, value| {
            let v = u8::from_lua(value, _lua)?;
            config.cursor.xcursor_size = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "cursor.hide_when_typing",
        |_lua, config| Ok(LuaValue::Boolean(config.cursor.hide_when_typing)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.cursor.hide_when_typing = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "cursor.hide_after_inactive_ms",
        |_lua, config| match config.cursor.hide_after_inactive_ms {
            Some(v) => Ok(LuaValue::Integer(v as i64)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<u32>::from_lua(value, _lua)?;
            config.cursor.hide_after_inactive_ms = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.repeat_delay",
        |_lua, config| Ok(LuaValue::Integer(config.input.keyboard.repeat_delay as i64)),
        |_lua, config, value| {
            let v = u16::from_lua(value, _lua)?;
            config.input.keyboard.repeat_delay = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.repeat_rate",
        |_lua, config| Ok(LuaValue::Integer(config.input.keyboard.repeat_rate as i64)),
        |_lua, config, value| {
            let v = u8::from_lua(value, _lua)?;
            config.input.keyboard.repeat_rate = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.track_layout",
        |_lua, config| match config.input.keyboard.track_layout {
            TrackLayout::Global => Ok(LuaValue::String(_lua.create_string("global")?)),
            TrackLayout::Window => Ok(LuaValue::String(_lua.create_string("window")?)),
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            config.input.keyboard.track_layout = match v.as_str() {
                "global" => TrackLayout::Global,
                "window" => TrackLayout::Window,
                other => {
                    return Err(LuaError::external(format!(
                        "invalid input.keyboard.track_layout value: {}",
                        other
                    )))
                }
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.xkb.rules",
        |_lua, config| {
            Ok(LuaValue::String(
                _lua.create_string(&config.input.keyboard.xkb.rules)?,
            ))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            config.input.keyboard.xkb.rules = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.xkb.model",
        |_lua, config| {
            Ok(LuaValue::String(
                _lua.create_string(&config.input.keyboard.xkb.model)?,
            ))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            config.input.keyboard.xkb.model = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.xkb.layout",
        |_lua, config| {
            Ok(LuaValue::String(
                _lua.create_string(&config.input.keyboard.xkb.layout)?,
            ))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            config.input.keyboard.xkb.layout = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.xkb.variant",
        |_lua, config| {
            Ok(LuaValue::String(
                _lua.create_string(&config.input.keyboard.xkb.variant)?,
            ))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            config.input.keyboard.xkb.variant = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.xkb.options",
        |_lua, config| match &config.input.keyboard.xkb.options {
            Some(v) => Ok(LuaValue::String(_lua.create_string(v)?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.keyboard.xkb.options = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.keyboard.xkb.file",
        |_lua, config| match &config.input.keyboard.xkb.file {
            Some(v) => Ok(LuaValue::String(_lua.create_string(v)?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.keyboard.xkb.file = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.mouse.natural_scroll",
        |_lua, config| Ok(LuaValue::Boolean(config.input.mouse.natural_scroll)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.mouse.natural_scroll = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.mouse.accel_speed",
        |_lua, config| Ok(LuaValue::Number(config.input.mouse.accel_speed.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.input.mouse.accel_speed = FloatOrInt::<-1, 1>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "input.mouse.accel_profile",
        |_lua, config| match config.input.mouse.accel_profile {
            Some(AccelProfile::Adaptive) => Ok(LuaValue::String(_lua.create_string("adaptive")?)),
            Some(AccelProfile::Flat) => Ok(LuaValue::String(_lua.create_string("flat")?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.mouse.accel_profile = match v.as_deref() {
                Some("adaptive") => Some(AccelProfile::Adaptive),
                Some("flat") => Some(AccelProfile::Flat),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid input.mouse.accel_profile value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "input.mouse.scroll_method",
        |_lua, config| match config.input.mouse.scroll_method {
            Some(ScrollMethod::NoScroll) => Ok(LuaValue::String(_lua.create_string("no_scroll")?)),
            Some(ScrollMethod::TwoFinger) => {
                Ok(LuaValue::String(_lua.create_string("two_finger")?))
            }
            Some(ScrollMethod::Edge) => Ok(LuaValue::String(_lua.create_string("edge")?)),
            Some(ScrollMethod::OnButtonDown) => {
                Ok(LuaValue::String(_lua.create_string("on_button_down")?))
            }
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.mouse.scroll_method = match v
                .as_deref()
                .map(|s| s.replace('-', "_").to_lowercase())
                .as_deref()
            {
                Some("no_scroll") => Some(ScrollMethod::NoScroll),
                Some("two_finger") => Some(ScrollMethod::TwoFinger),
                Some("edge") => Some(ScrollMethod::Edge),
                Some("on_button_down") => Some(ScrollMethod::OnButtonDown),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid input.mouse.scroll_method value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.tap",
        |_lua, config| Ok(LuaValue::Boolean(config.input.touchpad.tap)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.touchpad.tap = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.dwt",
        |_lua, config| Ok(LuaValue::Boolean(config.input.touchpad.dwt)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.touchpad.dwt = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.dwtp",
        |_lua, config| Ok(LuaValue::Boolean(config.input.touchpad.dwtp)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.touchpad.dwtp = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.natural_scroll",
        |_lua, config| Ok(LuaValue::Boolean(config.input.touchpad.natural_scroll)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.touchpad.natural_scroll = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.accel_speed",
        |_lua, config| Ok(LuaValue::Number(config.input.touchpad.accel_speed.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.input.touchpad.accel_speed = FloatOrInt::<-1, 1>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.accel_profile",
        |_lua, config| match config.input.touchpad.accel_profile {
            Some(AccelProfile::Adaptive) => Ok(LuaValue::String(_lua.create_string("adaptive")?)),
            Some(AccelProfile::Flat) => Ok(LuaValue::String(_lua.create_string("flat")?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.touchpad.accel_profile = match v.as_deref() {
                Some("adaptive") => Some(AccelProfile::Adaptive),
                Some("flat") => Some(AccelProfile::Flat),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid input.touchpad.accel_profile value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.tap_button_map",
        |_lua, config| match config.input.touchpad.tap_button_map {
            Some(TapButtonMap::LeftRightMiddle) => {
                Ok(LuaValue::String(_lua.create_string("left_right_middle")?))
            }
            Some(TapButtonMap::LeftMiddleRight) => {
                Ok(LuaValue::String(_lua.create_string("left_middle_right")?))
            }
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.touchpad.tap_button_map = match v
                .as_deref()
                .map(|s| s.replace('-', "_").to_lowercase())
                .as_deref()
            {
                Some("left_right_middle") => Some(TapButtonMap::LeftRightMiddle),
                Some("left_middle_right") => Some(TapButtonMap::LeftMiddleRight),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid input.touchpad.tap_button_map value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touchpad.scroll_method",
        |_lua, config| match config.input.touchpad.scroll_method {
            Some(ScrollMethod::NoScroll) => Ok(LuaValue::String(_lua.create_string("no_scroll")?)),
            Some(ScrollMethod::TwoFinger) => {
                Ok(LuaValue::String(_lua.create_string("two_finger")?))
            }
            Some(ScrollMethod::Edge) => Ok(LuaValue::String(_lua.create_string("edge")?)),
            Some(ScrollMethod::OnButtonDown) => {
                Ok(LuaValue::String(_lua.create_string("on_button_down")?))
            }
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.touchpad.scroll_method = match v
                .as_deref()
                .map(|s| s.replace('-', "_").to_lowercase())
                .as_deref()
            {
                Some("no_scroll") => Some(ScrollMethod::NoScroll),
                Some("two_finger") => Some(ScrollMethod::TwoFinger),
                Some("edge") => Some(ScrollMethod::Edge),
                Some("on_button_down") => Some(ScrollMethod::OnButtonDown),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid input.touchpad.scroll_method value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touch.off",
        |_lua, config| Ok(LuaValue::Boolean(config.input.touch.off)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.touch.off = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touch.natural_scroll",
        |_lua, config| Ok(LuaValue::Boolean(config.input.touch.natural_scroll)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.touch.natural_scroll = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touch.map_to_output",
        |lua, config| match &config.input.touch.map_to_output {
            Some(s) => Ok(LuaValue::String(lua.create_string(s)?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.touch.map_to_output = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.touch.calibration_matrix",
        |lua, config| match &config.input.touch.calibration_matrix {
            Some(v) => {
                let arr = lua.create_table()?;
                for (i, val) in v.iter().enumerate() {
                    arr.set(i + 1, *val as f64)?;
                }
                Ok(LuaValue::Table(arr))
            }
            None => Ok(LuaValue::Nil),
        },
        |lua, config, value| {
            let v: Option<Vec<f64>> = Option::from_lua(value, lua)?;
            config.input.touch.calibration_matrix =
                v.map(|arr| arr.iter().map(|x| *x as f32).collect());
            Ok(())
        },
    );

    registry.update_accessor(
        "input.trackpoint.natural_scroll",
        |_lua, config| Ok(LuaValue::Boolean(config.input.trackpoint.natural_scroll)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.trackpoint.natural_scroll = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.trackpoint.accel_speed",
        |_lua, config| Ok(LuaValue::Number(config.input.trackpoint.accel_speed.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.input.trackpoint.accel_speed = FloatOrInt::<-1, 1>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "input.trackpoint.accel_profile",
        |_lua, config| match config.input.trackpoint.accel_profile {
            Some(AccelProfile::Adaptive) => Ok(LuaValue::String(_lua.create_string("adaptive")?)),
            Some(AccelProfile::Flat) => Ok(LuaValue::String(_lua.create_string("flat")?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.trackpoint.accel_profile = match v.as_deref() {
                Some("adaptive") => Some(AccelProfile::Adaptive),
                Some("flat") => Some(AccelProfile::Flat),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid input.trackpoint.accel_profile value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "input.trackball.natural_scroll",
        |_lua, config| Ok(LuaValue::Boolean(config.input.trackball.natural_scroll)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.input.trackball.natural_scroll = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "input.trackball.accel_speed",
        |_lua, config| Ok(LuaValue::Number(config.input.trackball.accel_speed.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.input.trackball.accel_speed = FloatOrInt::<-1, 1>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "input.trackball.accel_profile",
        |_lua, config| match config.input.trackball.accel_profile {
            Some(AccelProfile::Adaptive) => Ok(LuaValue::String(_lua.create_string("adaptive")?)),
            Some(AccelProfile::Flat) => Ok(LuaValue::String(_lua.create_string("flat")?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.input.trackball.accel_profile = match v.as_deref() {
                Some("adaptive") => Some(AccelProfile::Adaptive),
                Some("flat") => Some(AccelProfile::Flat),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid input.trackball.accel_profile value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "clipboard.disable_primary",
        |_lua, config| Ok(LuaValue::Boolean(config.clipboard.disable_primary)),
        |_lua, config, value| {
            config.clipboard.disable_primary = match value {
                LuaValue::Nil => false,
                LuaValue::Boolean(b) => b,
                _ => return Err(mlua::Error::runtime("expected boolean or nil")),
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "screenshot_path",
        |_lua, config| match &config.screenshot_path.0 {
            Some(path) => Ok(LuaValue::String(_lua.create_string(path)?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.screenshot_path.0 = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "hotkey_overlay.skip_at_startup",
        |_lua, config| Ok(LuaValue::Boolean(config.hotkey_overlay.skip_at_startup)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.hotkey_overlay.skip_at_startup = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "config_notification.disable_failed",
        |_lua, config| Ok(LuaValue::Boolean(config.config_notification.disable_failed)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.config_notification.disable_failed = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.preview_render",
        |_lua, config| match config.debug.preview_render {
            Some(PreviewRender::Screencast) => {
                Ok(LuaValue::String(_lua.create_string("screencast")?))
            }
            Some(PreviewRender::ScreenCapture) => {
                Ok(LuaValue::String(_lua.create_string("screen_capture")?))
            }
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.debug.preview_render = match v.as_deref() {
                Some("screencast") => Some(PreviewRender::Screencast),
                Some("screen_capture") => Some(PreviewRender::ScreenCapture),
                Some(other) => {
                    return Err(LuaError::external(format!(
                        "invalid debug.preview_render value: {}",
                        other
                    )))
                }
                None => None,
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.dbus_interfaces_in_non_session_instances",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.dbus_interfaces_in_non_session_instances,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.dbus_interfaces_in_non_session_instances = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.wait_for_frame_completion_before_queueing",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.wait_for_frame_completion_before_queueing,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.wait_for_frame_completion_before_queueing = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.enable_overlay_planes",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.enable_overlay_planes)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.enable_overlay_planes = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.disable_cursor_plane",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.disable_cursor_plane)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.disable_cursor_plane = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.disable_direct_scanout",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.disable_direct_scanout)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.disable_direct_scanout = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.keep_max_bpc_unchanged",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.keep_max_bpc_unchanged)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.keep_max_bpc_unchanged = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.restrict_primary_scanout_to_matching_format",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.restrict_primary_scanout_to_matching_format,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.restrict_primary_scanout_to_matching_format = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.force_disable_connectors_on_resume",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.force_disable_connectors_on_resume,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.force_disable_connectors_on_resume = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.render_drm_device",
        |_lua, config| match &config.debug.render_drm_device {
            Some(path) => Ok(LuaValue::String(
                _lua.create_string(&*path.to_string_lossy())?,
            )),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            let v = Option::<String>::from_lua(value, _lua)?;
            config.debug.render_drm_device = v.map(PathBuf::from);
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.ignored_drm_devices",
        |_lua, config| {
            let tbl = _lua.create_table()?;
            for (i, dev) in config.debug.ignored_drm_devices.iter().enumerate() {
                tbl.set((i + 1) as i64, dev.to_string_lossy().to_string())?;
            }
            Ok(LuaValue::Table(tbl))
        },
        |_lua, config, value| {
            let devices = match value {
                LuaValue::Nil => Vec::new(),
                other => Vec::<String>::from_lua(other, _lua)?,
            };
            config.debug.ignored_drm_devices = devices.into_iter().map(PathBuf::from).collect();
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.force_pipewire_invalid_modifier",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.force_pipewire_invalid_modifier,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.force_pipewire_invalid_modifier = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.emulate_zero_presentation_time",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.emulate_zero_presentation_time,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.emulate_zero_presentation_time = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.disable_resize_throttling",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.disable_resize_throttling)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.disable_resize_throttling = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.disable_transactions",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.disable_transactions)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.disable_transactions = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.keep_laptop_panel_on_when_lid_is_closed",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.keep_laptop_panel_on_when_lid_is_closed,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.keep_laptop_panel_on_when_lid_is_closed = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.disable_monitor_names",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.disable_monitor_names)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.disable_monitor_names = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.strict_new_window_focus_policy",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.strict_new_window_focus_policy,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.strict_new_window_focus_policy = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.honor_xdg_activation_with_invalid_serial",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.honor_xdg_activation_with_invalid_serial,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.honor_xdg_activation_with_invalid_serial = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.deactivate_unfocused_windows",
        |_lua, config| Ok(LuaValue::Boolean(config.debug.deactivate_unfocused_windows)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.deactivate_unfocused_windows = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "debug.skip_cursor_only_updates_during_vrr",
        |_lua, config| {
            Ok(LuaValue::Boolean(
                config.debug.skip_cursor_only_updates_during_vrr,
            ))
        },
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.debug.skip_cursor_only_updates_during_vrr = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.gaps",
        |_lua, config| Ok(LuaValue::Number(config.layout.gaps)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.gaps = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.center_focused_column",
        |_lua, config| {
            let value = match config.layout.center_focused_column {
                CenterFocusedColumn::Never => "never",
                CenterFocusedColumn::Always => "always",
                CenterFocusedColumn::OnOverflow => "on_overflow",
            };
            Ok(LuaValue::String(_lua.create_string(value)?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            config.layout.center_focused_column = match v.as_str() {
                "never" => CenterFocusedColumn::Never,
                "always" => CenterFocusedColumn::Always,
                "on_overflow" => CenterFocusedColumn::OnOverflow,
                other => {
                    return Err(LuaError::external(format!(
                        "invalid layout.center_focused_column value: {}",
                        other
                    )))
                }
            };
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.struts.left",
        |_lua, config| Ok(LuaValue::Number(config.layout.struts.left.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.struts.left = FloatOrInt::<-65535, 65535>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.struts.right",
        |_lua, config| Ok(LuaValue::Number(config.layout.struts.right.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.struts.right = FloatOrInt::<-65535, 65535>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.struts.top",
        |_lua, config| Ok(LuaValue::Number(config.layout.struts.top.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.struts.top = FloatOrInt::<-65535, 65535>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.struts.bottom",
        |_lua, config| Ok(LuaValue::Number(config.layout.struts.bottom.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.struts.bottom = FloatOrInt::<-65535, 65535>(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.on",
        |_lua, config| Ok(LuaValue::Boolean(config.layout.shadow.on)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.layout.shadow.on = v;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.shadow.off",
        PropertyType::Bool,
        |_lua, config| Ok(LuaValue::Boolean(!config.layout.shadow.on)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.layout.shadow.on = !v;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.softness",
        |_lua, config| Ok(LuaValue::Number(config.layout.shadow.softness)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.shadow.softness = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.spread",
        |_lua, config| Ok(LuaValue::Number(config.layout.shadow.spread)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.shadow.spread = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.draw_behind_window",
        |_lua, config| Ok(LuaValue::Boolean(config.layout.shadow.draw_behind_window)),
        |_lua, config, value| {
            let v = bool::from_lua(value, _lua)?;
            config.layout.shadow.draw_behind_window = v;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.color",
        |_lua, config| {
            Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (config.layout.shadow.color.r * 255.) as u8,
                (config.layout.shadow.color.g * 255.) as u8,
                (config.layout.shadow.color.b * 255.) as u8,
                (config.layout.shadow.color.a * 255.) as u8,
            ))?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            let color = parse_color_string(&v)?;
            config.layout.shadow.color = color;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.inactive_color",
        |_lua, config| match &config.layout.shadow.inactive_color {
            Some(color) => Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ))?)),
            None => Ok(LuaValue::Nil),
        },
        |_lua, config, value| {
            if value.is_nil() {
                config.layout.shadow.inactive_color = None;
            } else {
                let v = String::from_lua(value, _lua)?;
                let color = parse_color_string(&v)?;
                config.layout.shadow.inactive_color = Some(color);
            }
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.offset.x",
        |_lua, config| Ok(LuaValue::Number(config.layout.shadow.offset.x.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.shadow.offset.x = FloatOrInt(v);
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.shadow.offset.y",
        |_lua, config| Ok(LuaValue::Number(config.layout.shadow.offset.y.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.shadow.offset.y = FloatOrInt(v);
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.focus_ring.off",
        PropertyType::Bool,
        |_lua, config| Ok(LuaValue::Boolean(config.layout.focus_ring.off)),
        |_lua, config, value| {
            config.layout.focus_ring.off = bool::from_lua(value, _lua)?;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.focus_ring.width",
        |_lua, config| Ok(LuaValue::Number(config.layout.focus_ring.width)),
        |_lua, config, value| {
            config.layout.focus_ring.width = f64::from_lua(value, _lua)?;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.focus_ring.active_color",
        PropertyType::String,
        |_lua, config| {
            let color = &config.layout.focus_ring.active_color;
            Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ))?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            let color = parse_color_string(&v)?;
            config.layout.focus_ring.active_color = color;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.focus_ring.inactive_color",
        PropertyType::String,
        |_lua, config| {
            let color = &config.layout.focus_ring.inactive_color;
            Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ))?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            let color = parse_color_string(&v)?;
            config.layout.focus_ring.inactive_color = color;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.focus_ring.urgent_color",
        PropertyType::String,
        |_lua, config| {
            let color = &config.layout.focus_ring.urgent_color;
            Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ))?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            let color = parse_color_string(&v)?;
            config.layout.focus_ring.urgent_color = color;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.border.off",
        PropertyType::Bool,
        |_lua, config| Ok(LuaValue::Boolean(config.layout.border.off)),
        |_lua, config, value| {
            config.layout.border.off = bool::from_lua(value, _lua)?;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.border.width",
        |_lua, config| Ok(LuaValue::Number(config.layout.border.width)),
        |_lua, config, value| {
            config.layout.border.width = f64::from_lua(value, _lua)?;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.border.active_color",
        PropertyType::String,
        |_lua, config| {
            let color = &config.layout.border.active_color;
            Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ))?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            let color = parse_color_string(&v)?;
            config.layout.border.active_color = color;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.border.inactive_color",
        PropertyType::String,
        |_lua, config| {
            let color = &config.layout.border.inactive_color;
            Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ))?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            let color = parse_color_string(&v)?;
            config.layout.border.inactive_color = color;
            Ok(())
        },
    );

    registry.update_accessor_with_type(
        "layout.border.urgent_color",
        PropertyType::String,
        |_lua, config| {
            let color = &config.layout.border.urgent_color;
            Ok(LuaValue::String(_lua.create_string(&format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ))?))
        },
        |_lua, config, value| {
            let v = String::from_lua(value, _lua)?;
            let color = parse_color_string(&v)?;
            config.layout.border.urgent_color = color;
            Ok(())
        },
    );
}
