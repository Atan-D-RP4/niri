//! Converts Lua configuration values to Niri Config structures.
//!
//! This module provides utilities for extracting configuration values from a Lua runtime
//! and applying them to Niri's Config struct.

use super::LuaRuntime;
use niri_config::{Config, binds::{Bind, Key, Action, WorkspaceReference}, misc::SpawnAtStartup};
use niri_ipc::SizeChange;
use anyhow::Result;
use log::{debug, info, warn};
use std::str::FromStr;

/// Represents a keybinding parsed from Lua configuration.
///
/// This is a simplified representation of a Niri keybinding that can be
/// extracted from Lua and converted to a Niri Bind struct.
#[derive(Debug, Clone)]
pub struct LuaKeybinding {
    /// The key combination (e.g., "Super+Return", "Ctrl+Alt+Delete")
    pub key: String,
    /// The action to perform (e.g., "spawn", "close-window")
    pub action: String,
    /// Optional arguments for the action (e.g., ["alacritty"] for spawn)
    pub args: Vec<String>,
    /// Whether the binding repeats when held
    pub repeat: bool,
}

/// Convert a Lua keybinding to a Niri Bind struct.
///
/// This function takes a Lua keybinding representation and attempts to convert it
/// to a Niri Bind struct. If the conversion fails, it logs a warning and returns None.
fn lua_keybinding_to_bind(lua_binding: LuaKeybinding) -> Option<Bind> {
    // Parse the key
    let key: Key = match Key::from_str(&lua_binding.key) {
        Ok(k) => k,
        Err(e) => {
            warn!("✗ Failed to parse key '{}': {}", lua_binding.key, e);
            return None;
        }
    };

    // Convert action string to Action enum
    let action = match lua_binding.action.as_str() {
        "spawn" => {
            if lua_binding.args.is_empty() {
                warn!("⚠ spawn action requires arguments");
                return None;
            }
            Action::Spawn(lua_binding.args)
        }
        "spawn-sh" => {
            if lua_binding.args.is_empty() {
                warn!("⚠ spawn-sh action requires arguments");
                return None;
            }
            let cmd = lua_binding.args.join(" ");
            Action::SpawnSh(cmd)
        }

        // Window management
        "close-window" => Action::CloseWindow,
        "fullscreen-window" => Action::FullscreenWindow,
        "toggle-windowed-fullscreen" => Action::ToggleWindowedFullscreen,
        "toggle-window-floating" => Action::ToggleWindowFloating,
        "maximize-column" => Action::MaximizeColumn,
        "center-column" => Action::CenterColumn,
        "center-visible-columns" => Action::CenterVisibleColumns,

        // Window focus
        "focus-window-down" => Action::FocusWindowDown,
        "focus-window-up" => Action::FocusWindowUp,
        "focus-window-or-workspace-down" => Action::FocusWindowOrWorkspaceDown,
        "focus-window-or-workspace-up" => Action::FocusWindowOrWorkspaceUp,

        // Column focus
        "focus-column-left" => Action::FocusColumnLeft,
        "focus-column-right" => Action::FocusColumnRight,
        "focus-column-first" => Action::FocusColumnFirst,
        "focus-column-last" => Action::FocusColumnLast,

        // Window movement
        "move-window-down" => Action::MoveWindowDown,
        "move-window-up" => Action::MoveWindowUp,

        // Column movement
        "move-column-left" => Action::MoveColumnLeft,
        "move-column-right" => Action::MoveColumnRight,
        "move-column-to-first" => Action::MoveColumnToFirst,
        "move-column-to-last" => Action::MoveColumnToLast,

        // Monitor focus
        "focus-monitor-left" => Action::FocusMonitorLeft,
        "focus-monitor-right" => Action::FocusMonitorRight,
        "focus-monitor-down" => Action::FocusMonitorDown,
        "focus-monitor-up" => Action::FocusMonitorUp,

        // Monitor movement
        "move-column-to-monitor-left" => Action::MoveColumnToMonitorLeft,
        "move-column-to-monitor-right" => Action::MoveColumnToMonitorRight,
        "move-column-to-monitor-down" => Action::MoveColumnToMonitorDown,
        "move-column-to-monitor-up" => Action::MoveColumnToMonitorUp,

        // Workspace focus
        "focus-workspace-down" => Action::FocusWorkspaceDown,
        "focus-workspace-up" => Action::FocusWorkspaceUp,

        // Workspace movement
        "move-workspace-down" => Action::MoveWorkspaceDown,
        "move-workspace-up" => Action::MoveWorkspaceUp,
        "move-column-to-workspace-down" => Action::MoveColumnToWorkspaceDown(true),
        "move-column-to-workspace-up" => Action::MoveColumnToWorkspaceUp(true),

        // Actions that require arguments - log warning instead of skipping
        "focus-workspace" => {
            if lua_binding.args.is_empty() {
                warn!("⚠ Action 'focus-workspace' requires a workspace number argument");
                return None;
            }
            match lua_binding.args[0].parse::<u8>() {
                Ok(index) => Action::FocusWorkspace(WorkspaceReference::Index(index)),
                Err(_) => {
                    warn!("⚠ Failed to parse workspace index from: {}", lua_binding.args[0]);
                    return None;
                }
            }
        }
        
        "move-column-to-workspace" => {
            if lua_binding.args.is_empty() {
                warn!("⚠ Action 'move-column-to-workspace' requires a workspace number argument");
                return None;
            }
            match lua_binding.args[0].parse::<u8>() {
                Ok(index) => Action::MoveColumnToWorkspace(WorkspaceReference::Index(index), true),
                Err(_) => {
                    warn!("⚠ Failed to parse workspace index from: {}", lua_binding.args[0]);
                    return None;
                }
            }
        }
        
        "set-column-width" => {
            if lua_binding.args.is_empty() {
                warn!("⚠ Action 'set-column-width' requires a size change argument");
                return None;
            }
            match SizeChange::from_str(&lua_binding.args[0]) {
                Ok(change) => Action::SetColumnWidth(change),
                Err(_) => {
                    warn!("⚠ Failed to parse size change from: {}", lua_binding.args[0]);
                    return None;
                }
            }
        }
        
        "set-window-height" => {
            if lua_binding.args.is_empty() {
                warn!("⚠ Action 'set-window-height' requires a size change argument");
                return None;
            }
            match SizeChange::from_str(&lua_binding.args[0]) {
                Ok(change) => Action::SetWindowHeight(change),
                Err(_) => {
                    warn!("⚠ Failed to parse size change from: {}", lua_binding.args[0]);
                    return None;
                }
            }
        }
        
        // Application control
        "exit" => Action::Quit(false),
        "quit" => Action::Quit(false),
        "suspend" => Action::Suspend,
        
        // Screen and power
        "power-off-monitors" => Action::PowerOffMonitors,
        "power-on-monitors" => Action::PowerOnMonitors,
        "do-screen-transition" => Action::DoScreenTransition(None),
        
        // UI overlays
        "show-hotkey-overlay" | "hotkey-overlay-toggle" => Action::ShowHotkeyOverlay,
        "toggle-overview" | "overview-toggle" => Action::ToggleOverview,
        "open-overview" => Action::OpenOverview,
        "close-overview" => Action::CloseOverview,
        
        // Screenshot actions
        "screenshot" => Action::Screenshot(true, None),
        "screenshot-screen" => Action::ScreenshotScreen(true, true, None),
        "screenshot-window" => Action::ScreenshotWindow(true, None),
        
        // Unsupported actions requiring arguments
        "move-column-to-monitor" |
        "move-window-to-monitor" |
        "focus-monitor" => {
            warn!("⚠ Action '{}' requires arguments that aren't fully supported yet in Lua config", lua_binding.action);
            return None;
        }

        _ => {
            warn!("✗ Unknown action: '{}'", lua_binding.action);
            return None;
        }
    };

    Some(Bind {
        key,
        action,
        repeat: lua_binding.repeat,
        cooldown: None,
        allow_when_locked: false,
        allow_inhibiting: true,
        hotkey_overlay_title: None,
    })
}


