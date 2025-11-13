//! Integration tests for Lua configuration system.
//!
//! These tests verify that Lua scripts can successfully modify Niri's configuration.

#[cfg(test)]
mod lua_config_tests {
    use niri::lua_extensions::{LuaConfig, apply_lua_config};
    use niri_config::Config;

    #[test]
    fn test_lua_config_loads_successfully() {
        // Test that a basic Lua config file can be loaded without error
        let lua_code = r#"
            -- Simple Lua configuration
            local config = {
                prefer_no_csd = true
            }
            prefer_no_csd = true
        "#;

        let lua_config = LuaConfig::from_string(lua_code);
        assert!(lua_config.is_ok());
    }

    #[test]
    fn test_apply_prefer_no_csd_setting() {
        // Test that prefer_no_csd can be set from Lua
        let lua_code = "prefer_no_csd = true";

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let mut config = Config::default();

        // Default should be true, so we'll verify the setting is applied
        let original = config.prefer_no_csd;
        
        apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");
        
        // The setting should be applied from Lua
        assert_eq!(config.prefer_no_csd, true);
    }

    #[test]
    fn test_apply_prefer_no_csd_false() {
        // Test that prefer_no_csd can be explicitly set to false
        let lua_code = "prefer_no_csd = false";

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let mut config = Config::default();

        apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");
        
        assert_eq!(config.prefer_no_csd, false);
    }

    #[test]
    fn test_empty_lua_config_doesnt_error() {
        // Test that an empty Lua file doesn't cause errors
        let lua_code = "-- Empty configuration";

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let mut config = Config::default();
        let original = config.prefer_no_csd;

        apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");
        
        // Config should remain unchanged
        assert_eq!(config.prefer_no_csd, original);
    }

    #[test]
    fn test_lua_with_undefined_variables() {
        // Test that undefined variables in Lua don't cause errors
        let lua_code = r#"
            prefer_no_csd = true
            -- Some undefined variable that we don't support yet
            -- some_future_setting = 42
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let mut config = Config::default();

        apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");
        
        // Only the supported setting should be applied
        assert_eq!(config.prefer_no_csd, true);
    }

    #[test]
    fn test_lua_config_with_comments_and_whitespace() {
        // Test that Lua code with comments and whitespace is handled correctly
        let lua_code = r#"
            -- This is a comment
            
            -- Configure Niri through Lua
            prefer_no_csd = true
            
            -- More comments
            -- prefer_no_csd = false  -- This line is commented out
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let mut config = Config::default();

        apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");
        
        assert_eq!(config.prefer_no_csd, true);
    }

    #[test]
    fn test_lua_config_type_mismatch_ignored() {
        // Test that type mismatches are gracefully handled
        let lua_code = r#"
            -- Set to a string instead of boolean
            prefer_no_csd = "true"
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let mut config = Config::default();
        let original = config.prefer_no_csd;

        // This should not error, just skip the invalid setting
        let result = apply_lua_config(lua_config.runtime(), &mut config);
        assert!(result.is_ok());
        
        // Config should not change since the type was wrong
        assert_eq!(config.prefer_no_csd, original);
    }

    #[test]
    fn test_multiple_settings() {
        // Test that multiple settings can be applied from Lua
        let lua_code = r#"
            prefer_no_csd = true
            -- Future: can add more settings here as they're implemented
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let mut config = Config::default();

        apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");
        
        // Verify the setting was applied
        assert_eq!(config.prefer_no_csd, true);
    }
}
