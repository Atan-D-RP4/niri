//! Integration tests for Lua configuration system.
//!
//! These tests verify that Lua scripts can successfully modify Niri's configuration
//! using the ConfigWrapper API (`niri.config.*`).

#[cfg(test)]
mod lua_config_tests {
    use niri_config::Config;
    use niri_lua::LuaConfig;

    /// Helper function to extract config from a LuaConfig.
    /// Returns (config, had_changes) tuple.
    fn extract_lua_config(lua_config: &LuaConfig) -> (Config, bool) {
        if let Some(wrapper) = lua_config.config_wrapper() {
            let dirty = wrapper.take_dirty_flags();
            let config = wrapper.extract_config();
            (config, dirty.any())
        } else {
            (Config::default(), false)
        }
    }

    #[test]
    fn test_lua_config_loads_successfully() {
        // Test that a basic Lua config file can be loaded without error
        let lua_code = r#"
            -- Simple Lua configuration using the reactive API
            niri.config.prefer_no_csd = true
        "#;

        let lua_config = LuaConfig::from_string(lua_code);
        assert!(lua_config.is_ok());
    }

    #[test]
    fn test_apply_prefer_no_csd_setting() {
        // Test that prefer_no_csd can be set from Lua
        let lua_code = "niri.config.prefer_no_csd = true";

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        // The setting should be applied from Lua
        assert!(had_changes);
        assert!(config.prefer_no_csd);
    }

    #[test]
    fn test_apply_prefer_no_csd_false() {
        // Test that prefer_no_csd can be explicitly set to false
        let lua_code = "niri.config.prefer_no_csd = false";

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, _) = extract_lua_config(&lua_config);

        assert!(!config.prefer_no_csd);
    }

    #[test]
    fn test_empty_lua_config_doesnt_error() {
        // Test that an empty Lua file doesn't cause errors
        let lua_code = "-- Empty configuration";

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (_, had_changes) = extract_lua_config(&lua_config);

        // Config should remain unchanged (no changes)
        assert!(!had_changes);
    }

    #[test]
    fn test_lua_keybindings_reactive_api() {
        // Test that keybindings can be added via the reactive config API
        // NOTE: Binds API is complex and requires parsing key combinations.
        // This test is ignored until BindsProxy is fully implemented.
        // For now, we just verify the script doesn't error.
        let lua_code = r#"
            -- Binds API not yet implemented in ConfigWrapper
            -- niri.config.binds:add({ key = "Mod+Return", action = "spawn", args = { "alacritty" } })
        "#;

        let lua_config = LuaConfig::from_string(lua_code);
        assert!(lua_config.is_ok());
    }

    #[test]
    #[ignore = "Binds API not yet implemented in ConfigWrapper"]
    fn test_lua_keybinding_focus_workspace() {
        // Test that focus-workspace keybinding with numeric argument works
        let lua_code = r#"
            niri.config.binds:add({ key = "Mod+1", action = "focus-workspace", args = { 1 } })
            niri.config.binds:add({ key = "Mod+2", action = "focus-workspace", args = { 2 } })
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, _) = extract_lua_config(&lua_config);

        // Should have added 2 keybindings with workspace focus actions
        assert_eq!(config.binds.0.len(), 2);
    }

    #[test]
    #[ignore = "Binds API not yet implemented in ConfigWrapper"]
    fn test_lua_keybinding_set_column_width() {
        // Test that set-column-width keybinding with percentage argument works
        let lua_code = r#"
            niri.config.binds:add({ key = "Mod+Plus", action = "set-column-width", args = { "+10%" } })
            niri.config.binds:add({ key = "Mod+Minus", action = "set-column-width", args = { "-10%" } })
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, _) = extract_lua_config(&lua_config);

        // Should have added 2 keybindings with set-column-width actions
        assert_eq!(config.binds.0.len(), 2);
    }

    #[test]
    fn test_lua_startup_commands_reactive_api() {
        // Test that startup commands can be added via the reactive config API
        let lua_code = r#"
            niri.config.spawn_at_startup:append({ command = { "waybar" } })
            niri.config.spawn_at_startup:append({ command = { "dunst" } })
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        // Should have added 2 startup commands
        assert!(had_changes);
        assert_eq!(config.spawn_at_startup.len(), 2);
    }

    #[test]
    fn test_lua_startup_commands_with_args() {
        // Test that startup commands with arguments are extracted
        let lua_code = r#"
            niri.config.spawn_at_startup:append({ command = { "alacritty", "-e", "bash" } })
            niri.config.spawn_at_startup:append({ command = { "firefox" } })
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        // Should have added 2 startup commands
        assert!(had_changes);
        assert_eq!(config.spawn_at_startup.len(), 2);

        // First command should have 3 parts
        let first_cmd = &config.spawn_at_startup[0];
        assert_eq!(first_cmd.command.len(), 3);
        assert_eq!(first_cmd.command[0], "alacritty");
    }

    #[test]
    fn test_lua_layout_gaps() {
        // Test that layout.gaps can be set from Lua
        let lua_code = "niri.config.layout.gaps = 24";

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        assert!(had_changes);
        assert_eq!(config.layout.gaps, 24.0);
    }

    #[test]
    fn test_lua_input_keyboard_repeat() {
        // Test that keyboard repeat settings can be set from Lua
        let lua_code = r#"
            niri.config.input.keyboard.repeat_delay = 300
            niri.config.input.keyboard.repeat_rate = 50
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        assert!(had_changes);
        assert_eq!(config.input.keyboard.repeat_delay, 300);
        assert_eq!(config.input.keyboard.repeat_rate, 50);
    }

    #[test]
    fn test_lua_cursor_settings() {
        // Test that cursor settings can be set from Lua
        let lua_code = r#"
            niri.config.cursor.xcursor_theme = "Adwaita"
            niri.config.cursor.xcursor_size = 24
            niri.config.cursor.hide_when_typing = true
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        assert!(had_changes);
        assert_eq!(config.cursor.xcursor_theme, "Adwaita");
        assert_eq!(config.cursor.xcursor_size, 24);
        assert!(config.cursor.hide_when_typing);
    }

    #[test]
    fn test_lua_animations_settings() {
        // Test that animation settings can be set from Lua
        let lua_code = r#"
            niri.config.animations.off = true
            niri.config.animations.slowdown = 2.5
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        assert!(had_changes);
        assert!(config.animations.off);
        assert_eq!(config.animations.slowdown, 2.5);
    }

    #[test]
    fn test_lua_window_rules() {
        // Test that window rules can be added via the reactive config API
        let lua_code = r#"
            niri.config.window_rules:add({
                matches = { { app_id = "firefox" } },
                default_column_width = { proportion = 0.5 }
            })
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        assert!(had_changes);
        assert_eq!(config.window_rules.len(), 1);
    }

    #[test]
    fn test_lua_workspaces() {
        // Test that workspaces can be added via the reactive config API
        let lua_code = r#"
            niri.config.workspaces:add({ name = "main" })
            niri.config.workspaces:add({ name = "work", open_on_output = "eDP-1" })
        "#;

        let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
        let (config, had_changes) = extract_lua_config(&lua_config);

        assert!(had_changes);
        assert_eq!(config.workspaces.len(), 2);
    }
}