/// Attempts to extract and apply Lua configuration values to the given Config.
///
/// This function looks for specific configuration values in the Lua runtime's global scope
/// and applies them to the provided Config struct. Unknown or invalid values are logged
/// and skipped rather than causing errors.
///
/// # Arguments
///
/// * `runtime` - The Lua runtime to extract configuration from
/// * `config` - The Config struct to apply values to (modified in place)
///
/// # Example
///
/// ```ignore
/// let runtime = LuaRuntime::new()?;
/// runtime.load_file("niri.lua")?;
/// let mut config = Config::default();
/// apply_lua_config(&runtime, &mut config)?;
/// ```
pub fn apply_lua_config(runtime: &LuaRuntime, config: &mut Config) -> Result<()> {
    debug!("=== Applying Lua configuration to Config ===");

    // Try to extract simple boolean settings
    debug!("Checking for prefer_no_csd in Lua globals");
    if runtime.has_global("prefer_no_csd") {
        info!("✓ Found prefer_no_csd in Lua globals");
        match runtime.get_global_bool_opt("prefer_no_csd") {
            Ok(Some(prefer_no_csd)) => {
                info!("✓ Applying prefer_no_csd: {} → {} (changed: {})", 
                    config.prefer_no_csd, prefer_no_csd, config.prefer_no_csd != prefer_no_csd);
                config.prefer_no_csd = prefer_no_csd;
            }
            Ok(None) => {
                warn!("⚠ prefer_no_csd was nil in Lua");
            }
            Err(e) => {
                warn!("✗ Error getting prefer_no_csd: {}", e);
            }
        }
    } else {
        debug!("ℹ prefer_no_csd not found in Lua globals");
    }

    // Extract and apply keybindings
    debug!("Checking for keybindings in Lua globals");
            match runtime.get_keybindings() {
        Ok(raw_keybindings) => {
            debug!("[apply_lua_config] get_keybindings returned {} bindings", raw_keybindings.len());
            if raw_keybindings.is_empty() {
                info!("ℹ No keybindings found in Lua configuration");
            } else {
                info!("✓ Found {} keybindings in Lua", raw_keybindings.len());
                let mut converted_binds = Vec::new();

                for (key, action, args) in raw_keybindings {
                    debug!("[apply_lua_config] Processing binding: key='{}', action='{}'", key, action);
                    let lua_binding = LuaKeybinding {
                        key,
                        action,
                        args,
                        repeat: true, // Default to true for now
                    };

                    if let Some(bind) = lua_keybinding_to_bind(lua_binding) {
                        converted_binds.push(bind);
                    }
                }

                if !converted_binds.is_empty() {
                    info!("✓ Successfully converted {} keybindings", converted_binds.len());
                    // Merge with existing binds or replace them
                    config.binds.0.extend(converted_binds);
                    // Diagnostic log: print a sample of the added binds
                    debug!("[apply_lua_config] Sample converted binds: {:?}", &config.binds.0.iter().rev().take(5).collect::<Vec<_>>());
                } else {
                    warn!("⚠ No valid keybindings could be converted from Lua");
                }
            }
        }
        Err(e) => {
            warn!("✗ Error extracting keybindings from Lua: {}", e);
        }
    }

    // Extract and apply startup commands
    debug!("Checking for startup commands in Lua globals");
            match runtime.get_startup_commands() {
        Ok(startup_cmds) => {
            debug!("[apply_lua_config] get_startup_commands returned {} commands", startup_cmds.len());
            if startup_cmds.is_empty() {
                info!("ℹ No startup commands found in Lua configuration");
            } else {
                info!("✓ Found {} startup commands in Lua", startup_cmds.len());
                let mut spawn_at_startup = Vec::new();

                for cmd_vec in startup_cmds {
                    if cmd_vec.is_empty() {
                        continue;
                    }

                    debug!("[apply_lua_config] Processing startup command: {:?}", cmd_vec);

                    // For now, treat all commands as spawn-at-startup (execute as array)
                    // This matches the KDL syntax: spawn-at-startup "cmd" "arg1" "arg2"
                    spawn_at_startup.push(SpawnAtStartup {
                        command: cmd_vec,
                    });
                }

                if !spawn_at_startup.is_empty() {
                    info!("✓ Added {} startup commands", spawn_at_startup.len());
                    config.spawn_at_startup.extend(spawn_at_startup);
                    debug!("[apply_lua_config] Sample spawn_at_startup entries: {:?}", &config.spawn_at_startup.iter().rev().take(5).collect::<Vec<_>>());
                }
            }
        }
        Err(e) => {
            warn!("✗ Error extracting startup commands from Lua: {}", e);
        }
    }

    // Additional configuration options can be added here as they're implemented
    // Examples:
    // - Screen lock settings
    // - Animation settings
    // - Cursor settings
    // - etc.

    // Register the config API so Lua scripts can read the current configuration
    debug!("Registering configuration API to Lua");
    runtime.register_config_api(config)
        .map_err(|e| anyhow::anyhow!("Failed to register config API: {}", e))?;
    
    info!("✓ Configuration API registered successfully");

    debug!("=== Lua configuration application completed ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_lua_config_empty() {
        let runtime = LuaRuntime::new().unwrap();
        let mut config = Config::default();
        let result = apply_lua_config(&runtime, &mut config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_lua_config_with_values() {
        let runtime = LuaRuntime::new().unwrap();
        runtime
            .load_string("prefer_no_csd = true")
            .expect("Failed to load Lua code");

        let mut config = Config::default();
        let _original_value = config.prefer_no_csd;
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        assert_eq!(config.prefer_no_csd, true);
    }

    #[test]
    fn test_extract_keybindings() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Mod+T", action = "spawn", args = { "kitty" } },
            { key = "Mod+Q", action = "close-window" },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        // We should have added 2 more bindings
        assert!(config.binds.0.len() >= initial_bind_count + 2, 
            "Expected at least {} bindings, got {}", 
            initial_bind_count + 2, 
            config.binds.0.len());
    }

    #[test]
    fn test_exit_action() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Super+Alt+Q", action = "exit", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        assert_eq!(config.binds.0.len(), initial_bind_count + 1);
        let last_bind = &config.binds.0[initial_bind_count];
        matches!(last_bind.action, niri_config::binds::Action::Quit(false));
    }

    #[test]
    fn test_overview_toggle_action() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Super+O", action = "overview-toggle", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        assert_eq!(config.binds.0.len(), initial_bind_count + 1);
        let last_bind = &config.binds.0[initial_bind_count];
        matches!(last_bind.action, niri_config::binds::Action::ToggleOverview);
    }

    #[test]
    fn test_hotkey_overlay_action() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Super+F1", action = "hotkey-overlay-toggle", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        assert_eq!(config.binds.0.len(), initial_bind_count + 1);
        let last_bind = &config.binds.0[initial_bind_count];
        matches!(last_bind.action, niri_config::binds::Action::ShowHotkeyOverlay);
    }

    #[test]
    fn test_suspend_action() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Super+Alt+S", action = "suspend", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        assert_eq!(config.binds.0.len(), initial_bind_count + 1);
        let last_bind = &config.binds.0[initial_bind_count];
        matches!(last_bind.action, niri_config::binds::Action::Suspend);
    }

    #[test]
    fn test_multiple_new_actions() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Super+Alt+Q", action = "exit", args = {} },
            { key = "Super+O", action = "overview-toggle", args = {} },
            { key = "Super+F1", action = "show-hotkey-overlay", args = {} },
            { key = "Super+Alt+S", action = "suspend", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        // All 4 actions should be added
        assert_eq!(config.binds.0.len(), initial_bind_count + 4);
    }
}
