//! Integration tests for Lua configuration system.
//!
//! These tests verify that Lua scripts can successfully modify Niri's configuration.

#[cfg(test)]
mod lua_config_tests {
    use crate::lua_extensions::{LuaConfig, apply_lua_config};
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
        let _original = config.prefer_no_csd;

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

     #[test]
     fn test_lua_keybindings_extracted() {
         // Test that keybindings can be extracted from Lua configuration
         let lua_code = r#"
             binds = {
                 {
                     key = "Super+Return",
                     action = "spawn",
                     args = { "alacritty" }
                 },
                 {
                     key = "Super+Q",
                     action = "close-window",
                     args = {}
                 }
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_binds_count = config.binds.0.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // Should have added 2 keybindings
         assert_eq!(config.binds.0.len(), original_binds_count + 2);
     }

     #[test]
     fn test_lua_keybinding_with_spawn_action() {
         // Test that spawn action keybindings are correctly converted
         let lua_code = r#"
             binds = {
                 {
                     key = "Super+Return",
                     action = "spawn",
                     args = { "alacritty" }
                 }
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_binds_count = config.binds.0.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // Should have added 1 keybinding
         assert_eq!(config.binds.0.len(), original_binds_count + 1);
     }

     #[test]
     fn test_lua_keybinding_with_action_only() {
         // Test that action-only keybindings (no args) work correctly
         let lua_code = r#"
             binds = {
                 {
                     key = "Super+J",
                     action = "focus-window-down",
                     args = {}
                 }
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_binds_count = config.binds.0.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // Should have added 1 keybinding
         assert_eq!(config.binds.0.len(), original_binds_count + 1);
     }

     #[test]
     fn test_lua_empty_binds_table() {
         // Test that an empty binds table doesn't cause errors
         let lua_code = "binds = {}";

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_binds_count = config.binds.0.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // No keybindings should be added
         assert_eq!(config.binds.0.len(), original_binds_count);
     }

     #[test]
     fn test_lua_keybinding_invalid_action_skipped() {
         // Test that keybindings with invalid actions are skipped
         let lua_code = r#"
             binds = {
                 {
                     key = "Super+X",
                     action = "invalid-action",
                     args = {}
                 }
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_binds_count = config.binds.0.len();

         // This should not error, just skip the invalid binding
         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // No keybindings should be added
         assert_eq!(config.binds.0.len(), original_binds_count);
     }

     #[test]
     fn test_lua_multiple_valid_and_invalid_keybindings() {
         // Test that valid keybindings are added even when some are invalid
         let lua_code = r#"
             binds = {
                 {
                     key = "Super+Return",
                     action = "spawn",
                     args = { "alacritty" }
                 },
                 {
                     key = "Super+Invalid",
                     action = "invalid-action",
                     args = {}
                 },
                 {
                     key = "Super+Q",
                     action = "close-window",
                     args = {}
                 }
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_binds_count = config.binds.0.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

          // Should have added 2 valid keybindings (the invalid one is skipped)
          assert_eq!(config.binds.0.len(), original_binds_count + 2);
      }

      #[test]
      fn test_lua_keybinding_focus_workspace() {
          // Test that focus-workspace keybinding with numeric argument works
          let lua_code = r#"
              binds = {
                  { key = "Super+1", action = "focus-workspace", args = { 1 } },
                  { key = "Super+2", action = "focus-workspace", args = { 2 } },
              }
          "#;

          let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
          let mut config = Config::default();
          let original_binds_count = config.binds.0.len();

          apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

          // Should have added 2 keybindings with workspace focus actions
          assert!(config.binds.0.len() >= original_binds_count + 2);
      }

      #[test]
      fn test_lua_keybinding_set_column_width() {
          // Test that set-column-width keybinding with percentage argument works
          let lua_code = r#"
              binds = {
                  { key = "Super+Plus", action = "set-column-width", args = { "+10%" } },
                  { key = "Super+Minus", action = "set-column-width", args = { "-10%" } },
              }
          "#;

          let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
          let mut config = Config::default();
          let original_binds_count = config.binds.0.len();

          apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

          // Should have added 2 keybindings with set-column-width actions
          assert!(config.binds.0.len() >= original_binds_count + 2);
      }

      #[test]
      fn test_lua_keybinding_set_window_height() {
          // Test that set-window-height keybinding with percentage argument works
          let lua_code = r#"
              binds = {
                  { key = "Super+Shift+Plus", action = "set-window-height", args = { "+5%" } },
                  { key = "Super+Shift+Minus", action = "set-window-height", args = { "-5%" } },
              }
          "#;

          let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
          let mut config = Config::default();
          let original_binds_count = config.binds.0.len();

          apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

          // Should have added 2 keybindings with set-window-height actions
          assert!(config.binds.0.len() >= original_binds_count + 2);
      }

      #[test]
      fn test_lua_keybinding_move_column_to_workspace() {
          // Test that move-column-to-workspace keybinding with numeric argument works
          let lua_code = r#"
              binds = {
                  { key = "Super+Ctrl+1", action = "move-column-to-workspace", args = { 1 } },
              }
          "#;

          let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
          let mut config = Config::default();
          let original_binds_count = config.binds.0.len();

          apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

          // Should have added 1 keybinding with move-column-to-workspace action
          assert!(config.binds.0.len() >= original_binds_count + 1);
      }

      #[test]
      #[ignore]  // Run with: cargo test -- --ignored --nocapture
      fn test_real_config_keybinding_analysis() {
          // Load test_config.lua and report statistics about keybinding extraction
         use std::fs;
         use std::path::Path;

         let config_path = Path::new("test_config.lua");
         if !config_path.exists() {
            eprintln!("✗ test_config.lua not found");
            return;
         }

         let lua_code = fs::read_to_string(config_path).expect("Failed to read test_config.lua");
         let lua_config = match LuaConfig::from_string(&lua_code) {
             Ok(c) => c,
             Err(e) => {
                 eprintln!("✗ Failed to load Lua config: {}", e);
                 return;
             }
         };

         let mut config = Config::default();
         let original_binds_count = config.binds.0.len();

         match apply_lua_config(lua_config.runtime(), &mut config) {
             Ok(_) => {
                 let new_binds_count = config.binds.0.len();
                 let added = new_binds_count - original_binds_count;
                 println!("\n=== Keybinding Extraction Analysis ===");
                 println!("✓ Successfully loaded test_config.lua");
                 println!("  Original bindings: {}", original_binds_count);
                 println!("  New bindings: {}", new_binds_count);
                 println!("  Extracted: {}", added);
             }
             Err(e) => {
                 eprintln!("✗ Failed to apply Lua config: {}", e);
             }
         }
     }

     #[test]
     fn test_lua_startup_commands_simple_strings() {
         // Test that simple startup commands as strings are extracted
         let lua_code = r#"
             startup = {
                 "waybar",
                 "dunst",
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_cmds = config.spawn_at_startup.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // Should have added 2 startup commands
         assert!(config.spawn_at_startup.len() >= original_cmds + 2);
     }

     #[test]
     fn test_lua_startup_commands_with_args() {
         // Test that startup commands with arguments are extracted
         let lua_code = r#"
             startup = {
                 { command = { "alacritty", "-e", "bash" } },
                 { command = { "firefox" } },
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_cmds = config.spawn_at_startup.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // Should have added 2 startup commands
         assert!(config.spawn_at_startup.len() >= original_cmds + 2);
         // First command should have 3 parts
         if let Some(first_cmd) = config.spawn_at_startup.get(original_cmds) {
             assert_eq!(first_cmd.command.len(), 3);
             assert_eq!(first_cmd.command[0], "alacritty");
         }
     }

     #[test]
     fn test_lua_mixed_startup_commands() {
         // Test mixed startup commands (simple strings and structured)
         let lua_code = r#"
             startup = {
                 "waybar",
                 { command = { "alacritty", "-e", "tmux" } },
                 "dunst",
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_cmds = config.spawn_at_startup.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // Should have added 3 startup commands
         assert!(config.spawn_at_startup.len() >= original_cmds + 3);
     }

     #[test]
     fn test_lua_config_with_returned_table() {
         // Test that config can be returned from a Lua script
         // This tests the new feature where local variables are returned
         let lua_code = r#"
             local binds = {
                 { key = "Super+Return", action = "spawn", args = { "alacritty" } },
                 { key = "Super+Q", action = "close-window", args = {} },
             }
             
             local startup = {
                 "waybar",
                 "dunst",
             }
             
             prefer_no_csd = true
             
             return {
                 binds = binds,
                 startup = startup,
                 prefer_no_csd = prefer_no_csd,
             }
         "#;

         let lua_config = LuaConfig::from_string(lua_code).expect("Failed to create Lua config");
         let mut config = Config::default();
         let original_binds = config.binds.0.len();
         let original_cmds = config.spawn_at_startup.len();

         apply_lua_config(lua_config.runtime(), &mut config).expect("Failed to apply config");

         // Verify bindings were extracted from returned table
         assert_eq!(config.binds.0.len(), original_binds + 2, "Should have 2 new bindings");
         
         // Verify startup commands were extracted from returned table
         assert!(config.spawn_at_startup.len() >= original_cmds + 2, "Should have added startup commands");
         
         // Verify prefer_no_csd was applied
         assert_eq!(config.prefer_no_csd, true, "prefer_no_csd should be true");
     }
}
