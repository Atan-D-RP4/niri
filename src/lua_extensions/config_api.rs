//! Configuration API module for exposing Niri settings to Lua.
//!
//! This module provides the `niri.config` API that allows Lua scripts to read and configure Niri settings.
//! Implements Tier 2 configuration parity with KDL.

use mlua::prelude::*;
use niri_config::{
    Animations, Config, Cursor, Gestures, Input, Layout, Outputs,
    animations::{Kind, EasingParams, Curve, SpringParams},
};

/// Main configuration API handler
pub struct ConfigApi;

impl ConfigApi {
    /// Register the configuration API to Lua
    pub fn register_to_lua(lua: &Lua, config: &Config) -> LuaResult<()> {
        let globals = lua.globals();

        // Get or create the niri table
        let niri_table: LuaTable = globals
            .get("niri")
            .unwrap_or_else(|_| lua.create_table().unwrap());

        // Create the config table
        let config_table = lua.create_table()?;

        // Register configuration subsystems
        Self::register_animations(lua, &config_table, &config.animations)?;
        Self::register_input(lua, &config_table, &config.input)?;
        Self::register_layout(lua, &config_table, &config.layout)?;
        Self::register_cursor(lua, &config_table, &config.cursor)?;
        Self::register_output(lua, &config_table, &config.outputs)?;
        Self::register_gestures(lua, &config_table, &config.gestures)?;

        // Set niri.config
        niri_table.set("config", config_table)?;
        globals.set("niri", niri_table)?;

        Ok(())
    }

    /// Register animations configuration
    fn register_animations(lua: &Lua, config_table: &LuaTable, anim_config: &Animations) -> LuaResult<()> {
        let animations = lua.create_table()?;

        // Global animation settings
        animations.set("off", anim_config.off)?;
        animations.set("slowdown", anim_config.slowdown)?;

        // Workspace switch animation
        let ws_switch = lua.create_table()?;
        Self::set_animation_values(lua, &ws_switch, &anim_config.workspace_switch.0)?;
        animations.set("workspace_switch", ws_switch)?;

        // Window open animation
        let win_open = lua.create_table()?;
        Self::set_animation_values(lua, &win_open, &anim_config.window_open.anim)?;
        if let Some(shader) = &anim_config.window_open.custom_shader {
            win_open.set("custom_shader", shader.clone())?;
        }
        animations.set("window_open", win_open)?;

        // Window close animation
        let win_close = lua.create_table()?;
        Self::set_animation_values(lua, &win_close, &anim_config.window_close.anim)?;
        if let Some(shader) = &anim_config.window_close.custom_shader {
            win_close.set("custom_shader", shader.clone())?;
        }
        animations.set("window_close", win_close)?;

        // Horizontal view movement animation
        let h_view = lua.create_table()?;
        Self::set_animation_values(lua, &h_view, &anim_config.horizontal_view_movement.0)?;
        animations.set("horizontal_view_movement", h_view)?;

        // Window movement animation
        let win_move = lua.create_table()?;
        Self::set_animation_values(lua, &win_move, &anim_config.window_movement.0)?;
        animations.set("window_movement", win_move)?;

        // Window resize animation
        let win_resize = lua.create_table()?;
        Self::set_animation_values(lua, &win_resize, &anim_config.window_resize.anim)?;
        if let Some(shader) = &anim_config.window_resize.custom_shader {
            win_resize.set("custom_shader", shader.clone())?;
        }
        animations.set("window_resize", win_resize)?;

        // Config notification animation
        let cfg_notif = lua.create_table()?;
        Self::set_animation_values(lua, &cfg_notif, &anim_config.config_notification_open_close.0)?;
        animations.set("config_notification_open_close", cfg_notif)?;

        // Exit confirmation animation
        let exit_confirm = lua.create_table()?;
        Self::set_animation_values(lua, &exit_confirm, &anim_config.exit_confirmation_open_close.0)?;
        animations.set("exit_confirmation_open_close", exit_confirm)?;

        // Screenshot UI animation
        let screenshot = lua.create_table()?;
        Self::set_animation_values(lua, &screenshot, &anim_config.screenshot_ui_open.0)?;
        animations.set("screenshot_ui_open", screenshot)?;

        // Overview animation
        let overview = lua.create_table()?;
        Self::set_animation_values(lua, &overview, &anim_config.overview_open_close.0)?;
        animations.set("overview_open_close", overview)?;

        config_table.set("animations", animations)?;
        Ok(())
    }

