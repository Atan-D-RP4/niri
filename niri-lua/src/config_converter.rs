//! Converts Lua configuration values to Niri Config structures.
//!
//! This module provides utilities for extracting configuration values from a Lua runtime
//! and applying them to Niri's Config struct.

use std::str::FromStr;

use anyhow::Result;
use log::{debug, info, warn};
use niri_config::binds::{Action, Bind, Key, WorkspaceReference};
use niri_config::misc::SpawnAtStartup;
use niri_config::{input, Config, FloatOrInt};
use niri_ipc::SizeChange;

use super::LuaRuntime;

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

        // Window sizing and layout presets
        "switch-preset-column-width" => Action::SwitchPresetColumnWidth,
        "switch-preset-column-width-back" => Action::SwitchPresetColumnWidthBack,
        "switch-preset-window-width" => Action::SwitchPresetWindowWidth,
        "switch-preset-window-width-back" => Action::SwitchPresetWindowWidthBack,
        "switch-preset-window-height" => Action::SwitchPresetWindowHeight,
        "switch-preset-window-height-back" => Action::SwitchPresetWindowHeightBack,

        // Window consumption and expulsion
        "consume-or-expel-window-left" => Action::ConsumeOrExpelWindowLeft,
        "consume-or-expel-window-right" => Action::ConsumeOrExpelWindowRight,
        "consume-window-into-column" => Action::ConsumeWindowIntoColumn,
        "expel-window-from-column" => Action::ExpelWindowFromColumn,

        // Window and column organization
        "swap-window-left" => Action::SwapWindowLeft,
        "swap-window-right" => Action::SwapWindowRight,
        "toggle-column-tabbed-display" => Action::ToggleColumnTabbedDisplay,
        "center-window" => Action::CenterWindow,
        "move-window-to-floating" => Action::MoveWindowToFloating,
        "move-window-to-tiling" => Action::MoveWindowToTiling,
        "focus-floating" => Action::FocusFloating,
        "focus-tiling" => Action::FocusTiling,
        "switch-focus-between-floating-and-tiling" => Action::SwitchFocusBetweenFloatingAndTiling,

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
                    warn!(
                        "⚠ Failed to parse workspace index from: {}",
                        lua_binding.args[0]
                    );
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
                    warn!(
                        "⚠ Failed to parse workspace index from: {}",
                        lua_binding.args[0]
                    );
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
                    warn!(
                        "⚠ Failed to parse size change from: {}",
                        lua_binding.args[0]
                    );
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
                    warn!(
                        "⚠ Failed to parse size change from: {}",
                        lua_binding.args[0]
                    );
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

        // Window sizing
        "reset-window-height" => Action::ResetWindowHeight,
        "expand-column-to-available-width" => Action::ExpandColumnToAvailableWidth,

        // Keyboard shortcuts inhibit
        "toggle-keyboard-shortcuts-inhibit" => Action::ToggleKeyboardShortcutsInhibit,

        // Unsupported actions requiring arguments
        "move-column-to-monitor" | "move-window-to-monitor" | "focus-monitor" => {
            warn!(
                "⚠ Action '{}' requires arguments that aren't fully supported yet in Lua config",
                lua_binding.action
            );
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

/// Helper function to parse hex color strings like "#1e1e2e" or "#1e1e2eff"
fn parse_hex_color(s: &str) -> Option<niri_config::Color> {
    let s = s.trim_start_matches('#');

    // Support both RGB and RGBA formats
    if s.len() == 6 {
        // RGB format - assume full opacity
        let r = u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0;
        Some(niri_config::Color { r, g, b, a: 1.0 })
    } else if s.len() == 8 {
        // RGBA format
        let r = u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0;
        let a = u8::from_str_radix(&s[6..8], 16).ok()? as f32 / 255.0;
        Some(niri_config::Color { r, g, b, a })
    } else {
        None
    }
}

/// Helper function to parse preset size from Lua value
/// Accepts either a number (for proportion 0.0-1.0) or a table with { proportion = 0.5 } or { fixed
/// = 800 }
fn parse_preset_size(value: &mlua::Value) -> Option<niri_config::layout::PresetSize> {
    use niri_config::layout::PresetSize;

    match value {
        mlua::Value::Number(n) => {
            // Direct number is interpreted as proportion (0.0 to 1.0)
            Some(PresetSize::Proportion(*n))
        }
        mlua::Value::Table(table) => {
            // Check for { proportion = X } format
            if let Ok(prop) = table.get::<f64>("proportion") {
                return Some(PresetSize::Proportion(prop));
            }
            // Check for { fixed = X } format
            if let Ok(fixed) = table.get::<i32>("fixed") {
                return Some(PresetSize::Fixed(fixed));
            }
            None
        }
        _ => None,
    }
}

/// Helper function to parse color from Lua table or string
fn parse_color_value(value: &mlua::Value) -> Option<niri_config::Color> {
    match value {
        mlua::Value::Table(table) => {
            // Try RGBA table format { r = 1.0, g = 0.5, b = 0.3, a = 1.0 }
            if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                table.get::<f32>("r"),
                table.get::<f32>("g"),
                table.get::<f32>("b"),
                table.get::<f32>("a"),
            ) {
                return Some(niri_config::Color { r, g, b, a });
            }
            None
        }
        mlua::Value::String(s) => {
            // Try hex string format "#RRGGBB" or "#RRGGBBAA"
            if let Ok(s_str) = s.to_str() {
                parse_hex_color(&s_str)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper function to parse border-like configuration (focus_ring, border)
/// These share the same structure: off, width, active/inactive/urgent colors/gradients
fn parse_border_config(
    table: &mlua::Table,
) -> Result<(
    bool,
    Option<f64>,
    Option<niri_config::Color>,
    Option<niri_config::Color>,
    Option<niri_config::Color>,
)> {
    let mut off = false;
    let mut on = false;
    let mut width = None;
    let mut active_color = None;
    let mut inactive_color = None;
    let mut urgent_color = None;

    // Extract on/off flags
    if let Ok(off_value) = table.get::<bool>("off") {
        off = off_value;
    }
    if let Ok(on_value) = table.get::<bool>("on") {
        on = on_value;
    }

    // 'on' takes precedence and sets off to false
    if on {
        off = false;
    }

    // Extract width
    if let Ok(w) = table.get::<f64>("width") {
        width = Some(w);
    }

    // Extract colors
    if let Ok(color_value) = table.get::<mlua::Value>("active_color") {
        active_color = parse_color_value(&color_value);
    }
    if let Ok(color_value) = table.get::<mlua::Value>("inactive_color") {
        inactive_color = parse_color_value(&color_value);
    }
    if let Ok(color_value) = table.get::<mlua::Value>("urgent_color") {
        urgent_color = parse_color_value(&color_value);
    }

    Ok((off, width, active_color, inactive_color, urgent_color))
}

/// Helper function to parse animation curve from Lua table
/// Returns either a Spring or Easing curve based on the provided parameters
fn parse_animation_curve(table: &mlua::Table) -> niri_config::animations::Kind {
    use mlua::Value;
    use niri_config::animations::{Curve, EasingParams, Kind, SpringParams};

    // Check if this is a spring animation (has damping_ratio, stiffness, epsilon)
    // We need to check that the value exists AND is not Nil
    let has_spring_params = matches!(
        table.get::<Value>("damping_ratio"),
        Ok(Value::Number(_)) | Ok(Value::Integer(_))
    ) || matches!(
        table.get::<Value>("stiffness"),
        Ok(Value::Number(_)) | Ok(Value::Integer(_))
    ) || matches!(
        table.get::<Value>("epsilon"),
        Ok(Value::Number(_)) | Ok(Value::Integer(_))
    );

    if has_spring_params {
        let damping_ratio = table.get::<f64>("damping_ratio").unwrap_or(1.0);
        let stiffness = table.get::<u32>("stiffness").unwrap_or(1000);
        let epsilon = table.get::<f64>("epsilon").unwrap_or(0.0001);

        Kind::Spring(SpringParams {
            damping_ratio,
            stiffness,
            epsilon,
        })
    }
    // Check if this is an easing animation (has duration_ms, curve)
    else if table.get::<mlua::Value>("duration_ms").is_ok()
        || table.get::<mlua::Value>("curve").is_ok()
    {
        let duration_ms = table.get::<u32>("duration_ms").unwrap_or(150);

        // Parse curve type (default to ease-out-expo)
        let curve = if let Ok(curve_str) = table.get::<String>("curve") {
            match curve_str.to_lowercase().as_str() {
                "linear" => Curve::Linear,
                "ease-out-quad" | "ease_out_quad" => Curve::EaseOutQuad,
                "ease-out-cubic" | "ease_out_cubic" => Curve::EaseOutCubic,
                "ease-out-expo" | "ease_out_expo" => Curve::EaseOutExpo,
                other => {
                    warn!(
                        "Unknown curve type '{}', defaulting to ease-out-expo",
                        other
                    );
                    Curve::EaseOutExpo
                }
            }
        } else {
            Curve::EaseOutExpo
        };

        Kind::Easing(EasingParams { duration_ms, curve })
    } else {
        // Default to spring if no parameters specified
        Kind::Spring(SpringParams {
            damping_ratio: 1.0,
            stiffness: 1000,
            epsilon: 0.0001,
        })
    }
}

/// Helper function to parse Animation struct from Lua table
fn parse_animation(table: &mlua::Table) -> niri_config::animations::Animation {
    use niri_config::animations::Animation;

    let mut off = false;
    let mut on = false;

    // Extract on/off flags
    if let Ok(off_value) = table.get::<bool>("off") {
        off = off_value;
    }
    if let Ok(on_value) = table.get::<bool>("on") {
        on = on_value;
    }

    // 'on' takes precedence and sets off to false
    if on {
        off = false;
    }

    // Check if there's a nested "spring" table
    let kind = if let Ok(spring_table) = table.get::<mlua::Table>("spring") {
        parse_animation_curve(&spring_table)
    } else {
        // Otherwise parse directly from the table (for easing animations)
        parse_animation_curve(table)
    };

    Animation { off, kind }
}

pub fn apply_lua_config(runtime: &LuaRuntime, config: &mut Config) -> Result<()> {
    debug!("=== Applying Lua configuration to Config ===");

    // Try to extract simple boolean settings
    debug!("Checking for prefer_no_csd in Lua globals");
    if runtime.has_global("prefer_no_csd") {
        info!("✓ Found prefer_no_csd in Lua globals");
        match runtime.get_global_bool_opt("prefer_no_csd") {
            Ok(Some(prefer_no_csd)) => {
                info!(
                    "✓ Applying prefer_no_csd: {} → {} (changed: {})",
                    config.prefer_no_csd,
                    prefer_no_csd,
                    config.prefer_no_csd != prefer_no_csd
                );
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
            debug!(
                "[apply_lua_config] get_keybindings returned {} bindings",
                raw_keybindings.len()
            );
            if raw_keybindings.is_empty() {
                info!("ℹ No keybindings found in Lua configuration");
            } else {
                info!("✓ Found {} keybindings in Lua", raw_keybindings.len());
                let mut converted_binds = Vec::new();

                for (key, action, args) in raw_keybindings {
                    // debug!(
                    //     "[apply_lua_config] Processing binding: key='{}', action='{}'",
                    //     key, action
                    // );
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
                    info!(
                        "✓ Successfully converted {} keybindings",
                        converted_binds.len()
                    );
                    // Merge with existing binds or replace them
                    config.binds.0.extend(converted_binds);
                    // Diagnostic log: print a sample of the added binds
                    debug!(
                        "[apply_lua_config] Sample converted binds: {:?}",
                        &config.binds.0.iter().rev().take(5).collect::<Vec<_>>()
                    );
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
            debug!(
                "[apply_lua_config] get_startup_commands returned {} commands",
                startup_cmds.len()
            );
            if startup_cmds.is_empty() {
                info!("ℹ No startup commands found in Lua configuration");
            } else {
                info!("✓ Found {} startup commands in Lua", startup_cmds.len());
                let mut spawn_at_startup = Vec::new();

                for cmd_vec in startup_cmds {
                    if cmd_vec.is_empty() {
                        continue;
                    }

                    debug!(
                        "[apply_lua_config] Processing startup command: {:?}",
                        cmd_vec
                    );

                    // For now, treat all commands as spawn-at-startup (execute as array)
                    // This matches the KDL syntax: spawn-at-startup "cmd" "arg1" "arg2"
                    spawn_at_startup.push(SpawnAtStartup { command: cmd_vec });
                }

                if !spawn_at_startup.is_empty() {
                    info!("✓ Added {} startup commands", spawn_at_startup.len());
                    config.spawn_at_startup.extend(spawn_at_startup);
                    debug!(
                        "[apply_lua_config] Sample spawn_at_startup entries: {:?}",
                        &config
                            .spawn_at_startup
                            .iter()
                            .rev()
                            .take(5)
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
        Err(e) => {
            warn!("✗ Error extracting startup commands from Lua: {}", e);
        }
    }

    // Extract and apply input configuration
    debug!("Checking for input configuration in Lua globals");
    if runtime.has_global("input") {
        info!("✓ Found input configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("input") {
            Ok(input_table) => {
                debug!("Processing input table");

                // Extract keyboard configuration
                if let Ok(keyboard_table) = input_table.get::<mlua::Table>("keyboard") {
                    debug!("Processing keyboard configuration");

                    // Extract xkb settings
                    if let Ok(xkb_table) = keyboard_table.get::<mlua::Table>("xkb") {
                        debug!("Processing xkb configuration");

                        // Extract layout
                        if let Ok(layout) = xkb_table.get::<String>("layout") {
                            if !layout.is_empty() {
                                debug!("Applying xkb.layout: {}", layout);
                                config.input.keyboard.xkb.layout = layout;
                            }
                        }

                        // Extract variant
                        if let Ok(variant) = xkb_table.get::<String>("variant") {
                            if !variant.is_empty() {
                                debug!("Applying xkb.variant: {}", variant);
                                config.input.keyboard.xkb.variant = variant;
                            }
                        }

                        // Extract options
                        if let Ok(options) = xkb_table.get::<String>("options") {
                            if !options.is_empty() {
                                debug!("Applying xkb.options: {}", options);
                                config.input.keyboard.xkb.options = Some(options);
                            }
                        }

                        // Extract model
                        if let Ok(model) = xkb_table.get::<String>("model") {
                            if !model.is_empty() {
                                debug!("Applying xkb.model: {}", model);
                                config.input.keyboard.xkb.model = model;
                            }
                        }

                        // Extract rules
                        if let Ok(rules) = xkb_table.get::<String>("rules") {
                            if !rules.is_empty() {
                                debug!("Applying xkb.rules: {}", rules);
                                config.input.keyboard.xkb.rules = rules;
                            }
                        }

                        info!("✓ Applied xkb configuration from Lua");
                    }

                    // Extract repeat_delay
                    if let Ok(repeat_delay) = keyboard_table.get::<u16>("repeat_delay") {
                        debug!("Applying keyboard.repeat_delay: {}", repeat_delay);
                        config.input.keyboard.repeat_delay = repeat_delay;
                    }

                    // Extract repeat_rate
                    if let Ok(repeat_rate) = keyboard_table.get::<u8>("repeat_rate") {
                        debug!("Applying keyboard.repeat_rate: {}", repeat_rate);
                        config.input.keyboard.repeat_rate = repeat_rate;
                    }

                    // Extract numlock
                    if let Ok(numlock) = keyboard_table.get::<bool>("numlock") {
                        debug!("Applying keyboard.numlock: {}", numlock);
                        config.input.keyboard.numlock = numlock;
                    }

                    info!("✓ Applied keyboard configuration from Lua");
                }

                // Extract touchpad configuration
                if let Ok(touchpad_table) = input_table.get::<mlua::Table>("touchpad") {
                    debug!("Processing touchpad configuration");

                    if let Ok(off) = touchpad_table.get::<bool>("off") {
                        debug!("Applying touchpad.off: {}", off);
                        config.input.touchpad.off = off;
                    }

                    if let Ok(tap) = touchpad_table.get::<bool>("tap") {
                        debug!("Applying touchpad.tap: {}", tap);
                        config.input.touchpad.tap = tap;
                    }

                    if let Ok(dwt) = touchpad_table.get::<bool>("dwt") {
                        debug!("Applying touchpad.dwt: {}", dwt);
                        config.input.touchpad.dwt = dwt;
                    }

                    if let Ok(dwtp) = touchpad_table.get::<bool>("dwtp") {
                        debug!("Applying touchpad.dwtp: {}", dwtp);
                        config.input.touchpad.dwtp = dwtp;
                    }

                    if let Ok(drag) = touchpad_table.get::<bool>("drag") {
                        debug!("Applying touchpad.drag: {}", drag);
                        config.input.touchpad.drag = Some(drag);
                    }

                    if let Ok(drag_lock) = touchpad_table.get::<bool>("drag_lock") {
                        debug!("Applying touchpad.drag_lock: {}", drag_lock);
                        config.input.touchpad.drag_lock = drag_lock;
                    }

                    if let Ok(natural_scroll) = touchpad_table.get::<bool>("natural_scroll") {
                        debug!("Applying touchpad.natural_scroll: {}", natural_scroll);
                        config.input.touchpad.natural_scroll = natural_scroll;
                    }

                    if let Ok(accel_speed) = touchpad_table.get::<f64>("accel_speed") {
                        debug!("Applying touchpad.accel_speed: {}", accel_speed);
                        config.input.touchpad.accel_speed = FloatOrInt(accel_speed);
                    }

                    if let Ok(accel_profile) = touchpad_table.get::<String>("accel_profile") {
                        debug!("Applying touchpad.accel_profile: {}", accel_profile);
                        config.input.touchpad.accel_profile =
                            match accel_profile.to_lowercase().as_str() {
                                "flat" => Some(input::AccelProfile::Flat),
                                "adaptive" => Some(input::AccelProfile::Adaptive),
                                _ => {
                                    warn!("Invalid accel_profile value: {}", accel_profile);
                                    None
                                }
                            };
                    }

                    if let Ok(scroll_method) = touchpad_table.get::<String>("scroll_method") {
                        debug!("Applying touchpad.scroll_method: {}", scroll_method);
                        config.input.touchpad.scroll_method =
                            match scroll_method.to_lowercase().replace("-", "").as_str() {
                                "noscroll" => Some(input::ScrollMethod::NoScroll),
                                "twofinger" => Some(input::ScrollMethod::TwoFinger),
                                "edge" => Some(input::ScrollMethod::Edge),
                                "onbuttondown" => Some(input::ScrollMethod::OnButtonDown),
                                _ => {
                                    warn!("Invalid scroll_method value: {}", scroll_method);
                                    None
                                }
                            };
                    }

                    if let Ok(click_method) = touchpad_table.get::<String>("click_method") {
                        debug!("Applying touchpad.click_method: {}", click_method);
                        config.input.touchpad.click_method =
                            match click_method.to_lowercase().replace("-", "").as_str() {
                                "clickfinger" => Some(input::ClickMethod::Clickfinger),
                                "buttonareas" => Some(input::ClickMethod::ButtonAreas),
                                _ => {
                                    warn!("Invalid click_method value: {}", click_method);
                                    None
                                }
                            };
                    }

                    if let Ok(left_handed) = touchpad_table.get::<bool>("left_handed") {
                        debug!("Applying touchpad.left_handed: {}", left_handed);
                        config.input.touchpad.left_handed = left_handed;
                    }

                    if let Ok(scroll_button) = touchpad_table.get::<u32>("scroll_button") {
                        debug!("Applying touchpad.scroll_button: {}", scroll_button);
                        config.input.touchpad.scroll_button = Some(scroll_button);
                    }

                    if let Ok(scroll_button_lock) = touchpad_table.get::<bool>("scroll_button_lock")
                    {
                        debug!(
                            "Applying touchpad.scroll_button_lock: {}",
                            scroll_button_lock
                        );
                        config.input.touchpad.scroll_button_lock = scroll_button_lock;
                    }

                    if let Ok(tap_button_map) = touchpad_table.get::<String>("tap_button_map") {
                        debug!("Applying touchpad.tap_button_map: {}", tap_button_map);
                        config.input.touchpad.tap_button_map =
                            match tap_button_map.to_lowercase().replace("-", "").as_str() {
                                "leftrightmiddle" => Some(input::TapButtonMap::LeftRightMiddle),
                                "leftmiddleright" => Some(input::TapButtonMap::LeftMiddleRight),
                                _ => {
                                    warn!("Invalid tap_button_map value: {}", tap_button_map);
                                    None
                                }
                            };
                    }

                    info!("✓ Applied touchpad configuration from Lua");
                }

                // Extract mouse configuration
                if let Ok(mouse_table) = input_table.get::<mlua::Table>("mouse") {
                    debug!("Processing mouse configuration");

                    if let Ok(off) = mouse_table.get::<bool>("off") {
                        debug!("Applying mouse.off: {}", off);
                        config.input.mouse.off = off;
                    }

                    if let Ok(natural_scroll) = mouse_table.get::<bool>("natural_scroll") {
                        debug!("Applying mouse.natural_scroll: {}", natural_scroll);
                        config.input.mouse.natural_scroll = natural_scroll;
                    }

                    if let Ok(accel_speed) = mouse_table.get::<f64>("accel_speed") {
                        debug!("Applying mouse.accel_speed: {}", accel_speed);
                        config.input.mouse.accel_speed = FloatOrInt(accel_speed);
                    }

                    if let Ok(accel_profile) = mouse_table.get::<String>("accel_profile") {
                        debug!("Applying mouse.accel_profile: {}", accel_profile);
                        config.input.mouse.accel_profile =
                            match accel_profile.to_lowercase().as_str() {
                                "flat" => Some(input::AccelProfile::Flat),
                                "adaptive" => Some(input::AccelProfile::Adaptive),
                                _ => {
                                    warn!("Invalid accel_profile value: {}", accel_profile);
                                    None
                                }
                            };
                    }

                    if let Ok(scroll_method) = mouse_table.get::<String>("scroll_method") {
                        debug!("Applying mouse.scroll_method: {}", scroll_method);
                        config.input.mouse.scroll_method =
                            match scroll_method.to_lowercase().replace("-", "").as_str() {
                                "noscroll" => Some(input::ScrollMethod::NoScroll),
                                "twofinger" => Some(input::ScrollMethod::TwoFinger),
                                "edge" => Some(input::ScrollMethod::Edge),
                                "onbuttondown" => Some(input::ScrollMethod::OnButtonDown),
                                _ => {
                                    warn!("Invalid scroll_method value: {}", scroll_method);
                                    None
                                }
                            };
                    }

                    if let Ok(left_handed) = mouse_table.get::<bool>("left_handed") {
                        debug!("Applying mouse.left_handed: {}", left_handed);
                        config.input.mouse.left_handed = left_handed;
                    }

                    if let Ok(middle_emulation) = mouse_table.get::<bool>("middle_emulation") {
                        debug!("Applying mouse.middle_emulation: {}", middle_emulation);
                        config.input.mouse.middle_emulation = middle_emulation;
                    }

                    if let Ok(scroll_button) = mouse_table.get::<u32>("scroll_button") {
                        debug!("Applying mouse.scroll_button: {}", scroll_button);
                        config.input.mouse.scroll_button = Some(scroll_button);
                    }

                    if let Ok(scroll_button_lock) = mouse_table.get::<bool>("scroll_button_lock") {
                        debug!("Applying mouse.scroll_button_lock: {}", scroll_button_lock);
                        config.input.mouse.scroll_button_lock = scroll_button_lock;
                    }

                    info!("✓ Applied mouse configuration from Lua");
                }

                // Extract trackpoint configuration
                if let Ok(trackpoint_table) = input_table.get::<mlua::Table>("trackpoint") {
                    debug!("Processing trackpoint configuration");

                    if let Ok(off) = trackpoint_table.get::<bool>("off") {
                        debug!("Applying trackpoint.off: {}", off);
                        config.input.trackpoint.off = off;
                    }

                    if let Ok(natural_scroll) = trackpoint_table.get::<bool>("natural_scroll") {
                        debug!("Applying trackpoint.natural_scroll: {}", natural_scroll);
                        config.input.trackpoint.natural_scroll = natural_scroll;
                    }

                    if let Ok(accel_speed) = trackpoint_table.get::<f64>("accel_speed") {
                        debug!("Applying trackpoint.accel_speed: {}", accel_speed);
                        config.input.trackpoint.accel_speed = FloatOrInt(accel_speed);
                    }

                    if let Ok(accel_profile) = trackpoint_table.get::<String>("accel_profile") {
                        debug!("Applying trackpoint.accel_profile: {}", accel_profile);
                        config.input.trackpoint.accel_profile =
                            match accel_profile.to_lowercase().as_str() {
                                "flat" => Some(input::AccelProfile::Flat),
                                "adaptive" => Some(input::AccelProfile::Adaptive),
                                _ => {
                                    warn!("Invalid accel_profile value: {}", accel_profile);
                                    None
                                }
                            };
                    }

                    if let Ok(scroll_method) = trackpoint_table.get::<String>("scroll_method") {
                        debug!("Applying trackpoint.scroll_method: {}", scroll_method);
                        config.input.trackpoint.scroll_method =
                            match scroll_method.to_lowercase().replace("-", "").as_str() {
                                "noscroll" => Some(input::ScrollMethod::NoScroll),
                                "twofinger" => Some(input::ScrollMethod::TwoFinger),
                                "edge" => Some(input::ScrollMethod::Edge),
                                "onbuttondown" => Some(input::ScrollMethod::OnButtonDown),
                                _ => {
                                    warn!("Invalid scroll_method value: {}", scroll_method);
                                    None
                                }
                            };
                    }

                    if let Ok(left_handed) = trackpoint_table.get::<bool>("left_handed") {
                        debug!("Applying trackpoint.left_handed: {}", left_handed);
                        config.input.trackpoint.left_handed = left_handed;
                    }

                    if let Ok(middle_emulation) = trackpoint_table.get::<bool>("middle_emulation") {
                        debug!("Applying trackpoint.middle_emulation: {}", middle_emulation);
                        config.input.trackpoint.middle_emulation = middle_emulation;
                    }

                    if let Ok(scroll_button) = trackpoint_table.get::<u32>("scroll_button") {
                        debug!("Applying trackpoint.scroll_button: {}", scroll_button);
                        config.input.trackpoint.scroll_button = Some(scroll_button);
                    }

                    if let Ok(scroll_button_lock) =
                        trackpoint_table.get::<bool>("scroll_button_lock")
                    {
                        debug!(
                            "Applying trackpoint.scroll_button_lock: {}",
                            scroll_button_lock
                        );
                        config.input.trackpoint.scroll_button_lock = scroll_button_lock;
                    }

                    info!("✓ Applied trackpoint configuration from Lua");
                }

                // Extract trackball configuration
                if let Ok(trackball_table) = input_table.get::<mlua::Table>("trackball") {
                    debug!("Processing trackball configuration");

                    if let Ok(off) = trackball_table.get::<bool>("off") {
                        debug!("Applying trackball.off: {}", off);
                        config.input.trackball.off = off;
                    }

                    if let Ok(natural_scroll) = trackball_table.get::<bool>("natural_scroll") {
                        debug!("Applying trackball.natural_scroll: {}", natural_scroll);
                        config.input.trackball.natural_scroll = natural_scroll;
                    }

                    if let Ok(accel_speed) = trackball_table.get::<f64>("accel_speed") {
                        debug!("Applying trackball.accel_speed: {}", accel_speed);
                        config.input.trackball.accel_speed = FloatOrInt(accel_speed);
                    }

                    if let Ok(accel_profile) = trackball_table.get::<String>("accel_profile") {
                        debug!("Applying trackball.accel_profile: {}", accel_profile);
                        config.input.trackball.accel_profile =
                            match accel_profile.to_lowercase().as_str() {
                                "flat" => Some(input::AccelProfile::Flat),
                                "adaptive" => Some(input::AccelProfile::Adaptive),
                                _ => {
                                    warn!("Invalid accel_profile value: {}", accel_profile);
                                    None
                                }
                            };
                    }

                    if let Ok(scroll_method) = trackball_table.get::<String>("scroll_method") {
                        debug!("Applying trackball.scroll_method: {}", scroll_method);
                        config.input.trackball.scroll_method =
                            match scroll_method.to_lowercase().replace("-", "").as_str() {
                                "noscroll" => Some(input::ScrollMethod::NoScroll),
                                "twofinger" => Some(input::ScrollMethod::TwoFinger),
                                "edge" => Some(input::ScrollMethod::Edge),
                                "onbuttondown" => Some(input::ScrollMethod::OnButtonDown),
                                _ => {
                                    warn!("Invalid scroll_method value: {}", scroll_method);
                                    None
                                }
                            };
                    }

                    if let Ok(left_handed) = trackball_table.get::<bool>("left_handed") {
                        debug!("Applying trackball.left_handed: {}", left_handed);
                        config.input.trackball.left_handed = left_handed;
                    }

                    if let Ok(middle_emulation) = trackball_table.get::<bool>("middle_emulation") {
                        debug!("Applying trackball.middle_emulation: {}", middle_emulation);
                        config.input.trackball.middle_emulation = middle_emulation;
                    }

                    if let Ok(scroll_button) = trackball_table.get::<u32>("scroll_button") {
                        debug!("Applying trackball.scroll_button: {}", scroll_button);
                        config.input.trackball.scroll_button = Some(scroll_button);
                    }

                    if let Ok(scroll_button_lock) =
                        trackball_table.get::<bool>("scroll_button_lock")
                    {
                        debug!(
                            "Applying trackball.scroll_button_lock: {}",
                            scroll_button_lock
                        );
                        config.input.trackball.scroll_button_lock = scroll_button_lock;
                    }

                    info!("✓ Applied trackball configuration from Lua");
                }

                // Extract tablet configuration
                if let Ok(tablet_table) = input_table.get::<mlua::Table>("tablet") {
                    debug!("Processing tablet configuration");

                    if let Ok(off) = tablet_table.get::<bool>("off") {
                        debug!("Applying tablet.off: {}", off);
                        config.input.tablet.off = off;
                    }

                    if let Ok(left_handed) = tablet_table.get::<bool>("left_handed") {
                        debug!("Applying tablet.left_handed: {}", left_handed);
                        config.input.tablet.left_handed = left_handed;
                    }

                    if let Ok(map_to_output) = tablet_table.get::<String>("map_to_output") {
                        debug!("Applying tablet.map_to_output: {}", map_to_output);
                        config.input.tablet.map_to_output = Some(map_to_output);
                    }

                    if let Ok(calibration_matrix) =
                        tablet_table.get::<mlua::Table>("calibration_matrix")
                    {
                        let mut matrix = Vec::new();
                        if let Ok(len) = calibration_matrix.len() {
                            for i in 1..=len {
                                if let Ok(value) = calibration_matrix.get::<f32>(i) {
                                    matrix.push(value);
                                }
                            }
                        }
                        if !matrix.is_empty() {
                            debug!("Applying tablet.calibration_matrix: {:?}", matrix);
                            config.input.tablet.calibration_matrix = Some(matrix);
                        }
                    }

                    info!("✓ Applied tablet configuration from Lua");
                }

                // Extract touch configuration
                if let Ok(touch_table) = input_table.get::<mlua::Table>("touch") {
                    debug!("Processing touch configuration");

                    if let Ok(off) = touch_table.get::<bool>("off") {
                        debug!("Applying touch.off: {}", off);
                        config.input.touch.off = off;
                    }

                    if let Ok(map_to_output) = touch_table.get::<String>("map_to_output") {
                        debug!("Applying touch.map_to_output: {}", map_to_output);
                        config.input.touch.map_to_output = Some(map_to_output);
                    }

                    if let Ok(calibration_matrix) =
                        touch_table.get::<mlua::Table>("calibration_matrix")
                    {
                        let mut matrix = Vec::new();
                        if let Ok(len) = calibration_matrix.len() {
                            for i in 1..=len {
                                if let Ok(value) = calibration_matrix.get::<f32>(i) {
                                    matrix.push(value);
                                }
                            }
                        }
                        if !matrix.is_empty() {
                            debug!("Applying touch.calibration_matrix: {:?}", matrix);
                            config.input.touch.calibration_matrix = Some(matrix);
                        }
                    }

                    info!("✓ Applied touch configuration from Lua");
                }

                // Extract global input settings
                if let Ok(disable_power_key) = input_table.get::<bool>("disable_power_key_handling")
                {
                    debug!("Applying disable_power_key_handling: {}", disable_power_key);
                    config.input.disable_power_key_handling = disable_power_key;
                }

                if let Ok(workspace_auto_back_and_forth) =
                    input_table.get::<bool>("workspace_auto_back_and_forth")
                {
                    debug!(
                        "Applying workspace_auto_back_and_forth: {}",
                        workspace_auto_back_and_forth
                    );
                    config.input.workspace_auto_back_and_forth = workspace_auto_back_and_forth;
                }

                // Extract warp_mouse_to_focus
                if let Ok(warp_table) = input_table.get::<mlua::Table>("warp_mouse_to_focus") {
                    debug!("Processing warp_mouse_to_focus configuration");
                    let mut warp_config = input::WarpMouseToFocus { mode: None };

                    if let Ok(mode) = warp_table.get::<String>("mode") {
                        debug!("Applying warp_mouse_to_focus.mode: {}", mode);
                        warp_config.mode = match mode.to_lowercase().replace("-", "").as_str() {
                            "centerxy" => Some(input::WarpMouseToFocusMode::CenterXy),
                            "centerxyalways" => Some(input::WarpMouseToFocusMode::CenterXyAlways),
                            _ => {
                                warn!("Invalid warp_mouse_to_focus mode: {}", mode);
                                None
                            }
                        };
                    }

                    config.input.warp_mouse_to_focus = Some(warp_config);
                    info!("✓ Applied warp_mouse_to_focus configuration from Lua");
                } else if let Ok(enabled) = input_table.get::<bool>("warp_mouse_to_focus") {
                    // Support simple boolean format: warp_mouse_to_focus = true
                    if enabled {
                        debug!("Applying warp_mouse_to_focus: enabled (default mode)");
                        config.input.warp_mouse_to_focus =
                            Some(input::WarpMouseToFocus { mode: None });
                    }
                }

                // Extract focus_follows_mouse
                if let Ok(focus_table) = input_table.get::<mlua::Table>("focus_follows_mouse") {
                    debug!("Processing focus_follows_mouse configuration");
                    let mut focus_config = input::FocusFollowsMouse {
                        max_scroll_amount: None,
                    };

                    if let Ok(max_scroll) = focus_table.get::<f64>("max_scroll_amount") {
                        debug!(
                            "Applying focus_follows_mouse.max_scroll_amount: {}",
                            max_scroll
                        );
                        focus_config.max_scroll_amount =
                            Some(niri_config::utils::Percent(max_scroll));
                    }

                    config.input.focus_follows_mouse = Some(focus_config);
                    info!("✓ Applied focus_follows_mouse configuration from Lua");
                } else if let Ok(enabled) = input_table.get::<bool>("focus_follows_mouse") {
                    // Support simple boolean format: focus_follows_mouse = true
                    if enabled {
                        debug!("Applying focus_follows_mouse: enabled");
                        config.input.focus_follows_mouse = Some(input::FocusFollowsMouse {
                            max_scroll_amount: None,
                        });
                    }
                }

                // Extract mod_key
                if let Ok(mod_key_str) = input_table.get::<String>("mod_key") {
                    debug!("Applying mod_key: {}", mod_key_str);
                    config.input.mod_key =
                        match mod_key_str.to_lowercase().replace("-", "").as_str() {
                            "ctrl" => Some(input::ModKey::Ctrl),
                            "shift" => Some(input::ModKey::Shift),
                            "alt" => Some(input::ModKey::Alt),
                            "super" => Some(input::ModKey::Super),
                            "isolevel3shift" => Some(input::ModKey::IsoLevel3Shift),
                            "isolevel5shift" => Some(input::ModKey::IsoLevel5Shift),
                            _ => {
                                warn!("Invalid mod_key value: {}", mod_key_str);
                                None
                            }
                        };
                }

                // Extract mod_key_nested
                if let Ok(mod_key_nested_str) = input_table.get::<String>("mod_key_nested") {
                    debug!("Applying mod_key_nested: {}", mod_key_nested_str);
                    config.input.mod_key_nested =
                        match mod_key_nested_str.to_lowercase().replace("-", "").as_str() {
                            "ctrl" => Some(input::ModKey::Ctrl),
                            "shift" => Some(input::ModKey::Shift),
                            "alt" => Some(input::ModKey::Alt),
                            "super" => Some(input::ModKey::Super),
                            "isolevel3shift" => Some(input::ModKey::IsoLevel3Shift),
                            "isolevel5shift" => Some(input::ModKey::IsoLevel5Shift),
                            _ => {
                                warn!("Invalid mod_key_nested value: {}", mod_key_nested_str);
                                None
                            }
                        };
                }
            }
            Err(e) => {
                warn!("✗ Error extracting input configuration from Lua: {}", e);
            }
        }
    } else {
        debug!("ℹ input configuration not found in Lua globals");
    }

    // Extract output configuration
    if runtime.has_global("outputs") {
        debug!("Found outputs configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Value>("outputs") {
            Ok(mlua::Value::Table(outputs_array)) => {
                debug!("Processing outputs array");

                for pair in outputs_array.pairs::<mlua::Value, mlua::Table>() {
                    let (_idx, output_table) = match pair {
                        Ok(p) => p,
                        Err(e) => {
                            warn!("Error iterating outputs: {}", e);
                            continue;
                        }
                    };

                    // Extract output name (required)
                    let name: String = match output_table.get("name") {
                        Ok(name) => name,
                        Err(_) => {
                            warn!("Output configuration missing required 'name' field, skipping");
                            continue;
                        }
                    };

                    debug!("Processing output: {}", name);

                    let mut output = niri_config::output::Output::default();
                    output.name = name.clone();

                    // Extract off setting
                    if let Ok(off) = output_table.get::<bool>("off") {
                        debug!("  off: {}", off);
                        output.off = off;
                    }

                    // Extract scale
                    if let Ok(scale) = output_table.get::<f64>("scale") {
                        debug!("  scale: {}", scale);
                        output.scale = Some(FloatOrInt(scale));
                    }

                    // Extract transform
                    if let Ok(transform_str) = output_table.get::<String>("transform") {
                        debug!("  transform: {}", transform_str);
                        output.transform = match transform_str.to_lowercase().as_str() {
                            "normal" => niri_ipc::Transform::Normal,
                            "90" => niri_ipc::Transform::_90,
                            "180" => niri_ipc::Transform::_180,
                            "270" => niri_ipc::Transform::_270,
                            "flipped" => niri_ipc::Transform::Flipped,
                            "flipped-90" | "flipped90" => niri_ipc::Transform::Flipped90,
                            "flipped-180" | "flipped180" => niri_ipc::Transform::Flipped180,
                            "flipped-270" | "flipped270" => niri_ipc::Transform::Flipped270,
                            _ => {
                                warn!("Invalid transform value: {}", transform_str);
                                niri_ipc::Transform::Normal
                            }
                        };
                    }

                    // Extract position
                    if let Ok(pos_table) = output_table.get::<mlua::Table>("position") {
                        if let (Ok(x), Ok(y)) =
                            (pos_table.get::<i32>("x"), pos_table.get::<i32>("y"))
                        {
                            debug!("  position: x={}, y={}", x, y);
                            output.position = Some(niri_config::output::Position { x, y });
                        }
                    }

                    // Extract mode (resolution and refresh rate)
                    if let Ok(mode_str) = output_table.get::<String>("mode") {
                        debug!("  mode: {}", mode_str);
                        // Parse mode string like "1920x1080@60" or "1920x1080"
                        if let Some((resolution, refresh_str)) = mode_str.split_once('@') {
                            // Has refresh rate
                            if let Some((width_str, height_str)) = resolution.split_once('x') {
                                if let (Ok(width), Ok(height), Ok(refresh)) = (
                                    width_str.parse::<u16>(),
                                    height_str.parse::<u16>(),
                                    refresh_str.parse::<f64>(),
                                ) {
                                    output.mode = Some(niri_config::output::Mode {
                                        custom: false,
                                        mode: niri_ipc::ConfiguredMode {
                                            width,
                                            height,
                                            refresh: Some(refresh),
                                        },
                                    });
                                }
                            }
                        } else if let Some((width_str, height_str)) = mode_str.split_once('x') {
                            // No refresh rate specified
                            if let (Ok(width), Ok(height)) =
                                (width_str.parse::<u16>(), height_str.parse::<u16>())
                            {
                                output.mode = Some(niri_config::output::Mode {
                                    custom: false,
                                    mode: niri_ipc::ConfiguredMode {
                                        width,
                                        height,
                                        refresh: None,
                                    },
                                });
                            }
                        }
                    }

                    // Extract variable refresh rate
                    if let Ok(vrr_table) = output_table.get::<mlua::Table>("variable_refresh_rate")
                    {
                        let on_demand = vrr_table.get::<bool>("on_demand").unwrap_or(false);
                        debug!("  variable_refresh_rate.on_demand: {}", on_demand);
                        output.variable_refresh_rate = Some(niri_config::output::Vrr { on_demand });
                    } else if let Ok(vrr_bool) = output_table.get::<bool>("variable_refresh_rate") {
                        // Support simple boolean format
                        if vrr_bool {
                            debug!("  variable_refresh_rate: enabled");
                            output.variable_refresh_rate =
                                Some(niri_config::output::Vrr { on_demand: false });
                        }
                    }

                    // Extract focus_at_startup
                    if let Ok(focus) = output_table.get::<bool>("focus_at_startup") {
                        debug!("  focus_at_startup: {}", focus);
                        output.focus_at_startup = focus;
                    }

                    info!("✓ Configured output: {}", name);
                    config.outputs.0.push(output);
                }

                info!(
                    "✓ Applied {} output configuration(s) from Lua",
                    config.outputs.0.len()
                );
            }
            Ok(_) => {
                debug!("ℹ outputs global exists but is not a table, skipping");
            }
            Err(e) => {
                debug!("ℹ Error extracting outputs: {}", e);
            }
        }
    } else {
        debug!("ℹ outputs configuration not found in Lua globals");
    }

    // Extract layout configuration
    if runtime.has_global("layout") {
        debug!("Found layout configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("layout") {
            Ok(layout_table) => {
                debug!("Processing layout configuration");

                // Extract gaps
                if let Ok(gaps) = layout_table.get::<f64>("gaps") {
                    debug!("  gaps: {}", gaps);
                    config.layout.gaps = gaps;
                }

                // Extract always_center_single_column
                if let Ok(center_single) = layout_table.get::<bool>("always_center_single_column") {
                    debug!("  always_center_single_column: {}", center_single);
                    config.layout.always_center_single_column = center_single;
                }

                // Extract empty_workspace_above_first
                if let Ok(empty_above) = layout_table.get::<bool>("empty_workspace_above_first") {
                    debug!("  empty_workspace_above_first: {}", empty_above);
                    config.layout.empty_workspace_above_first = empty_above;
                }

                // Extract center_focused_column
                if let Ok(center_str) = layout_table.get::<String>("center_focused_column") {
                    debug!("  center_focused_column: {}", center_str);
                    use niri_config::layout::CenterFocusedColumn;
                    config.layout.center_focused_column = match center_str.to_lowercase().as_str() {
                        "never" => CenterFocusedColumn::Never,
                        "always" => CenterFocusedColumn::Always,
                        "on-overflow" | "on_overflow" => CenterFocusedColumn::OnOverflow,
                        _ => {
                            warn!("Invalid center_focused_column value: {}", center_str);
                            CenterFocusedColumn::Never
                        }
                    };
                }

                // Extract default_column_display
                if let Ok(display_str) = layout_table.get::<String>("default_column_display") {
                    debug!("  default_column_display: {}", display_str);
                    config.layout.default_column_display = match display_str.to_lowercase().as_str()
                    {
                        "normal" => niri_ipc::ColumnDisplay::Normal,
                        "tab" | "tabbed" => niri_ipc::ColumnDisplay::Tabbed,
                        _ => {
                            warn!("Invalid default_column_display value: {}", display_str);
                            niri_ipc::ColumnDisplay::Normal
                        }
                    };
                }

                // Extract background_color
                if let Ok(color_table) = layout_table.get::<mlua::Table>("background_color") {
                    if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                        color_table.get::<f32>("r"),
                        color_table.get::<f32>("g"),
                        color_table.get::<f32>("b"),
                        color_table.get::<f32>("a"),
                    ) {
                        debug!("  background_color: rgba({}, {}, {}, {})", r, g, b, a);
                        config.layout.background_color = niri_config::Color { r, g, b, a };
                    }
                } else if let Ok(color_str) = layout_table.get::<String>("background_color") {
                    // Support hex color strings like "#1e1e2e"
                    debug!("  background_color (string): {}", color_str);
                    if let Some(color) = parse_hex_color(&color_str) {
                        config.layout.background_color = color;
                    } else {
                        warn!("Invalid color format: {}", color_str);
                    }
                }

                // Extract struts
                if let Ok(struts_table) = layout_table.get::<mlua::Table>("struts") {
                    debug!("Processing struts configuration");
                    use niri_config::layout::Struts;
                    let mut struts = Struts::default();

                    if let Ok(left) = struts_table.get::<f64>("left") {
                        debug!("  struts.left: {}", left);
                        struts.left = FloatOrInt(left);
                    }
                    if let Ok(right) = struts_table.get::<f64>("right") {
                        debug!("  struts.right: {}", right);
                        struts.right = FloatOrInt(right);
                    }
                    if let Ok(top) = struts_table.get::<f64>("top") {
                        debug!("  struts.top: {}", top);
                        struts.top = FloatOrInt(top);
                    }
                    if let Ok(bottom) = struts_table.get::<f64>("bottom") {
                        debug!("  struts.bottom: {}", bottom);
                        struts.bottom = FloatOrInt(bottom);
                    }

                    config.layout.struts = struts;
                }

                // Extract preset_column_widths
                if let Ok(widths_table) = layout_table.get::<mlua::Table>("preset_column_widths") {
                    debug!("Processing preset_column_widths configuration");
                    let mut widths = Vec::new();

                    // Iterate over the array
                    for pair in widths_table.pairs::<mlua::Value, mlua::Value>() {
                        match pair {
                            Ok((_, value)) => {
                                if let Some(preset_size) = parse_preset_size(&value) {
                                    debug!("  Added preset width: {:?}", preset_size);
                                    widths.push(preset_size);
                                } else {
                                    warn!("Invalid preset_column_width value");
                                }
                            }
                            Err(e) => {
                                warn!("Error reading preset_column_widths: {}", e);
                            }
                        }
                    }

                    if !widths.is_empty() {
                        config.layout.preset_column_widths = widths;
                    }
                }

                // Extract preset_window_heights
                if let Ok(heights_table) = layout_table.get::<mlua::Table>("preset_window_heights")
                {
                    debug!("Processing preset_window_heights configuration");
                    let mut heights = Vec::new();

                    // Iterate over the array
                    for pair in heights_table.pairs::<mlua::Value, mlua::Value>() {
                        match pair {
                            Ok((_, value)) => {
                                if let Some(preset_size) = parse_preset_size(&value) {
                                    debug!("  Added preset height: {:?}", preset_size);
                                    heights.push(preset_size);
                                } else {
                                    warn!("Invalid preset_window_height value");
                                }
                            }
                            Err(e) => {
                                warn!("Error reading preset_window_heights: {}", e);
                            }
                        }
                    }

                    if !heights.is_empty() {
                        config.layout.preset_window_heights = heights;
                    }
                }

                // Extract default_column_width
                if let Ok(default_width_value) =
                    layout_table.get::<mlua::Value>("default_column_width")
                {
                    if let Some(preset_size) = parse_preset_size(&default_width_value) {
                        debug!("  default_column_width: {:?}", preset_size);
                        config.layout.default_column_width = Some(preset_size);
                    } else {
                        warn!("Invalid default_column_width value");
                    }
                }

                // Extract focus_ring configuration
                if let Ok(focus_ring_table) = layout_table.get::<mlua::Table>("focus_ring") {
                    debug!("Processing focus_ring configuration");
                    if let Ok((off, width, active_color, inactive_color, urgent_color)) =
                        parse_border_config(&focus_ring_table)
                    {
                        config.layout.focus_ring.off = off;
                        if let Some(w) = width {
                            config.layout.focus_ring.width = w;
                        }
                        if let Some(color) = active_color {
                            config.layout.focus_ring.active_color = color;
                            debug!("  focus_ring.active_color set");
                        }
                        if let Some(color) = inactive_color {
                            config.layout.focus_ring.inactive_color = color;
                            debug!("  focus_ring.inactive_color set");
                        }
                        if let Some(color) = urgent_color {
                            config.layout.focus_ring.urgent_color = color;
                            debug!("  focus_ring.urgent_color set");
                        }
                    }
                }

                // Extract border configuration
                if let Ok(border_table) = layout_table.get::<mlua::Table>("border") {
                    debug!("Processing border configuration");
                    if let Ok((off, width, active_color, inactive_color, urgent_color)) =
                        parse_border_config(&border_table)
                    {
                        config.layout.border.off = off;
                        if let Some(w) = width {
                            config.layout.border.width = w;
                        }
                        if let Some(color) = active_color {
                            config.layout.border.active_color = color;
                            debug!("  border.active_color set");
                        }
                        if let Some(color) = inactive_color {
                            config.layout.border.inactive_color = color;
                            debug!("  border.inactive_color set");
                        }
                        if let Some(color) = urgent_color {
                            config.layout.border.urgent_color = color;
                            debug!("  border.urgent_color set");
                        }
                    }
                }

                // Extract insert_hint configuration
                if let Ok(hint_table) = layout_table.get::<mlua::Table>("insert_hint") {
                    debug!("Processing insert_hint configuration");

                    if let Ok(off_value) = hint_table.get::<bool>("off") {
                        config.layout.insert_hint.off = off_value;
                    }
                    if let Ok(on_value) = hint_table.get::<bool>("on") {
                        if on_value {
                            config.layout.insert_hint.off = false;
                        }
                    }

                    if let Ok(color_value) = hint_table.get::<mlua::Value>("color") {
                        if let Some(color) = parse_color_value(&color_value) {
                            config.layout.insert_hint.color = color;
                            debug!("  insert_hint.color set");
                        }
                    }
                }

                // Extract shadow configuration
                if let Ok(shadow_table) = layout_table.get::<mlua::Table>("shadow") {
                    debug!("Processing shadow configuration");

                    if let Ok(off_value) = shadow_table.get::<bool>("off") {
                        config.layout.shadow.on = !off_value;
                    }
                    if let Ok(on_value) = shadow_table.get::<bool>("on") {
                        config.layout.shadow.on = on_value;
                    }

                    if let Ok(softness) = shadow_table.get::<f64>("softness") {
                        config.layout.shadow.softness = softness;
                        debug!("  shadow.softness: {}", softness);
                    }

                    if let Ok(spread) = shadow_table.get::<f64>("spread") {
                        config.layout.shadow.spread = spread;
                        debug!("  shadow.spread: {}", spread);
                    }

                    if let Ok(draw_behind) = shadow_table.get::<bool>("draw_behind_window") {
                        config.layout.shadow.draw_behind_window = draw_behind;
                        debug!("  shadow.draw_behind_window: {}", draw_behind);
                    }

                    if let Ok(color_value) = shadow_table.get::<mlua::Value>("color") {
                        if let Some(color) = parse_color_value(&color_value) {
                            config.layout.shadow.color = color;
                            debug!("  shadow.color set");
                        }
                    }

                    if let Ok(color_value) = shadow_table.get::<mlua::Value>("inactive_color") {
                        if let Some(color) = parse_color_value(&color_value) {
                            config.layout.shadow.inactive_color = Some(color);
                            debug!("  shadow.inactive_color set");
                        }
                    }

                    // Extract shadow offset
                    if let Ok(offset_table) = shadow_table.get::<mlua::Table>("offset") {
                        use niri_config::FloatOrInt;
                        if let Ok(x) = offset_table.get::<f64>("x") {
                            config.layout.shadow.offset.x = FloatOrInt(x);
                            debug!("  shadow.offset.x: {}", x);
                        }
                        if let Ok(y) = offset_table.get::<f64>("y") {
                            config.layout.shadow.offset.y = FloatOrInt(y);
                            debug!("  shadow.offset.y: {}", y);
                        }
                    }
                }

                info!("✓ Applied layout configuration from Lua");
            }
            Err(e) => {
                warn!("✗ Error extracting layout configuration from Lua: {}", e);
            }
        }
    } else {
        debug!("ℹ layout configuration not found in Lua globals");
    }

    // Extract cursor configuration
    if runtime.has_global("cursor") {
        debug!("Found cursor configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("cursor") {
            Ok(cursor_table) => {
                debug!("Processing cursor configuration");

                // Extract xcursor_theme
                if let Ok(theme) = cursor_table.get::<String>("xcursor_theme") {
                    debug!("  xcursor_theme: {}", theme);
                    config.cursor.xcursor_theme = theme;
                }

                // Extract xcursor_size
                if let Ok(size) = cursor_table.get::<u8>("xcursor_size") {
                    debug!("  xcursor_size: {}", size);
                    config.cursor.xcursor_size = size;
                }

                // Extract hide_when_typing
                if let Ok(hide) = cursor_table.get::<bool>("hide_when_typing") {
                    debug!("  hide_when_typing: {}", hide);
                    config.cursor.hide_when_typing = hide;
                }

                // Extract hide_after_inactive_ms
                if let Ok(ms) = cursor_table.get::<u32>("hide_after_inactive_ms") {
                    debug!("  hide_after_inactive_ms: {}", ms);
                    config.cursor.hide_after_inactive_ms = Some(ms);
                }

                info!("✓ Applied cursor configuration from Lua");
            }
            Err(e) => {
                warn!("✗ Error extracting cursor configuration from Lua: {}", e);
            }
        }
    } else {
        debug!("ℹ cursor configuration not found in Lua globals");
    }

    // Extract screenshot_path configuration
    if runtime.has_global("screenshot_path") {
        debug!("Found screenshot_path configuration in Lua globals");
        match runtime.inner().globals().get::<String>("screenshot_path") {
            Ok(path) => {
                debug!("  screenshot_path: {}", path);
                config.screenshot_path = niri_config::ScreenshotPath(Some(path));
                info!("✓ Applied screenshot_path configuration from Lua");
            }
            Err(e) => {
                debug!("ℹ screenshot_path exists but is not a string: {}", e);
            }
        }
    } else if runtime.has_global("screenshot") {
        // Also check for screenshot table with a path field
        debug!("Found screenshot configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("screenshot") {
            Ok(screenshot_table) => {
                if let Ok(path) = screenshot_table.get::<String>("path") {
                    debug!("  screenshot.path: {}", path);
                    config.screenshot_path = niri_config::ScreenshotPath(Some(path));
                    info!("✓ Applied screenshot.path configuration from Lua");
                }
            }
            Err(e) => {
                debug!("ℹ screenshot exists but is not a table: {}", e);
            }
        }
    } else {
        debug!("ℹ screenshot_path not found in Lua globals");
    }

    // Extract environment variables
    if runtime.has_global("environment") {
        debug!("Found environment configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("environment") {
            Ok(env_table) => {
                debug!("Processing environment configuration");
                config.environment.0.clear();

                for pair in env_table.pairs::<String, mlua::Value>() {
                    if let Ok((name, value)) = pair {
                        let value_str = match value {
                            mlua::Value::String(s) => match s.to_str() {
                                Ok(str_val) => Some(str_val.to_string()),
                                Err(_) => {
                                    warn!("Invalid UTF-8 in environment variable {}", name);
                                    continue;
                                }
                            },
                            mlua::Value::Nil => None,
                            _ => {
                                warn!("Invalid environment variable value for {}", name);
                                continue;
                            }
                        };
                        debug!("  {}: {:?}", name, value_str);
                        config.environment.0.push(niri_config::EnvironmentVariable {
                            name,
                            value: value_str,
                        });
                    }
                }

                info!(
                    "✓ Applied {} environment variable(s) from Lua",
                    config.environment.0.len()
                );
            }
            Err(e) => {
                warn!(
                    "✗ Error extracting environment configuration from Lua: {}",
                    e
                );
            }
        }
    } else {
        debug!("ℹ environment configuration not found in Lua globals");
    }

    // Extract debug configuration
    if runtime.has_global("debug") {
        debug!("Found debug configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("debug") {
            Ok(debug_table) => {
                debug!("Processing debug configuration");

                // Extract preview_render
                if let Ok(preview_str) = debug_table.get::<String>("preview_render") {
                    use niri_config::debug::PreviewRender;
                    config.debug.preview_render = match preview_str.to_lowercase().as_str() {
                        "screencast" => Some(PreviewRender::Screencast),
                        "screen-capture" | "screen_capture" => Some(PreviewRender::ScreenCapture),
                        _ => {
                            warn!("Invalid preview_render value: {}", preview_str);
                            None
                        }
                    };
                    debug!("  preview_render: {:?}", config.debug.preview_render);
                }

                // Extract boolean flags
                if let Ok(value) =
                    debug_table.get::<bool>("dbus_interfaces_in_non_session_instances")
                {
                    debug!("  dbus_interfaces_in_non_session_instances: {}", value);
                    config.debug.dbus_interfaces_in_non_session_instances = value;
                }
                if let Ok(value) =
                    debug_table.get::<bool>("wait_for_frame_completion_before_queueing")
                {
                    debug!("  wait_for_frame_completion_before_queueing: {}", value);
                    config.debug.wait_for_frame_completion_before_queueing = value;
                }
                if let Ok(value) = debug_table.get::<bool>("enable_overlay_planes") {
                    debug!("  enable_overlay_planes: {}", value);
                    config.debug.enable_overlay_planes = value;
                }
                if let Ok(value) = debug_table.get::<bool>("disable_cursor_plane") {
                    debug!("  disable_cursor_plane: {}", value);
                    config.debug.disable_cursor_plane = value;
                }
                if let Ok(value) = debug_table.get::<bool>("disable_direct_scanout") {
                    debug!("  disable_direct_scanout: {}", value);
                    config.debug.disable_direct_scanout = value;
                }
                if let Ok(value) = debug_table.get::<bool>("keep_max_bpc_unchanged") {
                    debug!("  keep_max_bpc_unchanged: {}", value);
                    config.debug.keep_max_bpc_unchanged = value;
                }
                if let Ok(value) =
                    debug_table.get::<bool>("restrict_primary_scanout_to_matching_format")
                {
                    debug!("  restrict_primary_scanout_to_matching_format: {}", value);
                    config.debug.restrict_primary_scanout_to_matching_format = value;
                }
                if let Ok(value) = debug_table.get::<bool>("force_pipewire_invalid_modifier") {
                    debug!("  force_pipewire_invalid_modifier: {}", value);
                    config.debug.force_pipewire_invalid_modifier = value;
                }
                if let Ok(value) = debug_table.get::<bool>("emulate_zero_presentation_time") {
                    debug!("  emulate_zero_presentation_time: {}", value);
                    config.debug.emulate_zero_presentation_time = value;
                }
                if let Ok(value) = debug_table.get::<bool>("disable_resize_throttling") {
                    debug!("  disable_resize_throttling: {}", value);
                    config.debug.disable_resize_throttling = value;
                }
                if let Ok(value) = debug_table.get::<bool>("disable_transactions") {
                    debug!("  disable_transactions: {}", value);
                    config.debug.disable_transactions = value;
                }
                if let Ok(value) =
                    debug_table.get::<bool>("keep_laptop_panel_on_when_lid_is_closed")
                {
                    debug!("  keep_laptop_panel_on_when_lid_is_closed: {}", value);
                    config.debug.keep_laptop_panel_on_when_lid_is_closed = value;
                }
                if let Ok(value) = debug_table.get::<bool>("disable_monitor_names") {
                    debug!("  disable_monitor_names: {}", value);
                    config.debug.disable_monitor_names = value;
                }
                if let Ok(value) = debug_table.get::<bool>("strict_new_window_focus_policy") {
                    debug!("  strict_new_window_focus_policy: {}", value);
                    config.debug.strict_new_window_focus_policy = value;
                }
                if let Ok(value) =
                    debug_table.get::<bool>("honor_xdg_activation_with_invalid_serial")
                {
                    debug!("  honor_xdg_activation_with_invalid_serial: {}", value);
                    config.debug.honor_xdg_activation_with_invalid_serial = value;
                }
                if let Ok(value) = debug_table.get::<bool>("deactivate_unfocused_windows") {
                    debug!("  deactivate_unfocused_windows: {}", value);
                    config.debug.deactivate_unfocused_windows = value;
                }
                if let Ok(value) = debug_table.get::<bool>("skip_cursor_only_updates_during_vrr") {
                    debug!("  skip_cursor_only_updates_during_vrr: {}", value);
                    config.debug.skip_cursor_only_updates_during_vrr = value;
                }

                // Extract render_drm_device
                if let Ok(device_path) = debug_table.get::<String>("render_drm_device") {
                    debug!("  render_drm_device: {}", device_path);
                    config.debug.render_drm_device = Some(std::path::PathBuf::from(device_path));
                }

                // Extract ignored_drm_devices
                if let Ok(devices_table) = debug_table.get::<mlua::Table>("ignored_drm_devices") {
                    debug!("Processing ignored_drm_devices");
                    config.debug.ignored_drm_devices.clear();

                    for pair in devices_table.pairs::<usize, String>() {
                        if let Ok((_, device_path)) = pair {
                            debug!("  ignored device: {}", device_path);
                            config
                                .debug
                                .ignored_drm_devices
                                .push(std::path::PathBuf::from(device_path));
                        }
                    }
                }

                info!("✓ Applied debug configuration from Lua");
            }
            Err(e) => {
                warn!("✗ Error extracting debug configuration from Lua: {}", e);
            }
        }
    } else {
        debug!("ℹ debug configuration not found in Lua globals");
    }

    // Extract workspaces configuration
    if runtime.has_global("workspaces") {
        debug!("Found workspaces configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("workspaces") {
            Ok(workspaces_table) => {
                debug!("Processing workspaces configuration");
                config.workspaces.clear();

                for pair in workspaces_table.pairs::<usize, mlua::Value>() {
                    if let Ok((_, workspace_val)) = pair {
                        if let mlua::Value::Table(ws_table) = workspace_val {
                            // Extract workspace name (required)
                            if let Ok(name) = ws_table.get::<String>("name") {
                                debug!("  Processing workspace: {}", name);

                                let mut workspace = niri_config::workspace::Workspace {
                                    name: niri_config::workspace::WorkspaceName(name.clone()),
                                    open_on_output: None,
                                    layout: None,
                                };

                                // Extract optional open_on_output
                                if let Ok(output) = ws_table.get::<String>("open_on_output") {
                                    debug!("    open_on_output: {}", output);
                                    workspace.open_on_output = Some(output);
                                }

                                // Note: workspace layout is a subset of the main layout config
                                // For now, we skip layout parsing as it requires complex LayoutPart
                                // handling This can be added later
                                // if needed

                                config.workspaces.push(workspace);
                            } else {
                                warn!("Workspace missing required 'name' field");
                            }
                        } else if let mlua::Value::String(name_str) = workspace_val {
                            // Support simple string format for workspace names
                            if let Ok(name) = name_str.to_str() {
                                debug!("  Processing workspace (simple): {}", name);
                                config.workspaces.push(niri_config::workspace::Workspace {
                                    name: niri_config::workspace::WorkspaceName(name.to_string()),
                                    open_on_output: None,
                                    layout: None,
                                });
                            }
                        }
                    }
                }

                info!(
                    "✓ Applied {} workspace(s) from Lua",
                    config.workspaces.len()
                );
            }
            Err(e) => {
                debug!(
                    "ℹ workspaces configuration exists but is not a table: {}",
                    e
                );
            }
        }
    } else {
        debug!("ℹ workspaces configuration not found in Lua globals");
    }

    // Extract animations configuration
    if runtime.has_global("animations") {
        debug!("Found animations configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("animations") {
            Ok(animations_table) => {
                debug!("Processing animations configuration");

                // Extract global animation settings
                if let Ok(off) = animations_table.get::<bool>("off") {
                    debug!("  animations.off: {}", off);
                    config.animations.off = off;
                }
                if let Ok(_on) = animations_table.get::<bool>("on") {
                    debug!("  animations.on: true (setting off=false)");
                    config.animations.off = false;
                }
                if let Ok(slowdown) = animations_table.get::<f64>("slowdown") {
                    debug!("  animations.slowdown: {}", slowdown);
                    config.animations.slowdown = slowdown;
                }

                // Extract workspace_switch animation
                if let Ok(ws_switch_table) = animations_table.get::<mlua::Table>("workspace_switch")
                {
                    debug!("  Processing workspace_switch animation");
                    let anim = parse_animation(&ws_switch_table);
                    config.animations.workspace_switch =
                        niri_config::animations::WorkspaceSwitchAnim(anim);
                }

                // Extract horizontal_view_movement animation
                if let Ok(hvm_table) =
                    animations_table.get::<mlua::Table>("horizontal_view_movement")
                {
                    debug!("  Processing horizontal_view_movement animation");
                    let anim = parse_animation(&hvm_table);
                    config.animations.horizontal_view_movement =
                        niri_config::animations::HorizontalViewMovementAnim(anim);
                }

                // Extract window_movement animation
                if let Ok(wm_table) = animations_table.get::<mlua::Table>("window_movement") {
                    debug!("  Processing window_movement animation");
                    let anim = parse_animation(&wm_table);
                    config.animations.window_movement =
                        niri_config::animations::WindowMovementAnim(anim);
                }

                // Extract window_open animation (with optional custom_shader)
                if let Ok(wo_table) = animations_table.get::<mlua::Table>("window_open") {
                    debug!("  Processing window_open animation");
                    let anim = parse_animation(&wo_table);
                    let custom_shader = wo_table.get::<String>("custom_shader").ok();
                    config.animations.window_open = niri_config::animations::WindowOpenAnim {
                        anim,
                        custom_shader,
                    };
                }

                // Extract window_close animation (with optional custom_shader)
                if let Ok(wc_table) = animations_table.get::<mlua::Table>("window_close") {
                    debug!("  Processing window_close animation");
                    let anim = parse_animation(&wc_table);
                    let custom_shader = wc_table.get::<String>("custom_shader").ok();
                    config.animations.window_close = niri_config::animations::WindowCloseAnim {
                        anim,
                        custom_shader,
                    };
                }

                // Extract window_resize animation (with optional custom_shader)
                if let Ok(wr_table) = animations_table.get::<mlua::Table>("window_resize") {
                    debug!("  Processing window_resize animation");
                    let anim = parse_animation(&wr_table);
                    let custom_shader = wr_table.get::<String>("custom_shader").ok();
                    config.animations.window_resize = niri_config::animations::WindowResizeAnim {
                        anim,
                        custom_shader,
                    };
                }

                // Extract config_notification_open_close animation
                if let Ok(cn_table) =
                    animations_table.get::<mlua::Table>("config_notification_open_close")
                {
                    debug!("  Processing config_notification_open_close animation");
                    let anim = parse_animation(&cn_table);
                    config.animations.config_notification_open_close =
                        niri_config::animations::ConfigNotificationOpenCloseAnim(anim);
                }

                // Extract exit_confirmation_open_close animation
                if let Ok(ec_table) =
                    animations_table.get::<mlua::Table>("exit_confirmation_open_close")
                {
                    debug!("  Processing exit_confirmation_open_close animation");
                    let anim = parse_animation(&ec_table);
                    config.animations.exit_confirmation_open_close =
                        niri_config::animations::ExitConfirmationOpenCloseAnim(anim);
                }

                // Extract screenshot_ui_open animation
                if let Ok(su_table) = animations_table.get::<mlua::Table>("screenshot_ui_open") {
                    debug!("  Processing screenshot_ui_open animation");
                    let anim = parse_animation(&su_table);
                    config.animations.screenshot_ui_open =
                        niri_config::animations::ScreenshotUiOpenAnim(anim);
                }

                // Extract overview_open_close animation
                if let Ok(oo_table) = animations_table.get::<mlua::Table>("overview_open_close") {
                    debug!("  Processing overview_open_close animation");
                    let anim = parse_animation(&oo_table);
                    config.animations.overview_open_close =
                        niri_config::animations::OverviewOpenCloseAnim(anim);
                }

                info!("✓ Applied animations configuration from Lua");
            }
            Err(e) => {
                warn!(
                    "✗ Error extracting animations configuration from Lua: {}",
                    e
                );
            }
        }
    } else {
        debug!("ℹ animations configuration not found in Lua globals");
    }

    // Extract gestures configuration
    if runtime.has_global("gestures") {
        debug!("Found gestures configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("gestures") {
            Ok(gestures_table) => {
                debug!("Processing gestures configuration");

                // Extract dnd_edge_view_scroll settings
                if let Ok(devs_table) = gestures_table.get::<mlua::Table>("dnd_edge_view_scroll") {
                    debug!("  Processing dnd_edge_view_scroll");

                    if let Ok(trigger_width) = devs_table.get::<f64>("trigger_width") {
                        debug!("    trigger_width: {}", trigger_width);
                        config.gestures.dnd_edge_view_scroll.trigger_width = trigger_width;
                    }

                    if let Ok(delay_ms) = devs_table.get::<u16>("delay_ms") {
                        debug!("    delay_ms: {}", delay_ms);
                        config.gestures.dnd_edge_view_scroll.delay_ms = delay_ms;
                    }

                    if let Ok(max_speed) = devs_table.get::<f64>("max_speed") {
                        debug!("    max_speed: {}", max_speed);
                        config.gestures.dnd_edge_view_scroll.max_speed = max_speed;
                    }
                }

                // Extract dnd_edge_workspace_switch settings
                if let Ok(dews_table) =
                    gestures_table.get::<mlua::Table>("dnd_edge_workspace_switch")
                {
                    debug!("  Processing dnd_edge_workspace_switch");

                    if let Ok(trigger_height) = dews_table.get::<f64>("trigger_height") {
                        debug!("    trigger_height: {}", trigger_height);
                        config.gestures.dnd_edge_workspace_switch.trigger_height = trigger_height;
                    }

                    if let Ok(delay_ms) = dews_table.get::<u16>("delay_ms") {
                        debug!("    delay_ms: {}", delay_ms);
                        config.gestures.dnd_edge_workspace_switch.delay_ms = delay_ms;
                    }

                    if let Ok(max_speed) = dews_table.get::<f64>("max_speed") {
                        debug!("    max_speed: {}", max_speed);
                        config.gestures.dnd_edge_workspace_switch.max_speed = max_speed;
                    }
                }

                // Extract hot_corners settings
                if let Ok(hot_corners_table) = gestures_table.get::<mlua::Table>("hot_corners") {
                    debug!("  Processing hot_corners");

                    if let Ok(off) = hot_corners_table.get::<bool>("off") {
                        debug!("    off: {}", off);
                        config.gestures.hot_corners.off = off;
                    }

                    if let Ok(top_left) = hot_corners_table.get::<bool>("top_left") {
                        debug!("    top_left: {}", top_left);
                        config.gestures.hot_corners.top_left = top_left;
                    }

                    if let Ok(top_right) = hot_corners_table.get::<bool>("top_right") {
                        debug!("    top_right: {}", top_right);
                        config.gestures.hot_corners.top_right = top_right;
                    }

                    if let Ok(bottom_left) = hot_corners_table.get::<bool>("bottom_left") {
                        debug!("    bottom_left: {}", bottom_left);
                        config.gestures.hot_corners.bottom_left = bottom_left;
                    }

                    if let Ok(bottom_right) = hot_corners_table.get::<bool>("bottom_right") {
                        debug!("    bottom_right: {}", bottom_right);
                        config.gestures.hot_corners.bottom_right = bottom_right;
                    }
                }

                info!("✓ Applied gestures configuration from Lua");
            }
            Err(e) => {
                warn!("✗ Error extracting gestures configuration from Lua: {}", e);
            }
        }
    } else {
        debug!("ℹ gestures configuration not found in Lua globals");
    }

    // Extract clipboard configuration
    if runtime.has_global("clipboard") {
        debug!("Found clipboard configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("clipboard") {
            Ok(clipboard_table) => {
                debug!("Processing clipboard configuration");

                if let Ok(disable_primary) = clipboard_table.get::<bool>("disable_primary") {
                    debug!("  clipboard.disable_primary: {}", disable_primary);
                    config.clipboard.disable_primary = disable_primary;
                }

                info!("✓ Applied clipboard configuration from Lua");
            }
            Err(e) => {
                warn!("✗ Error extracting clipboard configuration from Lua: {}", e);
            }
        }
    } else {
        debug!("ℹ clipboard configuration not found in Lua globals");
    }

    // Extract hotkey_overlay configuration
    if runtime.has_global("hotkey_overlay") {
        debug!("Found hotkey_overlay configuration in Lua globals");
        match runtime
            .inner()
            .globals()
            .get::<mlua::Table>("hotkey_overlay")
        {
            Ok(hotkey_table) => {
                debug!("Processing hotkey_overlay configuration");

                if let Ok(skip_at_startup) = hotkey_table.get::<bool>("skip_at_startup") {
                    debug!("  hotkey_overlay.skip_at_startup: {}", skip_at_startup);
                    config.hotkey_overlay.skip_at_startup = skip_at_startup;
                }

                if let Ok(hide_not_bound) = hotkey_table.get::<bool>("hide_not_bound") {
                    debug!("  hotkey_overlay.hide_not_bound: {}", hide_not_bound);
                    config.hotkey_overlay.hide_not_bound = hide_not_bound;
                }

                info!("✓ Applied hotkey_overlay configuration from Lua");
            }
            Err(e) => {
                warn!(
                    "✗ Error extracting hotkey_overlay configuration from Lua: {}",
                    e
                );
            }
        }
    } else {
        debug!("ℹ hotkey_overlay configuration not found in Lua globals");
    }

    // Extract config_notification configuration
    if runtime.has_global("config_notification") {
        debug!("Found config_notification configuration in Lua globals");
        match runtime
            .inner()
            .globals()
            .get::<mlua::Table>("config_notification")
        {
            Ok(config_notif_table) => {
                debug!("Processing config_notification configuration");

                if let Ok(disable_failed) = config_notif_table.get::<bool>("disable_failed") {
                    debug!("  config_notification.disable_failed: {}", disable_failed);
                    config.config_notification.disable_failed = disable_failed;
                }

                info!("✓ Applied config_notification configuration from Lua");
            }
            Err(e) => {
                warn!(
                    "✗ Error extracting config_notification configuration from Lua: {}",
                    e
                );
            }
        }
    } else {
        debug!("ℹ config_notification configuration not found in Lua globals");
    }

    // Extract recent_windows configuration
    if runtime.has_global("recent_windows") {
        debug!("Found recent_windows configuration in Lua globals");
        match runtime
            .inner()
            .globals()
            .get::<mlua::Table>("recent_windows")
        {
            Ok(recent_windows_table) => {
                debug!("Processing recent_windows configuration");

                if let Ok(on) = recent_windows_table.get::<bool>("on") {
                    debug!("  recent_windows.on: {}", on);
                    config.recent_windows.on = on;
                }

                if let Ok(open_delay_ms) = recent_windows_table.get::<u16>("open_delay_ms") {
                    debug!("  recent_windows.open_delay_ms: {}", open_delay_ms);
                    config.recent_windows.open_delay_ms = open_delay_ms;
                }

                // Process highlight subtable
                if let Ok(highlight_table) = recent_windows_table.get::<mlua::Table>("highlight") {
                    debug!("  Processing recent_windows.highlight");

                    if let Ok(active_color) = highlight_table.get::<String>("active_color") {
                        if let Some(color) = parse_hex_color(&active_color) {
                            config.recent_windows.highlight.active_color = color;
                        }
                    }

                    if let Ok(urgent_color) = highlight_table.get::<String>("urgent_color") {
                        if let Some(color) = parse_hex_color(&urgent_color) {
                            config.recent_windows.highlight.urgent_color = color;
                        }
                    }

                    if let Ok(padding) = highlight_table.get::<f64>("padding") {
                        config.recent_windows.highlight.padding = padding;
                    }

                    if let Ok(corner_radius) = highlight_table.get::<f64>("corner_radius") {
                        config.recent_windows.highlight.corner_radius = corner_radius;
                    }
                }

                // Process previews subtable
                if let Ok(previews_table) = recent_windows_table.get::<mlua::Table>("previews") {
                    debug!("  Processing recent_windows.previews");

                    if let Ok(max_height) = previews_table.get::<f64>("max_height") {
                        config.recent_windows.previews.max_height = max_height;
                    }

                    if let Ok(max_scale) = previews_table.get::<f64>("max_scale") {
                        config.recent_windows.previews.max_scale = max_scale;
                    }
                }

                info!("✓ Applied recent_windows configuration from Lua");
            }
            Err(e) => {
                warn!(
                    "✗ Error extracting recent_windows configuration from Lua: {}",
                    e
                );
            }
        }
    } else {
        debug!("ℹ recent_windows configuration not found in Lua globals");
    }

    // Extract overview configuration
    if runtime.has_global("overview") {
        debug!("Found overview configuration in Lua globals");
        match runtime.inner().globals().get::<mlua::Table>("overview") {
            Ok(overview_table) => {
                debug!("Processing overview configuration");

                if let Ok(zoom) = overview_table.get::<f64>("zoom") {
                    debug!("  overview.zoom: {}", zoom);
                    config.overview.zoom = zoom;
                }

                if let Ok(backdrop_color) = overview_table.get::<String>("backdrop_color") {
                    if let Some(color) = parse_hex_color(&backdrop_color) {
                        config.overview.backdrop_color = color;
                    }
                }

                // Process workspace_shadow subtable
                if let Ok(shadow_table) = overview_table.get::<mlua::Table>("workspace_shadow") {
                    debug!("  Processing overview.workspace_shadow");

                    if let Ok(off) = shadow_table.get::<bool>("off") {
                        config.overview.workspace_shadow.off = off;
                    }

                    if let Ok(softness) = shadow_table.get::<f64>("softness") {
                        config.overview.workspace_shadow.softness = softness;
                    }

                    if let Ok(spread) = shadow_table.get::<f64>("spread") {
                        config.overview.workspace_shadow.spread = spread;
                    }

                    if let Ok(color) = shadow_table.get::<String>("color") {
                        if let Some(parsed_color) = parse_hex_color(&color) {
                            config.overview.workspace_shadow.color = parsed_color;
                        }
                    }

                    // Process offset
                    if let Ok(offset_table) = shadow_table.get::<mlua::Table>("offset") {
                        if let Ok(x) = offset_table.get::<f64>("x") {
                            config.overview.workspace_shadow.offset.x = niri_config::FloatOrInt(x);
                        }
                        if let Ok(y) = offset_table.get::<f64>("y") {
                            config.overview.workspace_shadow.offset.y = niri_config::FloatOrInt(y);
                        }
                    }
                }

                info!("✓ Applied overview configuration from Lua");
            }
            Err(e) => {
                warn!("✗ Error extracting overview configuration from Lua: {}", e);
            }
        }
    } else {
        debug!("ℹ overview configuration not found in Lua globals");
    }

    // Register the config API so Lua scripts can read the current configuration
    debug!("Registering configuration API to Lua");
    runtime
        .register_config_api(config)
        .map_err(|e| anyhow::anyhow!("Failed to register config API: {}", e))?;

    info!("✓ Configuration API registered successfully");

    debug!("=== Lua configuration application completed ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_lua_config_empty() {
        let runtime = LuaRuntime::new().unwrap();
        let mut config = Config::default();
        let result = apply_lua_config(&runtime, &mut config);
        assert!(result.is_ok());
    }

    #[test]
    fn apply_lua_config_with_values() {
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
    fn apply_lua_config_with_startup_commands_from_return() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        local startup_commands = { "waybar", "swaync" }
        return { startup_commands = startup_commands }
        "#;

        // This simulates what LuaConfig::from_string does
        let return_val = runtime.load_string(code).expect("Failed to load Lua code");

        // Extract startup_commands from returned table and set as globals
        if let mlua::Value::Table(config_table) = return_val {
            let globals = runtime.inner().globals();
            if let Ok(value) = config_table.get::<mlua::Value>("startup_commands") {
                if value != mlua::Value::Nil {
                    globals
                        .set("startup", value)
                        .expect("Failed to set startup global");
                }
            }
        }

        let mut config = Config::default();
        assert_eq!(config.spawn_at_startup.len(), 0);

        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");

        assert_eq!(config.spawn_at_startup.len(), 2);
        assert_eq!(config.spawn_at_startup[0].command, vec!["waybar"]);
        assert_eq!(config.spawn_at_startup[1].command, vec!["swaync"]);
    }

    #[test]
    fn lua_config_from_string_with_startup_commands() {
        use crate::LuaConfig;

        let code = r#"
        local startup_commands = { "waybar", "swaync", "nm-applet" }
        local binds = {
            { key = "Super+Return", action = "spawn", args = { "alacritty" } },
        }
        return {
            startup_commands = startup_commands,
            binds = binds,
        }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();
        let initial_startup_count = config.spawn_at_startup.len();
        let initial_bind_count = config.binds.0.len();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify startup commands were applied
        assert_eq!(config.spawn_at_startup.len(), initial_startup_count + 3);
        assert_eq!(
            config.spawn_at_startup[initial_startup_count].command,
            vec!["waybar"]
        );
        assert_eq!(
            config.spawn_at_startup[initial_startup_count + 1].command,
            vec!["swaync"]
        );
        assert_eq!(
            config.spawn_at_startup[initial_startup_count + 2].command,
            vec!["nm-applet"]
        );

        // Verify keybindings were also applied
        assert!(config.binds.0.len() >= initial_bind_count + 1);
    }

    #[test]
    fn extract_keybindings() {
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
        assert!(
            config.binds.0.len() >= initial_bind_count + 2,
            "Expected at least {} bindings, got {}",
            initial_bind_count + 2,
            config.binds.0.len()
        );
    }

    #[test]
    fn exit_action() {
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
    fn overview_toggle_action() {
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
    fn hotkey_overlay_action() {
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
        matches!(
            last_bind.action,
            niri_config::binds::Action::ShowHotkeyOverlay
        );
    }

    #[test]
    fn suspend_action() {
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
    fn multiple_new_actions() {
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

    #[test]
    fn switch_preset_column_width_action() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Mod+R", action = "switch-preset-column-width", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();

        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");

        assert_eq!(config.binds.0.len(), initial_bind_count + 1);
        let last_bind = &config.binds.0[initial_bind_count];
        matches!(
            last_bind.action,
            niri_config::binds::Action::SwitchPresetColumnWidth
        );
    }

    #[test]
    fn consume_or_expel_window_actions() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Mod+BracketLeft", action = "consume-or-expel-window-left", args = {} },
            { key = "Mod+BracketRight", action = "consume-or-expel-window-right", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();

        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");

        assert_eq!(config.binds.0.len(), initial_bind_count + 2);
    }

    #[test]
    fn switch_focus_between_floating_and_tiling() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Mod+Shift+V", action = "switch-focus-between-floating-and-tiling", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();

        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");

        assert_eq!(config.binds.0.len(), initial_bind_count + 1);
        let last_bind = &config.binds.0[initial_bind_count];
        matches!(
            last_bind.action,
            niri_config::binds::Action::SwitchFocusBetweenFloatingAndTiling
        );
    }

    #[test]
    fn all_window_management_actions() {
        let runtime = LuaRuntime::new().unwrap();
        let code = r#"
        binds = {
            { key = "Mod+R", action = "switch-preset-column-width", args = {} },
            { key = "Mod+BracketLeft", action = "consume-or-expel-window-left", args = {} },
            { key = "Mod+BracketRight", action = "consume-or-expel-window-right", args = {} },
            { key = "Mod+Shift+V", action = "switch-focus-between-floating-and-tiling", args = {} },
        }
        "#;
        runtime.load_string(code).expect("Failed to load Lua code");

        let mut config = Config::default();
        let initial_bind_count = config.binds.0.len();

        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");

        // All 4 actions should be added
        assert_eq!(config.binds.0.len(), initial_bind_count + 4);
    }

    #[test]
    fn apply_input_keyboard_xkb_config() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            keyboard = {
                xkb = {
                    layout = "us",
                    variant = "dvorak",
                    options = "ctrl:nocaps,compose:ralt",
                    model = "pc105",
                    rules = "evdev",
                },
                repeat_delay = 300,
                repeat_rate = 50,
                numlock = true,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        // Store original values for comparison
        let original_repeat_delay = config.input.keyboard.repeat_delay;
        let original_repeat_rate = config.input.keyboard.repeat_rate;
        let original_numlock = config.input.keyboard.numlock;

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify that xkb settings were applied
        assert_eq!(config.input.keyboard.xkb.layout, "us");
        assert_eq!(config.input.keyboard.xkb.variant, "dvorak");
        assert_eq!(
            config.input.keyboard.xkb.options,
            Some("ctrl:nocaps,compose:ralt".to_string())
        );
        assert_eq!(config.input.keyboard.xkb.model, "pc105");
        assert_eq!(config.input.keyboard.xkb.rules, "evdev");

        // Verify that keyboard settings were applied
        assert_eq!(config.input.keyboard.repeat_delay, 300);
        assert_ne!(config.input.keyboard.repeat_delay, original_repeat_delay);
        assert_eq!(config.input.keyboard.repeat_rate, 50);
        assert_ne!(config.input.keyboard.repeat_rate, original_repeat_rate);
        assert_eq!(config.input.keyboard.numlock, true);
        assert_ne!(config.input.keyboard.numlock, original_numlock);
    }

    #[test]
    fn apply_partial_input_config() {
        use crate::LuaConfig;

        // Test that we can apply only some xkb settings
        let code = r#"
        local input = {
            keyboard = {
                xkb = {
                    layout = "de",
                },
                repeat_delay = 200,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();
        let original_variant = config.input.keyboard.xkb.variant.clone();
        let original_repeat_rate = config.input.keyboard.repeat_rate;

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify that layout was changed
        assert_eq!(config.input.keyboard.xkb.layout, "de");

        // Verify that variant was not changed (not specified in Lua)
        assert_eq!(config.input.keyboard.xkb.variant, original_variant);

        // Verify that repeat_delay was changed
        assert_eq!(config.input.keyboard.repeat_delay, 200);

        // Verify that repeat_rate was not changed (not specified in Lua)
        assert_eq!(config.input.keyboard.repeat_rate, original_repeat_rate);
    }

    #[test]
    fn apply_example_config_input() {
        use crate::LuaConfig;

        // Test with the actual example config's input settings
        let code = r#"
        local input = {
            keyboard = {
                xkb = {
                    layout = "us",
                    variant = "intl,phonetic",
                },
                numlock = false,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify that the variant with comma is properly applied
        assert_eq!(config.input.keyboard.xkb.layout, "us");
        assert_eq!(config.input.keyboard.xkb.variant, "intl,phonetic");
        assert_eq!(config.input.keyboard.numlock, false);
    }

    #[test]
    fn apply_touchpad_config() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            touchpad = {
                tap = true,
                natural_scroll = true,
                accel_speed = 0.5,
                accel_profile = "flat",
                scroll_method = "two-finger",
                dwt = true,
                dwtp = false,
                drag = true,
                drag_lock = false,
                left_handed = false,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify touchpad configuration was applied
        assert_eq!(config.input.touchpad.tap, true);
        assert_eq!(config.input.touchpad.natural_scroll, true);
        assert_eq!(config.input.touchpad.accel_speed.0, 0.5);
        assert_eq!(
            config.input.touchpad.accel_profile,
            Some(input::AccelProfile::Flat)
        );
        assert_eq!(
            config.input.touchpad.scroll_method,
            Some(input::ScrollMethod::TwoFinger)
        );
        assert_eq!(config.input.touchpad.dwt, true);
        assert_eq!(config.input.touchpad.dwtp, false);
        assert_eq!(config.input.touchpad.drag, Some(true));
        assert_eq!(config.input.touchpad.drag_lock, false);
        assert_eq!(config.input.touchpad.left_handed, false);
    }

    #[test]
    fn apply_mouse_config() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            mouse = {
                natural_scroll = false,
                accel_speed = 0.3,
                accel_profile = "adaptive",
                left_handed = true,
                middle_emulation = true,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify mouse configuration was applied
        assert_eq!(config.input.mouse.natural_scroll, false);
        assert_eq!(config.input.mouse.accel_speed.0, 0.3);
        assert_eq!(
            config.input.mouse.accel_profile,
            Some(input::AccelProfile::Adaptive)
        );
        assert_eq!(config.input.mouse.left_handed, true);
        assert_eq!(config.input.mouse.middle_emulation, true);
    }

    #[test]
    fn apply_combined_input_config() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            keyboard = {
                xkb = {
                    layout = "us,ru",
                    variant = "dvorak,",
                },
                numlock = true,
            },
            touchpad = {
                tap = true,
                natural_scroll = true,
                accel_speed = 0.2,
            },
            mouse = {
                natural_scroll = false,
                accel_speed = -0.1,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify all input configurations were applied
        assert_eq!(config.input.keyboard.xkb.layout, "us,ru");
        assert_eq!(config.input.keyboard.xkb.variant, "dvorak,");
        assert_eq!(config.input.keyboard.numlock, true);

        assert_eq!(config.input.touchpad.tap, true);
        assert_eq!(config.input.touchpad.natural_scroll, true);
        assert_eq!(config.input.touchpad.accel_speed.0, 0.2);

        assert_eq!(config.input.mouse.natural_scroll, false);
        assert_eq!(config.input.mouse.accel_speed.0, -0.1);
    }

    #[test]
    fn apply_input_enum_values() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            touchpad = {
                accel_profile = "adaptive",
                scroll_method = "edge",
                click_method = "button-areas",
                tap_button_map = "left-middle-right",
            },
            mouse = {
                accel_profile = "flat",
                scroll_method = "on-button-down",
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify all enum values were correctly parsed
        assert_eq!(
            config.input.touchpad.accel_profile,
            Some(input::AccelProfile::Adaptive)
        );
        assert_eq!(
            config.input.touchpad.scroll_method,
            Some(input::ScrollMethod::Edge)
        );
        assert_eq!(
            config.input.touchpad.click_method,
            Some(input::ClickMethod::ButtonAreas)
        );
        assert_eq!(
            config.input.touchpad.tap_button_map,
            Some(input::TapButtonMap::LeftMiddleRight)
        );

        assert_eq!(
            config.input.mouse.accel_profile,
            Some(input::AccelProfile::Flat)
        );
        assert_eq!(
            config.input.mouse.scroll_method,
            Some(input::ScrollMethod::OnButtonDown)
        );
    }

    #[test]
    fn apply_trackpoint_trackball_config() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            trackpoint = {
                off = false,
                natural_scroll = true,
                left_handed = false,
                middle_emulation = true,
                scroll_button_lock = false,
                accel_speed = 0.3,
                accel_profile = "flat",
                scroll_method = "on-button-down",
                scroll_button = 9,
            },
            trackball = {
                off = false,
                natural_scroll = false,
                left_handed = true,
                middle_emulation = false,
                scroll_button_lock = true,
                accel_speed = -0.2,
                accel_profile = "adaptive",
                scroll_method = "no-scroll",
                scroll_button = 8,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify trackpoint settings
        assert_eq!(config.input.trackpoint.off, false);
        assert_eq!(config.input.trackpoint.natural_scroll, true);
        assert_eq!(config.input.trackpoint.left_handed, false);
        assert_eq!(config.input.trackpoint.middle_emulation, true);
        assert_eq!(config.input.trackpoint.scroll_button_lock, false);
        assert_eq!(config.input.trackpoint.accel_speed.0, 0.3);
        assert_eq!(
            config.input.trackpoint.accel_profile,
            Some(input::AccelProfile::Flat)
        );
        assert_eq!(
            config.input.trackpoint.scroll_method,
            Some(input::ScrollMethod::OnButtonDown)
        );
        assert_eq!(config.input.trackpoint.scroll_button, Some(9));

        // Verify trackball settings
        assert_eq!(config.input.trackball.off, false);
        assert_eq!(config.input.trackball.natural_scroll, false);
        assert_eq!(config.input.trackball.left_handed, true);
        assert_eq!(config.input.trackball.middle_emulation, false);
        assert_eq!(config.input.trackball.scroll_button_lock, true);
        assert_eq!(config.input.trackball.accel_speed.0, -0.2);
        assert_eq!(
            config.input.trackball.accel_profile,
            Some(input::AccelProfile::Adaptive)
        );
        assert_eq!(
            config.input.trackball.scroll_method,
            Some(input::ScrollMethod::NoScroll)
        );
        assert_eq!(config.input.trackball.scroll_button, Some(8));
    }

    #[test]
    fn apply_tablet_touch_config() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            tablet = {
                off = false,
                left_handed = true,
                map_to_output = "HDMI-1",
                calibration_matrix = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0},
            },
            touch = {
                off = false,
                map_to_output = "eDP-1",
                calibration_matrix = {0.9, 0.0, 0.1, 0.0, 0.9, 0.1},
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify tablet settings
        assert_eq!(config.input.tablet.off, false);
        assert_eq!(config.input.tablet.left_handed, true);
        assert_eq!(
            config.input.tablet.map_to_output,
            Some("HDMI-1".to_string())
        );
        assert_eq!(
            config.input.tablet.calibration_matrix,
            Some(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0])
        );

        // Verify touch settings
        assert_eq!(config.input.touch.off, false);
        assert_eq!(config.input.touch.map_to_output, Some("eDP-1".to_string()));
        assert_eq!(
            config.input.touch.calibration_matrix,
            Some(vec![0.9, 0.0, 0.1, 0.0, 0.9, 0.1])
        );
    }

    #[test]
    fn apply_global_input_settings() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            disable_power_key_handling = true,
            workspace_auto_back_and_forth = true,
            mod_key = "Super",
            mod_key_nested = "Alt",
            warp_mouse_to_focus = {
                mode = "center-xy-always",
            },
            focus_follows_mouse = {
                max_scroll_amount = 0.5,
            },
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify global settings
        assert_eq!(config.input.disable_power_key_handling, true);
        assert_eq!(config.input.workspace_auto_back_and_forth, true);
        assert_eq!(config.input.mod_key, Some(input::ModKey::Super));
        assert_eq!(config.input.mod_key_nested, Some(input::ModKey::Alt));

        // Verify warp_mouse_to_focus
        assert!(config.input.warp_mouse_to_focus.is_some());
        let warp = config.input.warp_mouse_to_focus.unwrap();
        assert_eq!(warp.mode, Some(input::WarpMouseToFocusMode::CenterXyAlways));

        // Verify focus_follows_mouse
        assert!(config.input.focus_follows_mouse.is_some());
        let focus = config.input.focus_follows_mouse.unwrap();
        assert_eq!(focus.max_scroll_amount.unwrap().0, 0.5);
    }

    #[test]
    fn apply_bool_warp_and_focus_follows() {
        use crate::LuaConfig;

        let code = r#"
        local input = {
            warp_mouse_to_focus = true,
            focus_follows_mouse = true,
        }
        return { input = input }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify that boolean values enable the features with default settings
        assert!(config.input.warp_mouse_to_focus.is_some());
        let warp = config.input.warp_mouse_to_focus.unwrap();
        assert_eq!(warp.mode, None); // Default mode

        assert!(config.input.focus_follows_mouse.is_some());
        let focus = config.input.focus_follows_mouse.unwrap();
        assert_eq!(focus.max_scroll_amount, None); // No max scroll limit
    }

    #[test]
    fn apply_output_basic_config() {
        use crate::LuaConfig;

        let code = r#"
        local outputs = {
            {
                name = "eDP-1",
                scale = 1.5,
                off = false,
            },
            {
                name = "HDMI-A-1",
                scale = 2.0,
            },
        }
        return { outputs = outputs }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify we have 2 outputs
        assert_eq!(config.outputs.0.len(), 2);

        // Verify first output
        assert_eq!(config.outputs.0[0].name, "eDP-1");
        assert_eq!(config.outputs.0[0].scale, Some(FloatOrInt(1.5)));
        assert_eq!(config.outputs.0[0].off, false);

        // Verify second output
        assert_eq!(config.outputs.0[1].name, "HDMI-A-1");
        assert_eq!(config.outputs.0[1].scale, Some(FloatOrInt(2.0)));
    }

    #[test]
    fn apply_output_transform() {
        use crate::LuaConfig;

        let code = r#"
        local outputs = {
            {
                name = "DP-1",
                transform = "90",
            },
            {
                name = "DP-2",
                transform = "flipped-180",
            },
            {
                name = "DP-3",
                transform = "normal",
            },
        }
        return { outputs = outputs }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify transforms
        assert_eq!(config.outputs.0[0].transform, niri_ipc::Transform::_90);
        assert_eq!(
            config.outputs.0[1].transform,
            niri_ipc::Transform::Flipped180
        );
        assert_eq!(config.outputs.0[2].transform, niri_ipc::Transform::Normal);
    }

    #[test]
    fn apply_output_position_and_mode() {
        use crate::LuaConfig;

        let code = r#"
        local outputs = {
            {
                name = "eDP-1",
                position = { x = 0, y = 0 },
                mode = "1920x1080@60",
            },
            {
                name = "HDMI-A-1",
                position = { x = 1920, y = 0 },
                mode = "3840x2160",
            },
        }
        return { outputs = outputs }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify first output
        assert_eq!(
            config.outputs.0[0].position,
            Some(niri_config::output::Position { x: 0, y: 0 })
        );
        let mode1 = config.outputs.0[0].mode.as_ref().unwrap();
        assert_eq!(mode1.mode.width, 1920);
        assert_eq!(mode1.mode.height, 1080);
        assert_eq!(mode1.mode.refresh, Some(60.0));

        // Verify second output
        assert_eq!(
            config.outputs.0[1].position,
            Some(niri_config::output::Position { x: 1920, y: 0 })
        );
        let mode2 = config.outputs.0[1].mode.as_ref().unwrap();
        assert_eq!(mode2.mode.width, 3840);
        assert_eq!(mode2.mode.height, 2160);
        assert_eq!(mode2.mode.refresh, None);
    }

    #[test]
    fn apply_output_vrr_and_focus() {
        use crate::LuaConfig;

        let code = r#"
        local outputs = {
            {
                name = "DP-1",
                variable_refresh_rate = { on_demand = true },
                focus_at_startup = true,
            },
            {
                name = "DP-2",
                variable_refresh_rate = true,
                focus_at_startup = false,
            },
        }
        return { outputs = outputs }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify first output - VRR with on_demand
        assert!(config.outputs.0[0].variable_refresh_rate.is_some());
        assert_eq!(
            config.outputs.0[0]
                .variable_refresh_rate
                .as_ref()
                .unwrap()
                .on_demand,
            true
        );
        assert_eq!(config.outputs.0[0].focus_at_startup, true);

        // Verify second output - VRR boolean shorthand
        assert!(config.outputs.0[1].variable_refresh_rate.is_some());
        assert_eq!(
            config.outputs.0[1]
                .variable_refresh_rate
                .as_ref()
                .unwrap()
                .on_demand,
            false
        );
        assert_eq!(config.outputs.0[1].focus_at_startup, false);
    }

    #[test]
    fn apply_output_disabled() {
        use crate::LuaConfig;

        let code = r#"
        local outputs = {
            {
                name = "HDMI-A-2",
                off = true,
            },
        }
        return { outputs = outputs }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify output is disabled
        assert_eq!(config.outputs.0[0].name, "HDMI-A-2");
        assert_eq!(config.outputs.0[0].off, true);
    }

    #[test]
    fn apply_layout_basic_config() {
        use crate::LuaConfig;

        let code = r#"
        local layout = {
            gaps = 8.0,
            always_center_single_column = true,
            empty_workspace_above_first = false,
        }
        return { layout = layout }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify layout settings
        assert_eq!(config.layout.gaps, 8.0);
        assert_eq!(config.layout.always_center_single_column, true);
        assert_eq!(config.layout.empty_workspace_above_first, false);
    }

    #[test]
    fn apply_layout_center_and_display() {
        use crate::LuaConfig;

        let code = r#"
        local layout = {
            center_focused_column = "on-overflow",
            default_column_display = "tabbed",
        }
        return { layout = layout }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify enum values
        use niri_config::layout::CenterFocusedColumn;
        assert_eq!(
            config.layout.center_focused_column,
            CenterFocusedColumn::OnOverflow
        );
        assert_eq!(
            config.layout.default_column_display,
            niri_ipc::ColumnDisplay::Tabbed
        );
    }

    #[test]
    fn apply_layout_background_color_rgba() {
        use crate::LuaConfig;

        let code = r#"
        local layout = {
            background_color = { r = 0.5, g = 0.25, b = 0.75, a = 1.0 },
        }
        return { layout = layout }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify color
        assert_eq!(config.layout.background_color.r, 0.5);
        assert_eq!(config.layout.background_color.g, 0.25);
        assert_eq!(config.layout.background_color.b, 0.75);
        assert_eq!(config.layout.background_color.a, 1.0);
    }

    #[test]
    fn apply_layout_hex_color() {
        use crate::LuaConfig;

        let code = r##"
        local layout = {
            background_color = "#1e1e2e",
        }
        return { layout = layout }
        "##;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify hex color was parsed (30/255 ≈ 0.118, 46/255 ≈ 0.180)
        assert!((config.layout.background_color.r - 0.118).abs() < 0.01);
        assert!((config.layout.background_color.g - 0.118).abs() < 0.01);
        assert!((config.layout.background_color.b - 0.180).abs() < 0.01);
        assert_eq!(config.layout.background_color.a, 1.0);
    }

    #[test]
    fn apply_layout_struts() {
        use crate::LuaConfig;

        let code = r#"
        local layout = {
            struts = { left = 10.0, right = 20.0, top = 30.0, bottom = 40.0 },
        }
        return { layout = layout }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify struts
        assert_eq!(config.layout.struts.left.0, 10.0);
        assert_eq!(config.layout.struts.right.0, 20.0);
        assert_eq!(config.layout.struts.top.0, 30.0);
        assert_eq!(config.layout.struts.bottom.0, 40.0);
    }

    #[test]
    fn apply_layout_preset_column_widths() {
        use niri_config::layout::PresetSize;

        use crate::LuaConfig;

        let code = r#"
        local layout = {
            preset_column_widths = { 0.33, { proportion = 0.5 }, { fixed = 800 } },
        }
        return { layout = layout }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify preset widths
        assert_eq!(config.layout.preset_column_widths.len(), 3);
        assert!(
            matches!(config.layout.preset_column_widths[0], PresetSize::Proportion(p) if (p - 0.33).abs() < 0.01)
        );
        assert!(
            matches!(config.layout.preset_column_widths[1], PresetSize::Proportion(p) if (p - 0.5).abs() < 0.01)
        );
        assert!(matches!(
            config.layout.preset_column_widths[2],
            PresetSize::Fixed(800)
        ));
    }

    #[test]
    fn apply_layout_preset_window_heights() {
        use niri_config::layout::PresetSize;

        use crate::LuaConfig;

        let code = r#"
        local layout = {
            preset_window_heights = { 0.25, { proportion = 0.5 }, { fixed = 600 } },
        }
        return { layout = layout }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify preset heights
        assert_eq!(config.layout.preset_window_heights.len(), 3);
        assert!(
            matches!(config.layout.preset_window_heights[0], PresetSize::Proportion(p) if (p - 0.25).abs() < 0.01)
        );
        assert!(
            matches!(config.layout.preset_window_heights[1], PresetSize::Proportion(p) if (p - 0.5).abs() < 0.01)
        );
        assert!(matches!(
            config.layout.preset_window_heights[2],
            PresetSize::Fixed(600)
        ));
    }

    #[test]
    fn apply_layout_default_column_width() {
        use niri_config::layout::PresetSize;

        use crate::LuaConfig;

        let code = r#"
        local layout = {
            default_column_width = { proportion = 0.6 },
        }
        return { layout = layout }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify default column width
        assert!(
            matches!(config.layout.default_column_width, Some(PresetSize::Proportion(p)) if (p - 0.6).abs() < 0.01)
        );
    }

    #[test]
    fn apply_layout_focus_ring() {
        use crate::LuaConfig;

        let code = r##"
        local layout = {
            focus_ring = {
                off = false,
                width = 6.0,
                active_color = "#ff0000",
                inactive_color = { r = 0.5, g = 0.5, b = 0.5, a = 1.0 },
            },
        }
        return { layout = layout }
        "##;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify focus ring settings
        assert_eq!(config.layout.focus_ring.off, false);
        assert_eq!(config.layout.focus_ring.width, 6.0);
        assert!((config.layout.focus_ring.active_color.r - 1.0).abs() < 0.01);
        assert!((config.layout.focus_ring.active_color.g - 0.0).abs() < 0.01);
        assert!((config.layout.focus_ring.active_color.b - 0.0).abs() < 0.01);
        assert_eq!(config.layout.focus_ring.inactive_color.r, 0.5);
    }

    #[test]
    fn apply_layout_border() {
        use crate::LuaConfig;

        let code = r##"
        local layout = {
            border = {
                on = true,
                width = 2.0,
                active_color = "#00ff00",
            },
        }
        return { layout = layout }
        "##;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify border settings (on should set off to false)
        assert_eq!(config.layout.border.off, false);
        assert_eq!(config.layout.border.width, 2.0);
        assert!((config.layout.border.active_color.r - 0.0).abs() < 0.01);
        assert!((config.layout.border.active_color.g - 1.0).abs() < 0.01);
        assert!((config.layout.border.active_color.b - 0.0).abs() < 0.01);
    }

    #[test]
    fn apply_layout_insert_hint() {
        use crate::LuaConfig;

        let code = r##"
        local layout = {
            insert_hint = {
                on = true,
                color = "#0000ff",
            },
        }
        return { layout = layout }
        "##;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify insert hint settings
        assert_eq!(config.layout.insert_hint.off, false);
        assert!((config.layout.insert_hint.color.r - 0.0).abs() < 0.01);
        assert!((config.layout.insert_hint.color.g - 0.0).abs() < 0.01);
        assert!((config.layout.insert_hint.color.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn apply_layout_shadow() {
        use crate::LuaConfig;

        let code = r##"
        local layout = {
            shadow = {
                on = true,
                softness = 20.0,
                spread = 10.0,
                draw_behind_window = true,
                color = "#000000",
                offset = { x = 5.0, y = 10.0 },
            },
        }
        return { layout = layout }
        "##;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify shadow settings
        assert_eq!(config.layout.shadow.on, true);
        assert_eq!(config.layout.shadow.softness, 20.0);
        assert_eq!(config.layout.shadow.spread, 10.0);
        assert_eq!(config.layout.shadow.draw_behind_window, true);
        assert!((config.layout.shadow.color.r - 0.0).abs() < 0.01);
        assert!((config.layout.shadow.color.g - 0.0).abs() < 0.01);
        assert!((config.layout.shadow.color.b - 0.0).abs() < 0.01);
        assert_eq!(config.layout.shadow.offset.x.0, 5.0);
        assert_eq!(config.layout.shadow.offset.y.0, 10.0);
    }

    #[test]
    fn apply_cursor_config() {
        use crate::LuaConfig;

        let code = r#"
        local cursor = {
            xcursor_theme = "Adwaita",
            xcursor_size = 32,
            hide_when_typing = true,
            hide_after_inactive_ms = 5000,
        }
        return { cursor = cursor }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify cursor settings
        assert_eq!(config.cursor.xcursor_theme, "Adwaita");
        assert_eq!(config.cursor.xcursor_size, 32);
        assert_eq!(config.cursor.hide_when_typing, true);
        assert_eq!(config.cursor.hide_after_inactive_ms, Some(5000));
    }

    #[test]
    fn apply_screenshot_path_config() {
        use crate::LuaConfig;

        let code = r#"
        screenshot_path = "~/Pictures/Screenshots/niri_%Y-%m-%d_%H-%M-%S.png"
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify screenshot path
        assert_eq!(
            config.screenshot_path.0,
            Some("~/Pictures/Screenshots/niri_%Y-%m-%d_%H-%M-%S.png".to_string())
        );
    }

    #[test]
    fn apply_environment_config() {
        use crate::LuaConfig;

        let code = r#"
        local environment = {
            DISPLAY = ":0",
            WAYLAND_DISPLAY = "wayland-1",
            XDG_SESSION_TYPE = "wayland",
            EDITOR = "nvim",
        }
        return { environment = environment }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify environment variables
        assert_eq!(config.environment.0.len(), 4);

        // Check that all expected variables exist
        let has_display = config
            .environment
            .0
            .iter()
            .any(|v| v.name == "DISPLAY" && v.value == Some(":0".to_string()));
        let has_wayland = config
            .environment
            .0
            .iter()
            .any(|v| v.name == "WAYLAND_DISPLAY" && v.value == Some("wayland-1".to_string()));
        let has_session = config
            .environment
            .0
            .iter()
            .any(|v| v.name == "XDG_SESSION_TYPE" && v.value == Some("wayland".to_string()));
        let has_editor = config
            .environment
            .0
            .iter()
            .any(|v| v.name == "EDITOR" && v.value == Some("nvim".to_string()));

        assert!(has_display, "DISPLAY variable not found");
        assert!(has_wayland, "WAYLAND_DISPLAY variable not found");
        assert!(has_session, "XDG_SESSION_TYPE variable not found");
        assert!(has_editor, "EDITOR variable not found");
    }

    #[test]
    fn apply_debug_config() {
        use crate::LuaConfig;

        let code = r#"
        local debug = {
            preview_render = "screencast",
            enable_overlay_planes = true,
            disable_cursor_plane = false,
            disable_direct_scanout = true,
            emulate_zero_presentation_time = true,
            render_drm_device = "/dev/dri/renderD128",
            ignored_drm_devices = {"/dev/dri/card0", "/dev/dri/card1"},
        }
        return { debug = debug }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify debug settings
        use niri_config::debug::PreviewRender;
        assert!(matches!(
            config.debug.preview_render,
            Some(PreviewRender::Screencast)
        ));
        assert_eq!(config.debug.enable_overlay_planes, true);
        assert_eq!(config.debug.disable_cursor_plane, false);
        assert_eq!(config.debug.disable_direct_scanout, true);
        assert_eq!(config.debug.emulate_zero_presentation_time, true);
        assert_eq!(
            config.debug.render_drm_device,
            Some(std::path::PathBuf::from("/dev/dri/renderD128"))
        );
        assert_eq!(config.debug.ignored_drm_devices.len(), 2);
        assert!(config
            .debug
            .ignored_drm_devices
            .contains(&std::path::PathBuf::from("/dev/dri/card0")));
        assert!(config
            .debug
            .ignored_drm_devices
            .contains(&std::path::PathBuf::from("/dev/dri/card1")));
    }

    #[test]
    fn apply_workspaces_config() {
        use crate::LuaConfig;

        let code = r#"
        local workspaces = {
            "1",  -- Simple string format
            "2",
            { name = "3", open_on_output = "DP-1" },  -- Table format with output
            { name = "browser", open_on_output = "HDMI-A-1" },
            { name = "code" },  -- Table format without output
        }
        return { workspaces = workspaces }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify workspaces
        assert_eq!(config.workspaces.len(), 5);

        assert_eq!(config.workspaces[0].name.0, "1");
        assert_eq!(config.workspaces[0].open_on_output, None);

        assert_eq!(config.workspaces[1].name.0, "2");
        assert_eq!(config.workspaces[1].open_on_output, None);

        assert_eq!(config.workspaces[2].name.0, "3");
        assert_eq!(
            config.workspaces[2].open_on_output,
            Some("DP-1".to_string())
        );

        assert_eq!(config.workspaces[3].name.0, "browser");
        assert_eq!(
            config.workspaces[3].open_on_output,
            Some("HDMI-A-1".to_string())
        );

        assert_eq!(config.workspaces[4].name.0, "code");
        assert_eq!(config.workspaces[4].open_on_output, None);
    }

    #[test]
    fn apply_animations_spring_config() {
        use niri_config::animations::Kind;

        use crate::LuaConfig;

        let code = r#"
        local animations = {
            workspace_switch = {
                spring = {
                    damping_ratio = 1.0,
                    stiffness = 1000,
                    epsilon = 0.001,
                }
            }
        }
        return { animations = animations }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify workspace_switch animation
        let anim = &config.animations.workspace_switch.0;
        assert_eq!(anim.off, false);
        if let Kind::Spring(spring) = &anim.kind {
            assert_eq!(spring.damping_ratio, 1.0);
            assert_eq!(spring.stiffness, 1000);
            assert_eq!(spring.epsilon, 0.001);
        } else {
            panic!("Expected workspace_switch to have Spring animation");
        }
    }

    #[test]
    fn apply_animations_easing_config() {
        use niri_config::animations::{Curve, Kind};

        use crate::LuaConfig;

        let code = r#"
        local animations = {
            window_open = {
                duration_ms = 150,
                curve = "ease-out-cubic",
            }
        }
        return { animations = animations }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify window_open animation
        let anim = &config.animations.window_open.anim;
        assert_eq!(anim.off, false);
        if let Kind::Easing(easing) = &anim.kind {
            assert_eq!(easing.duration_ms, 150);
            assert_eq!(easing.curve, Curve::EaseOutCubic);
        } else {
            panic!(
                "Expected window_open to have Easing animation, got {:?}",
                anim.kind
            );
        }
    }

    #[test]
    fn apply_animations_all_curves() {
        use niri_config::animations::{Curve, Kind};

        use crate::LuaConfig;

        let code = r#"
        local animations = {
            window_open = {
                duration_ms = 100,
                curve = "linear",
            },
            window_close = {
                duration_ms = 150,
                curve = "ease-out-quad",
            },
            screenshot_ui_open = {
                duration_ms = 200,
                curve = "ease-out-expo",
            }
        }
        return { animations = animations }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify window_open with linear curve
        let window_open_anim = &config.animations.window_open.anim;
        if let Kind::Easing(easing) = &window_open_anim.kind {
            assert_eq!(easing.curve, Curve::Linear);
        } else {
            panic!("Expected window_open to have Easing animation");
        }

        // Verify window_close with ease-out-quad curve
        let window_close_anim = &config.animations.window_close.anim;
        if let Kind::Easing(easing) = &window_close_anim.kind {
            assert_eq!(easing.curve, Curve::EaseOutQuad);
        } else {
            panic!("Expected window_close to have Easing animation");
        }

        // Verify screenshot_ui_open with ease-out-expo curve
        let screenshot_anim = &config.animations.screenshot_ui_open.0;
        if let Kind::Easing(easing) = &screenshot_anim.kind {
            assert_eq!(easing.curve, Curve::EaseOutExpo);
        } else {
            panic!("Expected screenshot_ui_open to have Easing animation");
        }
    }

    #[test]
    fn apply_animations_with_custom_shader() {
        use crate::LuaConfig;

        let code = r#"
        local animations = {
            window_open = {
                duration_ms = 150,
                curve = "ease-out-cubic",
                custom_shader = "path/to/shader.frag",
            },
            window_close = {
                duration_ms = 100,
                curve = "ease-out-quad",
                custom_shader = "another/shader.frag",
            },
            window_resize = {
                spring = {
                    damping_ratio = 0.8,
                    stiffness = 800,
                    epsilon = 0.001,
                },
                custom_shader = "resize/shader.frag",
            }
        }
        return { animations = animations }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify window_open with custom shader
        assert_eq!(config.animations.window_open.anim.off, false);
        assert_eq!(
            config.animations.window_open.custom_shader,
            Some("path/to/shader.frag".to_string())
        );

        // Verify window_close with custom shader
        assert_eq!(config.animations.window_close.anim.off, false);
        assert_eq!(
            config.animations.window_close.custom_shader,
            Some("another/shader.frag".to_string())
        );

        // Verify window_resize with custom shader
        assert_eq!(config.animations.window_resize.anim.off, false);
        assert_eq!(
            config.animations.window_resize.custom_shader,
            Some("resize/shader.frag".to_string())
        );
    }

    #[test]
    fn apply_animations_global_settings() {
        use crate::LuaConfig;

        let code = r#"
        local animations = {
            slowdown = 2.5,
        }
        return { animations = animations }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify slowdown setting
        assert_eq!(config.animations.slowdown, 2.5);
    }

    #[test]
    fn apply_animations_all_types() {
        use crate::LuaConfig;

        let code = r#"
        local animations = {
            workspace_switch = {
                spring = {
                    damping_ratio = 1.0,
                    stiffness = 1000,
                    epsilon = 0.001,
                }
            },
            horizontal_view_movement = {
                spring = {
                    damping_ratio = 0.9,
                    stiffness = 900,
                    epsilon = 0.001,
                }
            },
            window_movement = {
                spring = {
                    damping_ratio = 0.85,
                    stiffness = 850,
                    epsilon = 0.001,
                }
            },
            window_open = {
                duration_ms = 150,
                curve = "ease-out-cubic",
            },
            window_close = {
                duration_ms = 100,
                curve = "ease-out-quad",
            },
            window_resize = {
                spring = {
                    damping_ratio = 0.8,
                    stiffness = 800,
                    epsilon = 0.001,
                }
            },
            config_notification_open_close = {
                spring = {
                    damping_ratio = 0.75,
                    stiffness = 750,
                    epsilon = 0.001,
                }
            },
            exit_confirmation_open_close = {
                spring = {
                    damping_ratio = 0.7,
                    stiffness = 700,
                    epsilon = 0.001,
                }
            },
            screenshot_ui_open = {
                duration_ms = 200,
                curve = "ease-out-expo",
            },
            overview_open_close = {
                spring = {
                    damping_ratio = 0.65,
                    stiffness = 650,
                    epsilon = 0.001,
                }
            },
            slowdown = 1.5,
        }
        return { animations = animations }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify all animations are set (not off)
        assert_eq!(config.animations.workspace_switch.0.off, false);
        assert_eq!(config.animations.horizontal_view_movement.0.off, false);
        assert_eq!(config.animations.window_movement.0.off, false);
        assert_eq!(config.animations.window_open.anim.off, false);
        assert_eq!(config.animations.window_close.anim.off, false);
        assert_eq!(config.animations.window_resize.anim.off, false);
        assert_eq!(
            config.animations.config_notification_open_close.0.off,
            false
        );
        assert_eq!(config.animations.exit_confirmation_open_close.0.off, false);
        assert_eq!(config.animations.screenshot_ui_open.0.off, false);
        assert_eq!(config.animations.overview_open_close.0.off, false);
        assert_eq!(config.animations.slowdown, 1.5);
    }

    #[test]
    fn apply_animations_off_flag() {
        use crate::LuaConfig;

        let code = r#"
        local animations = {
            workspace_switch = {
                off = true,
            }
        }
        return { animations = animations }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify workspace_switch animation is off
        assert_eq!(config.animations.workspace_switch.0.off, true);
    }

    #[test]
    fn apply_gestures_dnd_edge_view_scroll() {
        use crate::LuaConfig;

        let code = r#"
        local gestures = {
            dnd_edge_view_scroll = {
                trigger_width = 50.0,
                delay_ms = 200,
                max_speed = 2000.0,
            }
        }
        return { gestures = gestures }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify dnd_edge_view_scroll settings
        assert_eq!(config.gestures.dnd_edge_view_scroll.trigger_width, 50.0);
        assert_eq!(config.gestures.dnd_edge_view_scroll.delay_ms, 200);
        assert_eq!(config.gestures.dnd_edge_view_scroll.max_speed, 2000.0);
    }

    #[test]
    fn apply_gestures_dnd_edge_workspace_switch() {
        use crate::LuaConfig;

        let code = r#"
        local gestures = {
            dnd_edge_workspace_switch = {
                trigger_height = 75.0,
                delay_ms = 150,
                max_speed = 1500.0,
            }
        }
        return { gestures = gestures }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify dnd_edge_workspace_switch settings
        assert_eq!(
            config.gestures.dnd_edge_workspace_switch.trigger_height,
            75.0
        );
        assert_eq!(config.gestures.dnd_edge_workspace_switch.delay_ms, 150);
        assert_eq!(config.gestures.dnd_edge_workspace_switch.max_speed, 1500.0);
    }

    #[test]
    fn apply_gestures_hot_corners() {
        use crate::LuaConfig;

        let code = r#"
        local gestures = {
            hot_corners = {
                off = false,
                top_left = true,
                top_right = false,
                bottom_left = true,
                bottom_right = false,
            }
        }
        return { gestures = gestures }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify hot_corners settings
        assert_eq!(config.gestures.hot_corners.off, false);
        assert_eq!(config.gestures.hot_corners.top_left, true);
        assert_eq!(config.gestures.hot_corners.top_right, false);
        assert_eq!(config.gestures.hot_corners.bottom_left, true);
        assert_eq!(config.gestures.hot_corners.bottom_right, false);
    }

    #[test]
    fn apply_gestures_all_settings() {
        use crate::LuaConfig;

        let code = r#"
        local gestures = {
            dnd_edge_view_scroll = {
                trigger_width = 100.0,
                delay_ms = 300,
                max_speed = 3000.0,
            },
            dnd_edge_workspace_switch = {
                trigger_height = 80.0,
                delay_ms = 250,
                max_speed = 2500.0,
            },
            hot_corners = {
                off = true,
                top_left = false,
                top_right = true,
                bottom_left = false,
                bottom_right = true,
            }
        }
        return { gestures = gestures }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify all gesture settings
        assert_eq!(config.gestures.dnd_edge_view_scroll.trigger_width, 100.0);
        assert_eq!(config.gestures.dnd_edge_view_scroll.delay_ms, 300);
        assert_eq!(config.gestures.dnd_edge_view_scroll.max_speed, 3000.0);

        assert_eq!(
            config.gestures.dnd_edge_workspace_switch.trigger_height,
            80.0
        );
        assert_eq!(config.gestures.dnd_edge_workspace_switch.delay_ms, 250);
        assert_eq!(config.gestures.dnd_edge_workspace_switch.max_speed, 2500.0);

        assert_eq!(config.gestures.hot_corners.off, true);
        assert_eq!(config.gestures.hot_corners.top_left, false);
        assert_eq!(config.gestures.hot_corners.top_right, true);
        assert_eq!(config.gestures.hot_corners.bottom_left, false);
        assert_eq!(config.gestures.hot_corners.bottom_right, true);
    }

    #[test]
    fn apply_clipboard_disable_primary() {
        use crate::LuaConfig;

        let code = r#"
        local clipboard = {
            disable_primary = true
        }
        return { clipboard = clipboard }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        // Store original value
        let original_disable_primary = config.clipboard.disable_primary;

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify clipboard setting changed
        assert_eq!(config.clipboard.disable_primary, true);
        assert_ne!(config.clipboard.disable_primary, original_disable_primary);
    }

    #[test]
    fn apply_clipboard_enable_primary() {
        use crate::LuaConfig;

        let code = r#"
        local clipboard = {
            disable_primary = false
        }
        return { clipboard = clipboard }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify clipboard setting
        assert_eq!(config.clipboard.disable_primary, false);
    }

    #[test]
    fn apply_hotkey_overlay_skip_at_startup() {
        use crate::LuaConfig;

        let code = r#"
        local hotkey_overlay = {
            skip_at_startup = true
        }
        return { hotkey_overlay = hotkey_overlay }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        // Store original value
        let original_skip = config.hotkey_overlay.skip_at_startup;

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify hotkey_overlay setting changed
        assert_eq!(config.hotkey_overlay.skip_at_startup, true);
        assert_ne!(config.hotkey_overlay.skip_at_startup, original_skip);
    }

    #[test]
    fn apply_hotkey_overlay_hide_not_bound() {
        use crate::LuaConfig;

        let code = r#"
        local hotkey_overlay = {
            hide_not_bound = true
        }
        return { hotkey_overlay = hotkey_overlay }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        // Store original value
        let original_hide = config.hotkey_overlay.hide_not_bound;

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify hotkey_overlay setting changed
        assert_eq!(config.hotkey_overlay.hide_not_bound, true);
        assert_ne!(config.hotkey_overlay.hide_not_bound, original_hide);
    }

    #[test]
    fn apply_hotkey_overlay_all_settings() {
        use crate::LuaConfig;

        let code = r#"
        local hotkey_overlay = {
            skip_at_startup = true,
            hide_not_bound = false
        }
        return { hotkey_overlay = hotkey_overlay }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify both hotkey_overlay settings
        assert_eq!(config.hotkey_overlay.skip_at_startup, true);
        assert_eq!(config.hotkey_overlay.hide_not_bound, false);
    }

    #[test]
    fn apply_config_notification_disable_failed() {
        use crate::LuaConfig;

        let code = r#"
        local config_notification = {
            disable_failed = true
        }
        return { config_notification = config_notification }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        // Store original value
        let original_disable = config.config_notification.disable_failed;

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify config_notification setting changed
        assert_eq!(config.config_notification.disable_failed, true);
        assert_ne!(config.config_notification.disable_failed, original_disable);
    }

    #[test]
    fn apply_config_notification_enable_failed() {
        use crate::LuaConfig;

        let code = r#"
        local config_notification = {
            disable_failed = false
        }
        return { config_notification = config_notification }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify config_notification setting
        assert_eq!(config.config_notification.disable_failed, false);
    }

    #[test]
    fn apply_all_misc_configs_together() {
        use crate::LuaConfig;

        let code = r#"
        local gestures = {
            dnd_edge_view_scroll = {
                trigger_width = 60.0,
                delay_ms = 180,
                max_speed = 1800.0,
            },
            hot_corners = {
                off = false,
                top_right = true,
            }
        }

        local clipboard = {
            disable_primary = true
        }

        local hotkey_overlay = {
            skip_at_startup = false,
            hide_not_bound = true
        }

        local config_notification = {
            disable_failed = true
        }

        return {
            gestures = gestures,
            clipboard = clipboard,
            hotkey_overlay = hotkey_overlay,
            config_notification = config_notification
        }
        "#;

        let lua_config = LuaConfig::from_string(code).expect("Failed to create LuaConfig");
        let runtime = lua_config.runtime();

        let mut config = Config::default();

        apply_lua_config(runtime, &mut config).expect("Failed to apply config");

        // Verify all settings were applied correctly
        assert_eq!(config.gestures.dnd_edge_view_scroll.trigger_width, 60.0);
        assert_eq!(config.gestures.dnd_edge_view_scroll.delay_ms, 180);
        assert_eq!(config.gestures.dnd_edge_view_scroll.max_speed, 1800.0);
        assert_eq!(config.gestures.hot_corners.off, false);
        assert_eq!(config.gestures.hot_corners.top_right, true);

        assert_eq!(config.clipboard.disable_primary, true);

        assert_eq!(config.hotkey_overlay.skip_at_startup, false);
        assert_eq!(config.hotkey_overlay.hide_not_bound, true);

        assert_eq!(config.config_notification.disable_failed, true);
    }
}
