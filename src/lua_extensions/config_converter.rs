//! Converts Lua configuration values to Niri Config structures.
//!
//! This module provides utilities for extracting configuration values from a Lua runtime
//! and applying them to Niri's Config struct.

use super::LuaRuntime;
use niri_config::{Config, binds::{Bind, Key, Action}};
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

         // Window operations
         "consume-or-expel-window-left" => Action::ConsumeOrExpelWindowLeft,
         "consume-or-expel-window-right" => Action::ConsumeOrExpelWindowRight,
         "consume-window-into-column" => Action::ConsumeWindowIntoColumn,
         "expel-window-from-column" => Action::ExpelWindowFromColumn,
         "toggle-column-tabbed-display" => Action::ToggleColumnTabbedDisplay,
         "switch-focus-between-floating-and-tiling" => Action::SwitchFocusBetweenFloatingAndTiling,

        // Column operations
        "reset-window-height" => Action::ResetWindowHeight,
        "expand-column-to-available-width" => Action::ExpandColumnToAvailableWidth,
        "switch-preset-column-width" => Action::SwitchPresetColumnWidth,
        "switch-preset-window-height" => Action::SwitchPresetWindowHeight,

        // Screenshots
        "screenshot" => Action::Screenshot(true, None),
        "screenshot-screen" => Action::ScreenshotScreen(true, true, None),
        "screenshot-window" => Action::ScreenshotWindow(true, None),

        // Utilities
        "toggle-overview" => Action::ToggleOverview,
        "show-hotkey-overlay" => Action::ShowHotkeyOverlay,
        "toggle-keyboard-shortcuts-inhibit" => Action::ToggleKeyboardShortcutsInhibit,
        "power-off-monitors" => Action::PowerOffMonitors,

        // System
        "quit" => Action::Quit(false),
        "suspend" => Action::Suspend,

        // Actions that require arguments - log warning instead of skipping
        "focus-workspace" |
        "move-column-to-workspace" |
        "set-column-width" |
        "set-window-height" |
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
            if raw_keybindings.is_empty() {
                info!("ℹ No keybindings found in Lua configuration");
            } else {
                info!("✓ Found {} keybindings in Lua", raw_keybindings.len());
                let mut converted_binds = Vec::new();

                for (key, action, args) in raw_keybindings {
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
                } else {
                    warn!("⚠ No valid keybindings could be converted from Lua");
                }
            }
        }
        Err(e) => {
            warn!("✗ Error extracting keybindings from Lua: {}", e);
        }
    }

    // Additional configuration options can be added here as they're implemented
    // Examples:
    // - Screen lock settings
    // - Animation settings
    // - Cursor settings
    // - etc.

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
            .load_string("prefer_no_csd = false")
            .expect("Failed to load Lua code");

        let mut config = Config::default();
        let original_value = config.prefer_no_csd;
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        assert_eq!(config.prefer_no_csd, false);
        assert_ne!(config.prefer_no_csd, original_value);
    }
}