    /// Helper to set animation values (off flag and animation kind)
    fn set_animation_values(lua: &Lua, table: &LuaTable, anim: &niri_config::Animation) -> LuaResult<()> {
        table.set("off", anim.off)?;

        match anim.kind {
            Kind::Easing(EasingParams { duration_ms, curve }) => {
                table.set("duration_ms", duration_ms)?;
                let curve_str = match curve {
                    Curve::Linear => "linear",
                    Curve::EaseOutQuad => "ease-out-quad",
                    Curve::EaseOutCubic => "ease-out-cubic",
                    Curve::EaseOutExpo => "ease-out-expo",
                    Curve::CubicBezier(x1, y1, x2, y2) => {
                        let bezier_table = lua.create_table()?;
                        bezier_table.set("x1", x1)?;
                        bezier_table.set("y1", y1)?;
                        bezier_table.set("x2", x2)?;
                        bezier_table.set("y2", y2)?;
                        table.set("bezier", bezier_table)?;
                        "cubic-bezier"
                    }
                };
                table.set("curve", curve_str)?;
            }
            Kind::Spring(SpringParams {
                damping_ratio,
                stiffness,
                epsilon,
            }) => {
                table.set("damping_ratio", damping_ratio)?;
                table.set("stiffness", stiffness)?;
                table.set("epsilon", epsilon)?;
                table.set("kind", "spring")?;
            }
        }

        Ok(())
    }

    /// Register input configuration
    fn register_input(lua: &Lua, config_table: &LuaTable, input_config: &Input) -> LuaResult<()> {
        let input = lua.create_table()?;

        // Keyboard settings
        let keyboard = lua.create_table()?;
        keyboard.set("repeat_delay", input_config.keyboard.repeat_delay)?;
        keyboard.set("repeat_rate", input_config.keyboard.repeat_rate)?;
        
        let xkb = lua.create_table()?;
        xkb.set("layout", input_config.keyboard.xkb.layout.clone())?;
        xkb.set("variant", input_config.keyboard.xkb.variant.clone())?;
        xkb.set("rules", input_config.keyboard.xkb.rules.clone())?;
        xkb.set("model", input_config.keyboard.xkb.model.clone())?;
        if let Some(opts) = &input_config.keyboard.xkb.options {
            xkb.set("options", opts.clone())?;
        }
        keyboard.set("xkb", xkb)?;
        keyboard.set("numlock", input_config.keyboard.numlock)?;
        input.set("keyboard", keyboard)?;

        // Mouse settings
        let mouse = lua.create_table()?;
        mouse.set("accel_speed", input_config.mouse.accel_speed.0)?;
        mouse.set("accel_profile", format!("{:?}", input_config.mouse.accel_profile))?;
        input.set("mouse", mouse)?;

        // Touchpad settings
        let touchpad = lua.create_table()?;
        touchpad.set("accel_speed", input_config.touchpad.accel_speed.0)?;
        touchpad.set("accel_profile", format!("{:?}", input_config.touchpad.accel_profile))?;
        touchpad.set("tap", input_config.touchpad.tap)?;
        touchpad.set("tap_button_map", format!("{:?}", input_config.touchpad.tap_button_map))?;
        touchpad.set("natural_scroll", input_config.touchpad.natural_scroll)?;
        input.set("touchpad", touchpad)?;

        config_table.set("input", input)?;
        Ok(())
    }

    /// Register layout configuration
    fn register_layout(lua: &Lua, config_table: &LuaTable, layout_config: &Layout) -> LuaResult<()> {
        let layout = lua.create_table()?;

        layout.set("gaps", layout_config.gaps)?;
        layout.set("struts", lua.create_table()?)?;  // TODO: implement struts properly

        config_table.set("layout", layout)?;
        Ok(())
    }

