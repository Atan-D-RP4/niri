//! Configuration validators module for type safety and early error detection.
//!
//! This module provides validation functions for all Niri configuration settings,
//! ensuring type correctness and reasonable value ranges before settings are applied.

use mlua::prelude::*;
use regex::Regex;

/// Configuration validator for checking settings before applying them.
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate entire configuration table
    pub fn validate_config(config: &LuaValue) -> LuaResult<()> {
        match config {
            LuaValue::Table(_table) => {
                // Basic validation that it's a table
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Configuration must be a table".to_string(),
            )),
        }
    }

    /// Validate a specific setting by key and value
    pub fn validate_setting(key: &str, value: &LuaValue) -> LuaResult<()> {
        match key {
            // Appearance settings
            "gaps" => Self::validate_gaps(value),
            "border_width" => Self::validate_border_width(value),
            "border_active" => Self::validate_color(value),
            "border_inactive" => Self::validate_color(value),
            "focus_ring" => Self::validate_color(value),
            "prefer_no_csd" => Self::validate_bool(value),

            // Animation settings
            "duration" => Self::validate_duration(value),
            "curve" => Self::validate_curve(value),

            // Input settings
            "repeat_delay" => Self::validate_repeat_delay(value),
            "repeat_rate" => Self::validate_repeat_rate(value),
            "accel_speed" => Self::validate_accel_speed(value),
            "accel_profile" => Self::validate_accel_profile(value),
            "tap" => Self::validate_bool(value),

            // Output settings
            "scale" => Self::validate_scale(value),
            "refresh_rate" => Self::validate_refresh_rate(value),

            // Layout settings
            "default_width_percent" => Self::validate_percentage(value),
            "default_height_percent" => Self::validate_percentage(value),

            // Unknown settings
            _ => Err(mlua::Error::RuntimeError(format!(
                "Unknown setting: {}",
                key
            ))),
        }
    }

    /// Validate gaps value (0-100)
    fn validate_gaps(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n < 0 || *n > 100 {
                    return Err(mlua::Error::RuntimeError(format!(
                        "gaps must be between 0 and 100, got {}",
                        n
                    )));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "gaps must be an integer".to_string(),
            )),
        }
    }

    /// Validate border width (0-20)
    fn validate_border_width(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n < 0 || *n > 20 {
                    return Err(mlua::Error::RuntimeError(
                        "border_width must be between 0 and 20".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "border_width must be an integer".to_string(),
            )),
        }
    }

    /// Validate color format (hex or named color)
    fn validate_color(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::String(s) => {
                let s = s.to_string_lossy();

                // Check for hex color format
                if s.starts_with('#') {
                    if !Regex::new(r"^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$")
                        .unwrap()
                        .is_match(&s)
                    {
                        return Err(mlua::Error::RuntimeError(
                            format!("Invalid color format: {}. Use #RRGGBB or #RRGGBBAA", s),
                        ));
                    }
                    Ok(())
                } else {
                    // For named colors, just check it's a reasonable string
                    if s.len() > 32 {
                        return Err(mlua::Error::RuntimeError(
                            "Color name too long".to_string(),
                        ));
                    }
                    Ok(())
                }
            }
            _ => Err(mlua::Error::RuntimeError(
                "Color must be a string".to_string(),
            )),
        }
    }

    /// Validate boolean value
    fn validate_bool(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Boolean(_) => Ok(()),
            _ => Err(mlua::Error::RuntimeError(
                "Value must be a boolean".to_string(),
            )),
        }
    }

    /// Validate animation duration (1-5000 ms)
    fn validate_duration(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n <= 0 || *n > 5000 {
                    return Err(mlua::Error::RuntimeError(
                        "Duration must be between 1 and 5000 ms".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Duration must be an integer".to_string(),
            )),
        }
    }

    /// Validate animation curve
    fn validate_curve(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::String(s) => {
                let s = s.to_string_lossy();
                let valid_curves = ["linear", "ease_in_out_cubic", "ease_out_cubic"];
                if !valid_curves.contains(&s.as_ref()) {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Unknown animation curve '{}'. Valid curves: linear, ease_in_out_cubic, ease_out_cubic",
                        s
                    )));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Curve must be a string".to_string(),
            )),
        }
    }

    /// Validate keyboard repeat delay (25-2000 ms)
    fn validate_repeat_delay(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n < 25 || *n > 2000 {
                    return Err(mlua::Error::RuntimeError(
                        "repeat_delay must be between 25 and 2000 ms".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "repeat_delay must be an integer".to_string(),
            )),
        }
    }

    /// Validate keyboard repeat rate (1-1000 cps)
    fn validate_repeat_rate(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Integer(n) => {
                if *n < 1 || *n > 1000 {
                    return Err(mlua::Error::RuntimeError(
                        "repeat_rate must be between 1 and 1000 characters per second".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "repeat_rate must be an integer".to_string(),
            )),
        }
    }

    /// Validate mouse/touchpad acceleration speed (0.0 to 10.0)
    fn validate_accel_speed(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Number(n) => {
                if *n < 0.0 || *n > 10.0 {
                    return Err(mlua::Error::RuntimeError(
                        "Acceleration speed must be between 0.0 and 10.0".to_string(),
                    ));
                }
                Ok(())
            }
            LuaValue::Integer(n) => {
                let n = *n as f64;
                if n < 0.0 || n > 10.0 {
                    return Err(mlua::Error::RuntimeError(
                        "Acceleration speed must be between 0.0 and 10.0".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Acceleration speed must be a number".to_string(),
            )),
        }
    }

    /// Validate acceleration profile
    fn validate_accel_profile(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::String(s) => {
                let s = s.to_string_lossy();
                let valid_profiles = ["flat", "adaptive"];
                if !valid_profiles.contains(&s.as_ref()) {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Unknown acceleration profile '{}'. Valid profiles: flat, adaptive",
                        s
                    )));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Acceleration profile must be a string".to_string(),
            )),
        }
    }

    /// Validate monitor scale (0.5 to 4.0)
    fn validate_scale(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Number(n) => {
                if *n < 0.5 || *n > 4.0 {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Scale must be between 0.5 and 4.0, got {}",
                        n
                    )));
                }
                Ok(())
            }
            LuaValue::Integer(n) => {
                let n = *n as f64;
                if n < 0.5 || n > 4.0 {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Scale must be between 0.5 and 4.0, got {}",
                        n
                    )));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Scale must be a number".to_string(),
            )),
        }
    }

    /// Validate refresh rate (30-240 Hz)
    fn validate_refresh_rate(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Number(n) => {
                if *n < 30.0 || *n > 240.0 {
                    return Err(mlua::Error::RuntimeError(
                        "Refresh rate must be between 30 and 240 Hz".to_string(),
                    ));
                }
                Ok(())
            }
            LuaValue::Integer(n) => {
                if *n < 30 || *n > 240 {
                    return Err(mlua::Error::RuntimeError(
                        "Refresh rate must be between 30 and 240 Hz".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Refresh rate must be a number".to_string(),
            )),
        }
    }

    /// Validate percentage (0-100)
    fn validate_percentage(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Number(n) => {
                if *n < 0.0 || *n > 100.0 {
                    return Err(mlua::Error::RuntimeError(
                        "Percentage must be between 0 and 100".to_string(),
                    ));
                }
                Ok(())
            }
            LuaValue::Integer(n) => {
                if *n < 0 || *n > 100 {
                    return Err(mlua::Error::RuntimeError(
                        "Percentage must be between 0 and 100".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                "Percentage must be a number".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_gaps_valid() {
        let value = LuaValue::Integer(8);
        assert!(ConfigValidator::validate_gaps(&value).is_ok());
    }

    #[test]
    fn test_validate_gaps_invalid() {
        let value = LuaValue::Integer(150);
        assert!(ConfigValidator::validate_gaps(&value).is_err());
    }

    #[test]
    fn test_validate_border_width_valid() {
        let value = LuaValue::Integer(2);
        assert!(ConfigValidator::validate_border_width(&value).is_ok());
    }

    #[test]
    fn test_validate_border_width_invalid() {
        let value = LuaValue::Integer(50);
        assert!(ConfigValidator::validate_border_width(&value).is_err());
    }

    #[test]
    fn test_validate_duration_valid() {
        let value = LuaValue::Integer(200);
        assert!(ConfigValidator::validate_duration(&value).is_ok());
    }

    #[test]
    fn test_validate_duration_invalid_zero() {
        let value = LuaValue::Integer(0);
        assert!(ConfigValidator::validate_duration(&value).is_err());
    }

    #[test]
    fn test_validate_duration_invalid_too_large() {
        let value = LuaValue::Integer(6000);
        assert!(ConfigValidator::validate_duration(&value).is_err());
    }

    #[test]
    fn test_validate_repeat_delay_valid() {
        let value = LuaValue::Integer(300);
        assert!(ConfigValidator::validate_repeat_delay(&value).is_ok());
    }

    #[test]
    fn test_validate_repeat_delay_too_low() {
        let value = LuaValue::Integer(10);
        assert!(ConfigValidator::validate_repeat_delay(&value).is_err());
    }

    #[test]
    fn test_validate_scale_valid() {
        let value = LuaValue::Number(2.0);
        assert!(ConfigValidator::validate_scale(&value).is_ok());
    }

    #[test]
    fn test_validate_scale_invalid() {
        let value = LuaValue::Number(5.0);
        assert!(ConfigValidator::validate_scale(&value).is_err());
    }

    #[test]
    fn test_validate_percentage_valid() {
        let value = LuaValue::Number(50.0);
        assert!(ConfigValidator::validate_percentage(&value).is_ok());
    }

    #[test]
    fn test_validate_percentage_invalid() {
        let value = LuaValue::Number(150.0);
        assert!(ConfigValidator::validate_percentage(&value).is_err());
    }

    // ========================================================================
    // Entry point tests
    // ========================================================================

    #[test]
    fn test_validate_config_table() {
        let lua = mlua::Lua::new();
        let table = lua.create_table().unwrap();
        let value = LuaValue::Table(table);
        assert!(ConfigValidator::validate_config(&value).is_ok());
    }

    #[test]
    fn test_validate_config_non_table() {
        let value = LuaValue::Integer(42);
        assert!(ConfigValidator::validate_config(&value).is_err());
    }

    #[test]
    fn test_validate_setting_gaps() {
        let value = LuaValue::Integer(10);
        assert!(ConfigValidator::validate_setting("gaps", &value).is_ok());
    }

    #[test]
    fn test_validate_setting_unknown_key() {
        let value = LuaValue::Integer(10);
        assert!(ConfigValidator::validate_setting("unknown_setting", &value).is_err());
    }

    // ========================================================================
    // Color validator tests
    // ========================================================================

    #[test]
    fn test_validate_color_valid_hex() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("#FF0000").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_color(&value).is_ok());
    }

    #[test]
    fn test_validate_color_invalid_type() {
        let value = LuaValue::Integer(42);
        assert!(ConfigValidator::validate_color(&value).is_err());
    }

    // ========================================================================
    // Bool validator tests
    // ========================================================================

    #[test]
    fn test_validate_bool_true() {
        let value = LuaValue::Boolean(true);
        assert!(ConfigValidator::validate_bool(&value).is_ok());
    }

    #[test]
    fn test_validate_bool_false() {
        let value = LuaValue::Boolean(false);
        assert!(ConfigValidator::validate_bool(&value).is_ok());
    }

    #[test]
    fn test_validate_bool_invalid_type() {
        let value = LuaValue::Integer(1);
        assert!(ConfigValidator::validate_bool(&value).is_err());
    }

    // ========================================================================
    // Curve validator tests
    // ========================================================================

    #[test]
    fn test_validate_curve_linear() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("linear").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_curve(&value).is_ok());
    }

    #[test]
    fn test_validate_curve_ease_in_out_cubic() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("ease_in_out_cubic").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_curve(&value).is_ok());
    }

    #[test]
    fn test_validate_curve_ease_out_cubic() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("ease_out_cubic").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_curve(&value).is_ok());
    }

    #[test]
    fn test_validate_curve_invalid() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("invalid_curve").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_curve(&value).is_err());
    }

    #[test]
    fn test_validate_curve_wrong_type() {
        let value = LuaValue::Integer(42);
        assert!(ConfigValidator::validate_curve(&value).is_err());
    }

    // ========================================================================
    // Repeat rate validator tests
    // ========================================================================

    #[test]
    fn test_validate_repeat_rate_valid() {
        let value = LuaValue::Integer(50);
        assert!(ConfigValidator::validate_repeat_rate(&value).is_ok());
    }

    #[test]
    fn test_validate_repeat_rate_zero() {
        let value = LuaValue::Integer(0);
        assert!(ConfigValidator::validate_repeat_rate(&value).is_err());
    }

    #[test]
    fn test_validate_repeat_rate_too_high() {
        let value = LuaValue::Integer(2000);
        assert!(ConfigValidator::validate_repeat_rate(&value).is_err());
    }

    #[test]
    fn test_validate_repeat_rate_wrong_type() {
        let value = LuaValue::Number(50.5);
        assert!(ConfigValidator::validate_repeat_rate(&value).is_err());
    }

    // ========================================================================
    // Acceleration speed validator tests
    // ========================================================================

    #[test]
    fn test_validate_accel_speed_valid_number() {
        let value = LuaValue::Number(2.5);
        assert!(ConfigValidator::validate_accel_speed(&value).is_ok());
    }

    #[test]
    fn test_validate_accel_speed_valid_integer() {
        let value = LuaValue::Integer(2);
        assert!(ConfigValidator::validate_accel_speed(&value).is_ok());
    }

    #[test]
    fn test_validate_accel_speed_zero() {
        let value = LuaValue::Number(0.0);
        assert!(ConfigValidator::validate_accel_speed(&value).is_ok());
    }

    #[test]
    fn test_validate_accel_speed_too_high() {
        let value = LuaValue::Number(15.0);
        assert!(ConfigValidator::validate_accel_speed(&value).is_err());
    }

    #[test]
    fn test_validate_accel_speed_negative() {
        let value = LuaValue::Number(-1.0);
        assert!(ConfigValidator::validate_accel_speed(&value).is_err());
    }

    #[test]
    fn test_validate_accel_speed_wrong_type() {
        let value = LuaValue::Boolean(true);
        assert!(ConfigValidator::validate_accel_speed(&value).is_err());
    }

    // ========================================================================
    // Acceleration profile validator tests
    // ========================================================================

    #[test]
    fn test_validate_accel_profile_flat() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("flat").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_accel_profile(&value).is_ok());
    }

    #[test]
    fn test_validate_accel_profile_adaptive() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("adaptive").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_accel_profile(&value).is_ok());
    }

    #[test]
    fn test_validate_accel_profile_invalid() {
        let lua = mlua::Lua::new();
        let s = lua.create_string("invalid").unwrap();
        let value = LuaValue::String(s);
        assert!(ConfigValidator::validate_accel_profile(&value).is_err());
    }

    #[test]
    fn test_validate_accel_profile_wrong_type() {
        let value = LuaValue::Integer(42);
        assert!(ConfigValidator::validate_accel_profile(&value).is_err());
    }

    // ========================================================================
    // Refresh rate validator tests
    // ========================================================================

    #[test]
    fn test_validate_refresh_rate_valid_number() {
        let value = LuaValue::Number(60.0);
        assert!(ConfigValidator::validate_refresh_rate(&value).is_ok());
    }

    #[test]
    fn test_validate_refresh_rate_valid_integer() {
        let value = LuaValue::Integer(144);
        assert!(ConfigValidator::validate_refresh_rate(&value).is_ok());
    }

    #[test]
    fn test_validate_refresh_rate_low_boundary() {
        let value = LuaValue::Integer(30);
        assert!(ConfigValidator::validate_refresh_rate(&value).is_ok());
    }

    #[test]
    fn test_validate_refresh_rate_high_boundary() {
        let value = LuaValue::Integer(240);
        assert!(ConfigValidator::validate_refresh_rate(&value).is_ok());
    }

    #[test]
    fn test_validate_refresh_rate_too_low() {
        let value = LuaValue::Integer(20);
        assert!(ConfigValidator::validate_refresh_rate(&value).is_err());
    }

    #[test]
    fn test_validate_refresh_rate_too_high() {
        let value = LuaValue::Integer(300);
        assert!(ConfigValidator::validate_refresh_rate(&value).is_err());
    }

    #[test]
    fn test_validate_refresh_rate_wrong_type() {
        let value = LuaValue::Boolean(true);
        assert!(ConfigValidator::validate_refresh_rate(&value).is_err());
    }

    // ========================================================================
    // Scale validator tests
    // ========================================================================

    #[test]
    fn test_validate_scale_boundary_low() {
        let value = LuaValue::Number(0.5);
        assert!(ConfigValidator::validate_scale(&value).is_ok());
    }

    #[test]
    fn test_validate_scale_boundary_high() {
        let value = LuaValue::Number(4.0);
        assert!(ConfigValidator::validate_scale(&value).is_ok());
    }

    #[test]
    fn test_validate_scale_too_low() {
        let value = LuaValue::Number(0.25);
        assert!(ConfigValidator::validate_scale(&value).is_err());
    }

    #[test]
    fn test_validate_scale_too_high() {
        let value = LuaValue::Number(5.0);
        assert!(ConfigValidator::validate_scale(&value).is_err());
    }

    #[test]
    fn test_validate_scale_with_integer() {
        let value = LuaValue::Integer(2);
        assert!(ConfigValidator::validate_scale(&value).is_ok());
    }

    #[test]
    fn test_validate_scale_wrong_type() {
        let value = LuaValue::Boolean(true);
        assert!(ConfigValidator::validate_scale(&value).is_err());
    }

    // ========================================================================
    // Gaps validator edge cases
    // ========================================================================

    #[test]
    fn test_validate_gaps_boundary_zero() {
        let value = LuaValue::Integer(0);
        assert!(ConfigValidator::validate_gaps(&value).is_ok());
    }

    #[test]
    fn test_validate_gaps_boundary_max() {
        let value = LuaValue::Integer(100);
        assert!(ConfigValidator::validate_gaps(&value).is_ok());
    }

    #[test]
    fn test_validate_gaps_negative() {
        let value = LuaValue::Integer(-1);
        assert!(ConfigValidator::validate_gaps(&value).is_err());
    }

    #[test]
    fn test_validate_gaps_wrong_type() {
        let value = LuaValue::Number(8.5);
        assert!(ConfigValidator::validate_gaps(&value).is_err());
    }

    // ========================================================================
    // Border width validator edge cases
    // ========================================================================

    #[test]
    fn test_validate_border_width_boundary_zero() {
        let value = LuaValue::Integer(0);
        assert!(ConfigValidator::validate_border_width(&value).is_ok());
    }

    #[test]
    fn test_validate_border_width_boundary_max() {
        let value = LuaValue::Integer(20);
        assert!(ConfigValidator::validate_border_width(&value).is_ok());
    }

    #[test]
    fn test_validate_border_width_negative() {
        let value = LuaValue::Integer(-1);
        assert!(ConfigValidator::validate_border_width(&value).is_err());
    }

    #[test]
    fn test_validate_border_width_wrong_type() {
        let value = LuaValue::Number(2.5);
        assert!(ConfigValidator::validate_border_width(&value).is_err());
    }

    // ========================================================================
    // Percentage validator edge cases
    // ========================================================================

    #[test]
    fn test_validate_percentage_boundary_zero() {
        let value = LuaValue::Number(0.0);
        assert!(ConfigValidator::validate_percentage(&value).is_ok());
    }

    #[test]
    fn test_validate_percentage_boundary_max() {
        let value = LuaValue::Number(100.0);
        assert!(ConfigValidator::validate_percentage(&value).is_ok());
    }

    #[test]
    fn test_validate_percentage_negative() {
        let value = LuaValue::Number(-0.1);
        assert!(ConfigValidator::validate_percentage(&value).is_err());
    }

    #[test]
    fn test_validate_percentage_with_integer() {
        let value = LuaValue::Integer(75);
        assert!(ConfigValidator::validate_percentage(&value).is_ok());
    }

    #[test]
    fn test_validate_percentage_wrong_type() {
        let value = LuaValue::Boolean(true);
        assert!(ConfigValidator::validate_percentage(&value).is_err());
    }

    // ========================================================================
    // Snapshot test examples (using insta crate)
    // ========================================================================
    // These examples demonstrate how to use snapshot testing for complex
    // assertions. Snapshots are useful for testing output that's lengthy
    // or complex to manually verify.
    //
    // To update snapshots: cargo insta review
    // To regenerate: cargo test --package niri-lua -- --nocapture

    #[test]
    fn test_validate_config_snapshot() {
        use insta::assert_snapshot;
        
        // Example: Verify validation result format matches expected output
        let lua = mlua::Lua::new();
        let table = lua.create_table().unwrap();
        let value = LuaValue::Table(table);
        let result = ConfigValidator::validate_config(&value);
        
        // Instead of manual assertions, use snapshot
        assert_snapshot!(format!("{:?}", result));
    }
}