    /// Register cursor configuration
    fn register_cursor(lua: &Lua, config_table: &LuaTable, cursor_config: &Cursor) -> LuaResult<()> {
        let cursor = lua.create_table()?;

        cursor.set("xcursor_theme", cursor_config.xcursor_theme.clone())?;
        cursor.set("xcursor_size", cursor_config.xcursor_size)?;
        cursor.set("hide_when_typing", cursor_config.hide_when_typing)?;
        if let Some(ms) = cursor_config.hide_after_inactive_ms {
            cursor.set("hide_after_inactive_ms", ms)?;
        }

        config_table.set("cursor", cursor)?;
        Ok(())
    }

    /// Register output configuration
    fn register_output(lua: &Lua, config_table: &LuaTable, outputs: &Outputs) -> LuaResult<()> {
        let output_table = lua.create_table()?;

        for output in &outputs.0 {
            let output_config = lua.create_table()?;
            
            // Set output name and basic properties
            output_config.set("off", output.off)?;
            
            if let Some(scale) = output.scale {
                output_config.set("scale", scale.0)?;
            }
            
            if let Some(position) = &output.position {
                output_config.set("x", position.x)?;
                output_config.set("y", position.y)?;
            }
            
            if let Some(mode) = &output.mode {
                output_config.set("mode_custom", mode.custom)?;
                // TODO: expose more mode details
            }

            output_table.set(output.name.clone(), output_config)?;
        }

        config_table.set("output", output_table)?;
        Ok(())
    }

    /// Register gestures configuration
    fn register_gestures(lua: &Lua, config_table: &LuaTable, _gestures_config: &Gestures) -> LuaResult<()> {
        let gestures = lua.create_table()?;

        // TODO: Implement gestures configuration more fully
        let touchpad = lua.create_table()?;
        touchpad.set("enabled", false)?;
        gestures.set("touchpad", touchpad)?;

        config_table.set("gestures", gestures)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_api_registration() {
        let lua = Lua::new();
        // Create niri table first
        let globals = lua.globals();
        let niri_table = lua.create_table().unwrap();
        globals.set("niri", niri_table).unwrap();

        // Register should not fail with default configurations
        let result = ConfigApi::register_to_lua(&lua, &Config::default());
        assert!(result.is_ok());

        // Verify config table exists and is accessible
        let niri_table: LuaTable = globals.get("niri").unwrap();
        let config_table: LuaTable = niri_table.get("config").unwrap();
        
        // Verify all subsystems are registered
        assert!(config_table.get::<LuaTable>("animations").is_ok());
        assert!(config_table.get::<LuaTable>("input").is_ok());
        assert!(config_table.get::<LuaTable>("layout").is_ok());
        assert!(config_table.get::<LuaTable>("cursor").is_ok());
        assert!(config_table.get::<LuaTable>("output").is_ok());
        assert!(config_table.get::<LuaTable>("gestures").is_ok());
    }

    #[test]
    fn test_animations_api() {
        let lua = Lua::new();
        let globals = lua.globals();
        let niri_table = lua.create_table().unwrap();
        globals.set("niri", niri_table).unwrap();

        ConfigApi::register_to_lua(&lua, &Config::default()).unwrap();

        let niri_table: LuaTable = globals.get("niri").unwrap();
        let config_table: LuaTable = niri_table.get("config").unwrap();
        let animations: LuaTable = config_table.get("animations").unwrap();

        // Verify animation settings are accessible
        assert_eq!(animations.get::<bool>("off").unwrap(), false);
        assert_eq!(animations.get::<f64>("slowdown").unwrap(), 1.0);

        // Verify animation tables exist
        assert!(animations.get::<LuaTable>("workspace_switch").is_ok());
        assert!(animations.get::<LuaTable>("window_open").is_ok());
        assert!(animations.get::<LuaTable>("window_close").is_ok());
    }
}
