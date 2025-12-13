//! Action proxy system for the `niri.action` namespace.
//!
//! This module implements Phase R5 of the API refactor, providing direct access
//! to all compositor actions via `niri.action.*` methods.
//!
//! Example usage:
//! ```lua
//! niri.action.spawn({"kitty"})
//! niri.action.focus_column_left()
//! niri.action.set_window_width("+10%")
//! niri.action.focus_workspace(2)
//! ```

use std::sync::Arc;

/// Macro to register multiple no-argument action methods.
///
/// This reduces boilerplate for actions that take no parameters and have
/// a direct mapping from Lua method name to Action variant.
///
/// # Usage
/// ```ignore
/// register_actions!(methods,
///     "method_name" => ActionVariant,
///     "another_method" => AnotherVariant,
/// );
/// ```
macro_rules! register_actions {
    ($methods:expr, $( $name:literal => $action:ident ),* $(,)?) => {
        $(
            $methods.add_method($name, |_lua, this, ()| {
                this.execute(Action::$action {})
            });
        )*
    };
}

use log::debug;
use mlua::prelude::*;
use niri_ipc::{Action, LayoutSwitchTarget, PositionChange, SizeChange, WorkspaceReferenceArg};

use crate::parse_utils;

/// Type alias for the action execution callback.
/// This callback sends actions to the compositor for execution.
pub type ActionCallback = Arc<dyn Fn(Action) -> LuaResult<()> + Send + Sync>;

/// Proxy for the `niri.action` namespace.
///
/// This struct holds the callback for executing actions.
/// It's used internally to create the action table with functions.
#[derive(Clone)]
pub struct ActionProxy {
    /// Callback to execute actions
    callback: ActionCallback,
}

impl ActionProxy {
    /// Create a new action proxy with the given callback
    pub fn new(callback: ActionCallback) -> Self {
        Self { callback }
    }

    /// Execute an action via the callback
    pub fn execute(&self, action: Action) -> LuaResult<()> {
        debug!("ActionProxy executing: {:?}", action);
        (self.callback)(action)
    }
}

/// Parse a SizeChange from a Lua value.
/// Accepts formats like: "100", "50%", "+10", "-10", "+5%", "-5%"
fn parse_size_change(value: LuaValue) -> LuaResult<SizeChange> {
    match value {
        LuaValue::Integer(n) => Ok(SizeChange::SetFixed(n as i32)),
        LuaValue::Number(n) => Ok(SizeChange::SetProportion(n / 100.0)),
        LuaValue::String(s) => {
            let s = s.to_str()?;
            parse_utils::parse_size_change(&s)
                .ok_or_else(|| LuaError::external(format!("invalid size change: {}", s)))
        }
        _ => Err(LuaError::external("size change must be a number or string")),
    }
}

/// Parse a PositionChange from a Lua value.
fn parse_position_change(value: LuaValue) -> LuaResult<PositionChange> {
    match value {
        LuaValue::Integer(n) => Ok(PositionChange::SetFixed(n as f64)),
        LuaValue::Number(n) => Ok(PositionChange::SetFixed(n)),
        LuaValue::String(s) => {
            let s = s.to_str()?;
            parse_position_change_str(&s)
        }
        _ => Err(LuaError::external(
            "position change must be a number or string",
        )),
    }
}

fn parse_position_change_str(s: &str) -> LuaResult<PositionChange> {
    let s = s.trim();
    if s.is_empty() {
        return Err(LuaError::external("position change cannot be empty"));
    }

    let is_relative = s.starts_with('+') || s.starts_with('-');
    let is_proportion = s.ends_with('%');

    let num_str = s
        .trim_start_matches('+')
        .trim_start_matches('-')
        .trim_end_matches('%');

    if is_proportion {
        let value: f64 = num_str
            .parse()
            .map_err(|_| LuaError::external(format!("invalid proportion: {}", s)))?;
        let proportion = value / 100.0;
        if is_relative {
            if s.starts_with('-') {
                Ok(PositionChange::AdjustProportion(-proportion))
            } else {
                Ok(PositionChange::AdjustProportion(proportion))
            }
        } else {
            Ok(PositionChange::SetProportion(proportion))
        }
    } else {
        let value: f64 = num_str
            .parse()
            .map_err(|_| LuaError::external(format!("invalid position: {}", s)))?;
        if is_relative {
            if s.starts_with('-') {
                Ok(PositionChange::AdjustFixed(-value))
            } else {
                Ok(PositionChange::AdjustFixed(value))
            }
        } else {
            Ok(PositionChange::SetFixed(value))
        }
    }
}

/// Parse a WorkspaceReferenceArg from a Lua value.
/// Accepts: index (number), name (string), or table { id = N } / { index = N } / { name = "..." }
fn parse_workspace_reference(value: LuaValue) -> LuaResult<WorkspaceReferenceArg> {
    match value {
        LuaValue::Integer(n) => Ok(WorkspaceReferenceArg::Index(n as u8)),
        LuaValue::Number(n) => Ok(WorkspaceReferenceArg::Index(n as u8)),
        LuaValue::String(s) => {
            let s = s.to_str()?;
            // Try parsing as number first
            if let Ok(index) = s.parse::<u8>() {
                Ok(WorkspaceReferenceArg::Index(index))
            } else {
                Ok(WorkspaceReferenceArg::Name(s.to_string()))
            }
        }
        LuaValue::Table(t) => {
            if let Ok(id) = t.get::<u64>("id") {
                Ok(WorkspaceReferenceArg::Id(id))
            } else if let Ok(index) = t.get::<u8>("index") {
                Ok(WorkspaceReferenceArg::Index(index))
            } else if let Ok(name) = t.get::<String>("name") {
                Ok(WorkspaceReferenceArg::Name(name))
            } else {
                Err(LuaError::external(
                    "workspace reference table must have 'id', 'index', or 'name'",
                ))
            }
        }
        _ => Err(LuaError::external(
            "workspace reference must be a number, string, or table",
        )),
    }
}

/// Parse a LayoutSwitchTarget from a Lua value.
fn parse_layout_switch_target(value: LuaValue) -> LuaResult<LayoutSwitchTarget> {
    match value {
        LuaValue::Integer(n) => Ok(LayoutSwitchTarget::Index(n as u8)),
        LuaValue::Number(n) => Ok(LayoutSwitchTarget::Index(n as u8)),
        LuaValue::String(s) => {
            let s = s.to_str()?;
            match s.to_lowercase().as_str() {
                "next" => Ok(LayoutSwitchTarget::Next),
                "prev" | "previous" => Ok(LayoutSwitchTarget::Prev),
                _ => {
                    if let Ok(index) = s.parse::<u8>() {
                        Ok(LayoutSwitchTarget::Index(index))
                    } else {
                        Err(LuaError::external(format!(
                            "invalid layout target: {} (use 'next', 'prev', or a number)",
                            s
                        )))
                    }
                }
            }
        }
        _ => Err(LuaError::external(
            "layout target must be a number or string ('next', 'prev')",
        )),
    }
}

impl LuaUserData for ActionProxy {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // ============================================================
        // No-argument actions (via macro)
        // ============================================================
        register_actions!(methods,
            // System
            "power_off_monitors" => PowerOffMonitors,
            "power_on_monitors" => PowerOnMonitors,
            "load_config_file" => LoadConfigFile,
            // Window focus
            "focus_window_previous" => FocusWindowPrevious,
            "toggle_keyboard_shortcuts_inhibit" => ToggleKeyboardShortcutsInhibit,
            // Column focus
            "focus_column_left" => FocusColumnLeft,
            "focus_column_right" => FocusColumnRight,
            "focus_column_first" => FocusColumnFirst,
            "focus_column_last" => FocusColumnLast,
            "focus_column_right_or_first" => FocusColumnRightOrFirst,
            "focus_column_left_or_last" => FocusColumnLeftOrLast,
            // Window focus (vertical)
            "focus_window_down" => FocusWindowDown,
            "focus_window_up" => FocusWindowUp,
            "focus_window_or_monitor_up" => FocusWindowOrMonitorUp,
            "focus_window_or_monitor_down" => FocusWindowOrMonitorDown,
            "focus_column_or_monitor_left" => FocusColumnOrMonitorLeft,
            "focus_column_or_monitor_right" => FocusColumnOrMonitorRight,
            "focus_window_down_or_column_left" => FocusWindowDownOrColumnLeft,
            "focus_window_down_or_column_right" => FocusWindowDownOrColumnRight,
            "focus_window_up_or_column_left" => FocusWindowUpOrColumnLeft,
            "focus_window_up_or_column_right" => FocusWindowUpOrColumnRight,
            "focus_window_or_workspace_down" => FocusWindowOrWorkspaceDown,
            "focus_window_or_workspace_up" => FocusWindowOrWorkspaceUp,
            "focus_window_top" => FocusWindowTop,
            "focus_window_bottom" => FocusWindowBottom,
            "focus_window_down_or_top" => FocusWindowDownOrTop,
            "focus_window_up_or_bottom" => FocusWindowUpOrBottom,
            // Column move
            "move_column_left" => MoveColumnLeft,
            "move_column_right" => MoveColumnRight,
            "move_column_to_first" => MoveColumnToFirst,
            "move_column_to_last" => MoveColumnToLast,
            "move_column_left_or_to_monitor_left" => MoveColumnLeftOrToMonitorLeft,
            "move_column_right_or_to_monitor_right" => MoveColumnRightOrToMonitorRight,
            // Window move (vertical)
            "move_window_down" => MoveWindowDown,
            "move_window_up" => MoveWindowUp,
            "move_window_down_or_to_workspace_down" => MoveWindowDownOrToWorkspaceDown,
            "move_window_up_or_to_workspace_up" => MoveWindowUpOrToWorkspaceUp,
            // Consume/expel
            "consume_window_into_column" => ConsumeWindowIntoColumn,
            "expel_window_from_column" => ExpelWindowFromColumn,
            "swap_window_right" => SwapWindowRight,
            "swap_window_left" => SwapWindowLeft,
            // Column display
            "toggle_column_tabbed_display" => ToggleColumnTabbedDisplay,
            "center_column" => CenterColumn,
            "center_visible_columns" => CenterVisibleColumns,
            // Workspace
            "focus_workspace_down" => FocusWorkspaceDown,
            "focus_workspace_up" => FocusWorkspaceUp,
            "focus_workspace_previous" => FocusWorkspacePrevious,
            "move_workspace_down" => MoveWorkspaceDown,
            "move_workspace_up" => MoveWorkspaceUp,
            // Monitor focus
            "focus_monitor_left" => FocusMonitorLeft,
            "focus_monitor_right" => FocusMonitorRight,
            "focus_monitor_down" => FocusMonitorDown,
            "focus_monitor_up" => FocusMonitorUp,
            "focus_monitor_previous" => FocusMonitorPrevious,
            "focus_monitor_next" => FocusMonitorNext,
            // Window to monitor
            "move_window_to_monitor_left" => MoveWindowToMonitorLeft,
            "move_window_to_monitor_right" => MoveWindowToMonitorRight,
            "move_window_to_monitor_down" => MoveWindowToMonitorDown,
            "move_window_to_monitor_up" => MoveWindowToMonitorUp,
            "move_window_to_monitor_previous" => MoveWindowToMonitorPrevious,
            "move_window_to_monitor_next" => MoveWindowToMonitorNext,
            // Column to monitor
            "move_column_to_monitor_left" => MoveColumnToMonitorLeft,
            "move_column_to_monitor_right" => MoveColumnToMonitorRight,
            "move_column_to_monitor_down" => MoveColumnToMonitorDown,
            "move_column_to_monitor_up" => MoveColumnToMonitorUp,
            "move_column_to_monitor_previous" => MoveColumnToMonitorPrevious,
            "move_column_to_monitor_next" => MoveColumnToMonitorNext,
            // Size/width
            "switch_preset_column_width" => SwitchPresetColumnWidth,
            "switch_preset_column_width_back" => SwitchPresetColumnWidthBack,
            "maximize_column" => MaximizeColumn,
            "expand_column_to_available_width" => ExpandColumnToAvailableWidth,
            // Layout
            "show_hotkey_overlay" => ShowHotkeyOverlay,
            // Workspace to monitor
            "move_workspace_to_monitor_left" => MoveWorkspaceToMonitorLeft,
            "move_workspace_to_monitor_right" => MoveWorkspaceToMonitorRight,
            "move_workspace_to_monitor_down" => MoveWorkspaceToMonitorDown,
            "move_workspace_to_monitor_up" => MoveWorkspaceToMonitorUp,
            "move_workspace_to_monitor_previous" => MoveWorkspaceToMonitorPrevious,
            "move_workspace_to_monitor_next" => MoveWorkspaceToMonitorNext,
            // Debug
            "toggle_debug_tint" => ToggleDebugTint,
            "debug_toggle_opaque_regions" => DebugToggleOpaqueRegions,
            "debug_toggle_damage" => DebugToggleDamage,
            // Floating
            "focus_floating" => FocusFloating,
            "focus_tiling" => FocusTiling,
            "switch_focus_between_floating_and_tiling" => SwitchFocusBetweenFloatingAndTiling,
            // Dynamic cast
            "clear_dynamic_cast_target" => ClearDynamicCastTarget,
            // Overview
            "toggle_overview" => ToggleOverview,
            "open_overview" => OpenOverview,
            "close_overview" => CloseOverview,
        );

        // ============================================================
        // Actions with parameters (manual registration)
        // ============================================================

        // quit(skip_confirmation?)
        methods.add_method("quit", |_lua, this, skip_confirmation: Option<bool>| {
            this.execute(Action::Quit {
                skip_confirmation: skip_confirmation.unwrap_or(false),
            })
        });

        // spawn(command) - command is array of strings
        methods.add_method("spawn", |_lua, this, command: Vec<String>| {
            this.execute(Action::Spawn { command })
        });

        // spawn_sh(command) - command is a shell string
        methods.add_method("spawn_sh", |_lua, this, command: String| {
            this.execute(Action::SpawnSh { command })
        });

        // do_screen_transition(delay_ms?)
        methods.add_method(
            "do_screen_transition",
            |_lua, this, delay_ms: Option<u16>| {
                this.execute(Action::DoScreenTransition { delay_ms })
            },
        );

        // screenshot(show_pointer?, path?)
        methods.add_method(
            "screenshot",
            |_lua, this, (show_pointer, path): (Option<bool>, Option<String>)| {
                this.execute(Action::Screenshot {
                    show_pointer: show_pointer.unwrap_or(true),
                    path,
                })
            },
        );

        // screenshot_screen(write_to_disk?, show_pointer?, path?)
        methods.add_method(
            "screenshot_screen",
            |_lua,
             this,
             (write_to_disk, show_pointer, path): (Option<bool>, Option<bool>, Option<String>)| {
                this.execute(Action::ScreenshotScreen {
                    write_to_disk: write_to_disk.unwrap_or(true),
                    show_pointer: show_pointer.unwrap_or(true),
                    path,
                })
            },
        );

        // screenshot_window(id?, write_to_disk?, path?)
        methods.add_method(
            "screenshot_window",
            |_lua, this, (id, write_to_disk, path): (Option<u64>, Option<bool>, Option<String>)| {
                this.execute(Action::ScreenshotWindow {
                    id,
                    write_to_disk: write_to_disk.unwrap_or(true),
                    path,
                })
            },
        );

        // close_window(id?)
        methods.add_method("close_window", |_lua, this, id: Option<u64>| {
            this.execute(Action::CloseWindow { id })
        });

        // fullscreen_window(id?)
        methods.add_method("fullscreen_window", |_lua, this, id: Option<u64>| {
            this.execute(Action::FullscreenWindow { id })
        });

        // toggle_windowed_fullscreen(id?)
        methods.add_method(
            "toggle_windowed_fullscreen",
            |_lua, this, id: Option<u64>| this.execute(Action::ToggleWindowedFullscreen { id }),
        );

        // focus_window(id)
        methods.add_method("focus_window", |_lua, this, id: u64| {
            this.execute(Action::FocusWindow { id })
        });

        // focus_window_in_column(index)
        methods.add_method("focus_window_in_column", |_lua, this, index: u8| {
            this.execute(Action::FocusWindowInColumn { index })
        });

        // focus_column(index)
        methods.add_method("focus_column", |_lua, this, index: usize| {
            this.execute(Action::FocusColumn { index })
        });

        // move_column_to_index(index)
        methods.add_method("move_column_to_index", |_lua, this, index: usize| {
            this.execute(Action::MoveColumnToIndex { index })
        });

        // consume_or_expel_window_left(id?)
        methods.add_method(
            "consume_or_expel_window_left",
            |_lua, this, id: Option<u64>| this.execute(Action::ConsumeOrExpelWindowLeft { id }),
        );

        // consume_or_expel_window_right(id?)
        methods.add_method(
            "consume_or_expel_window_right",
            |_lua, this, id: Option<u64>| this.execute(Action::ConsumeOrExpelWindowRight { id }),
        );

        // set_column_display(display) - "normal" or "tabbed"
        methods.add_method("set_column_display", |_lua, this, display: String| {
            let display = match display.to_lowercase().as_str() {
                "normal" => niri_ipc::ColumnDisplay::Normal,
                "tabbed" => niri_ipc::ColumnDisplay::Tabbed,
                _ => {
                    return Err(LuaError::external(format!(
                        "invalid column display: {} (use 'normal' or 'tabbed')",
                        display
                    )))
                }
            };
            this.execute(Action::SetColumnDisplay { display })
        });

        // center_window(id?)
        methods.add_method("center_window", |_lua, this, id: Option<u64>| {
            this.execute(Action::CenterWindow { id })
        });

        // focus_workspace(reference) - index, name, or {id=N}
        methods.add_method("focus_workspace", |_lua, this, reference: LuaValue| {
            let reference = parse_workspace_reference(reference)?;
            this.execute(Action::FocusWorkspace { reference })
        });

        // move_window_to_workspace_down(focus?)
        methods.add_method(
            "move_window_to_workspace_down",
            |_lua, this, focus: Option<bool>| {
                this.execute(Action::MoveWindowToWorkspaceDown {
                    focus: focus.unwrap_or(true),
                })
            },
        );

        // move_window_to_workspace_up(focus?)
        methods.add_method(
            "move_window_to_workspace_up",
            |_lua, this, focus: Option<bool>| {
                this.execute(Action::MoveWindowToWorkspaceUp {
                    focus: focus.unwrap_or(true),
                })
            },
        );

        // move_window_to_workspace(reference, window_id?, focus?)
        methods.add_method(
            "move_window_to_workspace",
            |_lua, this, (reference, window_id, focus): (LuaValue, Option<u64>, Option<bool>)| {
                let reference = parse_workspace_reference(reference)?;
                this.execute(Action::MoveWindowToWorkspace {
                    window_id,
                    reference,
                    focus: focus.unwrap_or(true),
                })
            },
        );

        // move_column_to_workspace_down(focus?)
        methods.add_method(
            "move_column_to_workspace_down",
            |_lua, this, focus: Option<bool>| {
                this.execute(Action::MoveColumnToWorkspaceDown {
                    focus: focus.unwrap_or(true),
                })
            },
        );

        // move_column_to_workspace_up(focus?)
        methods.add_method(
            "move_column_to_workspace_up",
            |_lua, this, focus: Option<bool>| {
                this.execute(Action::MoveColumnToWorkspaceUp {
                    focus: focus.unwrap_or(true),
                })
            },
        );

        // move_column_to_workspace(reference, focus?)
        methods.add_method(
            "move_column_to_workspace",
            |_lua, this, (reference, focus): (LuaValue, Option<bool>)| {
                let reference = parse_workspace_reference(reference)?;
                this.execute(Action::MoveColumnToWorkspace {
                    reference,
                    focus: focus.unwrap_or(true),
                })
            },
        );

        // move_workspace_to_index(index, reference?)
        methods.add_method(
            "move_workspace_to_index",
            |_lua, this, (index, reference): (usize, Option<LuaValue>)| {
                let reference = reference.map(parse_workspace_reference).transpose()?;
                this.execute(Action::MoveWorkspaceToIndex { index, reference })
            },
        );

        // set_workspace_name(name, workspace?)
        methods.add_method(
            "set_workspace_name",
            |_lua, this, (name, workspace): (String, Option<LuaValue>)| {
                let workspace = workspace.map(parse_workspace_reference).transpose()?;
                this.execute(Action::SetWorkspaceName { name, workspace })
            },
        );

        // unset_workspace_name(reference?)
        methods.add_method(
            "unset_workspace_name",
            |_lua, this, reference: Option<LuaValue>| {
                let reference = reference.map(parse_workspace_reference).transpose()?;
                this.execute(Action::UnsetWorkspaceName { reference })
            },
        );

        // focus_monitor(output)
        methods.add_method("focus_monitor", |_lua, this, output: String| {
            this.execute(Action::FocusMonitor { output })
        });

        // move_window_to_monitor(output, id?)
        methods.add_method(
            "move_window_to_monitor",
            |_lua, this, (output, id): (String, Option<u64>)| {
                this.execute(Action::MoveWindowToMonitor { id, output })
            },
        );

        // move_column_to_monitor(output)
        methods.add_method("move_column_to_monitor", |_lua, this, output: String| {
            this.execute(Action::MoveColumnToMonitor { output })
        });

        // set_window_width(change, id?)
        methods.add_method(
            "set_window_width",
            |_lua, this, (change, id): (LuaValue, Option<u64>)| {
                let change = parse_size_change(change)?;
                this.execute(Action::SetWindowWidth { id, change })
            },
        );

        // set_window_height(change, id?)
        methods.add_method(
            "set_window_height",
            |_lua, this, (change, id): (LuaValue, Option<u64>)| {
                let change = parse_size_change(change)?;
                this.execute(Action::SetWindowHeight { id, change })
            },
        );

        // reset_window_height(id?)
        methods.add_method("reset_window_height", |_lua, this, id: Option<u64>| {
            this.execute(Action::ResetWindowHeight { id })
        });

        // switch_preset_window_width(id?)
        methods.add_method(
            "switch_preset_window_width",
            |_lua, this, id: Option<u64>| this.execute(Action::SwitchPresetWindowWidth { id }),
        );

        // switch_preset_window_width_back(id?)
        methods.add_method(
            "switch_preset_window_width_back",
            |_lua, this, id: Option<u64>| this.execute(Action::SwitchPresetWindowWidthBack { id }),
        );

        // switch_preset_window_height(id?)
        methods.add_method(
            "switch_preset_window_height",
            |_lua, this, id: Option<u64>| this.execute(Action::SwitchPresetWindowHeight { id }),
        );

        // switch_preset_window_height_back(id?)
        methods.add_method(
            "switch_preset_window_height_back",
            |_lua, this, id: Option<u64>| this.execute(Action::SwitchPresetWindowHeightBack { id }),
        );

        // maximize_window_to_edges(id?)
        methods.add_method("maximize_window_to_edges", |_lua, this, id: Option<u64>| {
            this.execute(Action::MaximizeWindowToEdges { id })
        });

        // set_column_width(change)
        methods.add_method("set_column_width", |_lua, this, change: LuaValue| {
            let change = parse_size_change(change)?;
            this.execute(Action::SetColumnWidth { change })
        });

        // switch_layout(layout) - "next", "prev", or index
        methods.add_method("switch_layout", |_lua, this, layout: LuaValue| {
            let layout = parse_layout_switch_target(layout)?;
            this.execute(Action::SwitchLayout { layout })
        });

        // move_workspace_to_monitor(output, reference?)
        methods.add_method(
            "move_workspace_to_monitor",
            |_lua, this, (output, reference): (String, Option<LuaValue>)| {
                let reference = reference.map(parse_workspace_reference).transpose()?;
                this.execute(Action::MoveWorkspaceToMonitor { output, reference })
            },
        );

        // toggle_window_floating(id?)
        methods.add_method("toggle_window_floating", |_lua, this, id: Option<u64>| {
            this.execute(Action::ToggleWindowFloating { id })
        });

        // move_window_to_floating(id?)
        methods.add_method("move_window_to_floating", |_lua, this, id: Option<u64>| {
            this.execute(Action::MoveWindowToFloating { id })
        });

        // move_window_to_tiling(id?)
        methods.add_method("move_window_to_tiling", |_lua, this, id: Option<u64>| {
            this.execute(Action::MoveWindowToTiling { id })
        });

        // move_floating_window(x, y, id?)
        methods.add_method(
            "move_floating_window",
            |_lua, this, (x, y, id): (LuaValue, LuaValue, Option<u64>)| {
                let x = parse_position_change(x)?;
                let y = parse_position_change(y)?;
                this.execute(Action::MoveFloatingWindow { id, x, y })
            },
        );

        // toggle_window_rule_opacity(id?)
        methods.add_method(
            "toggle_window_rule_opacity",
            |_lua, this, id: Option<u64>| this.execute(Action::ToggleWindowRuleOpacity { id }),
        );

        // set_dynamic_cast_window(id?)
        methods.add_method("set_dynamic_cast_window", |_lua, this, id: Option<u64>| {
            this.execute(Action::SetDynamicCastWindow { id })
        });

        // set_dynamic_cast_monitor(output?)
        methods.add_method(
            "set_dynamic_cast_monitor",
            |_lua, this, output: Option<String>| {
                this.execute(Action::SetDynamicCastMonitor { output })
            },
        );

        // toggle_window_urgent(id)
        methods.add_method("toggle_window_urgent", |_lua, this, id: u64| {
            this.execute(Action::ToggleWindowUrgent { id })
        });

        // set_window_urgent(id)
        methods.add_method("set_window_urgent", |_lua, this, id: u64| {
            this.execute(Action::SetWindowUrgent { id })
        });

        // unset_window_urgent(id)
        methods.add_method("unset_window_urgent", |_lua, this, id: u64| {
            this.execute(Action::UnsetWindowUrgent { id })
        });
    }
}

/// Register the action proxy to the Lua runtime.
///
/// This creates the `niri.action` table as a userdata with methods.
///
/// # Arguments
/// * `lua` - The Lua runtime
/// * `callback` - Callback to execute actions
///
/// # Returns
/// LuaResult indicating success or Lua error
pub fn register_action_proxy(lua: &Lua, callback: ActionCallback) -> LuaResult<()> {
    let globals = lua.globals();

    // Get or create the niri table
    let niri_table: LuaTable = match globals.get::<LuaValue>("niri")? {
        LuaValue::Table(t) => t,
        LuaValue::Nil => {
            let t = lua.create_table()?;
            globals.set("niri", t.clone())?;
            t
        }
        _ => return Err(LuaError::external("niri global is not a table")),
    };

    let proxy = ActionProxy::new(callback);
    niri_table.set("action", proxy)?;

    debug!("Registered action proxy to niri.action");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_env() -> (Lua, Arc<std::sync::Mutex<Vec<Action>>>) {
        let lua = Lua::new();
        let actions: Arc<std::sync::Mutex<Vec<Action>>> = Arc::new(std::sync::Mutex::new(vec![]));

        // Create niri namespace
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();

        let actions_clone = actions.clone();
        let callback: ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        register_action_proxy(&lua, callback).unwrap();

        (lua, actions)
    }

    #[test]
    fn test_action_proxy_creation() {
        let (lua, _actions) = create_test_env();

        // Verify niri.action exists
        let result: LuaResult<LuaValue> = lua.load("return niri.action").eval();
        assert!(result.is_ok());
        assert!(!matches!(result.unwrap(), LuaValue::Nil));
    }

    #[test]
    fn test_quit_action() {
        let (lua, actions) = create_test_env();

        lua.load("niri.action:quit()").exec().unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0],
            Action::Quit {
                skip_confirmation: false
            }
        ));
    }

    #[test]
    fn test_quit_with_skip_confirmation() {
        let (lua, actions) = create_test_env();

        lua.load("niri.action:quit(true)").exec().unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0],
            Action::Quit {
                skip_confirmation: true
            }
        ));
    }

    #[test]
    fn test_spawn_action() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:spawn({"kitty", "-e", "htop"})"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::Spawn { command } = &actions[0] {
            assert_eq!(command, &vec!["kitty", "-e", "htop"]);
        } else {
            panic!("Expected Spawn action");
        }
    }

    #[test]
    fn test_spawn_sh_action() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:spawn_sh("echo hello | grep h")"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::SpawnSh { command } = &actions[0] {
            assert_eq!(command, "echo hello | grep h");
        } else {
            panic!("Expected SpawnSh action");
        }
    }

    #[test]
    fn test_focus_column_actions() {
        let (lua, actions) = create_test_env();

        lua.load("niri.action:focus_column_left()").exec().unwrap();
        lua.load("niri.action:focus_column_right()").exec().unwrap();
        lua.load("niri.action:focus_column_first()").exec().unwrap();
        lua.load("niri.action:focus_column_last()").exec().unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 4);
        assert!(matches!(actions[0], Action::FocusColumnLeft {}));
        assert!(matches!(actions[1], Action::FocusColumnRight {}));
        assert!(matches!(actions[2], Action::FocusColumnFirst {}));
        assert!(matches!(actions[3], Action::FocusColumnLast {}));
    }

    #[test]
    fn test_focus_workspace_with_index() {
        let (lua, actions) = create_test_env();

        lua.load("niri.action:focus_workspace(2)").exec().unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::FocusWorkspace { reference } = &actions[0] {
            assert!(matches!(reference, WorkspaceReferenceArg::Index(2)));
        } else {
            panic!("Expected FocusWorkspace action");
        }
    }

    #[test]
    fn test_focus_workspace_with_name() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:focus_workspace("main")"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::FocusWorkspace { reference } = &actions[0] {
            if let WorkspaceReferenceArg::Name(name) = reference {
                assert_eq!(name, "main");
            } else {
                panic!("Expected Name reference");
            }
        } else {
            panic!("Expected FocusWorkspace action");
        }
    }

    #[test]
    fn test_set_window_width_fixed() {
        let (lua, actions) = create_test_env();

        lua.load("niri.action:set_window_width(500)")
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::SetWindowWidth { id, change } = &actions[0] {
            assert!(id.is_none());
            assert!(matches!(change, SizeChange::SetFixed(500)));
        } else {
            panic!("Expected SetWindowWidth action");
        }
    }

    #[test]
    fn test_set_window_width_proportion() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:set_window_width("50%")"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::SetWindowWidth { id, change } = &actions[0] {
            assert!(id.is_none());
            if let SizeChange::SetProportion(p) = change {
                assert!((p - 0.5).abs() < 0.001);
            } else {
                panic!("Expected SetProportion");
            }
        } else {
            panic!("Expected SetWindowWidth action");
        }
    }

    #[test]
    fn test_set_window_width_adjust() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:set_window_width("+10")"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::SetWindowWidth { change, .. } = &actions[0] {
            assert!(matches!(change, SizeChange::AdjustFixed(10)));
        } else {
            panic!("Expected SetWindowWidth action");
        }
    }

    #[test]
    fn test_set_window_width_adjust_negative() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:set_window_width("-10")"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::SetWindowWidth { change, .. } = &actions[0] {
            assert!(matches!(change, SizeChange::AdjustFixed(-10)));
        } else {
            panic!("Expected SetWindowWidth action");
        }
    }

    #[test]
    fn test_set_column_display() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:set_column_display("tabbed")"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::SetColumnDisplay { display } = &actions[0] {
            assert!(matches!(display, niri_ipc::ColumnDisplay::Tabbed));
        } else {
            panic!("Expected SetColumnDisplay action");
        }
    }

    #[test]
    fn test_switch_layout() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:switch_layout("next")"#)
            .exec()
            .unwrap();
        lua.load(r#"niri.action:switch_layout("prev")"#)
            .exec()
            .unwrap();
        lua.load("niri.action:switch_layout(1)").exec().unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 3);

        if let Action::SwitchLayout { layout } = &actions[0] {
            assert!(matches!(layout, LayoutSwitchTarget::Next));
        } else {
            panic!("Expected SwitchLayout Next");
        }

        if let Action::SwitchLayout { layout } = &actions[1] {
            assert!(matches!(layout, LayoutSwitchTarget::Prev));
        } else {
            panic!("Expected SwitchLayout Prev");
        }

        if let Action::SwitchLayout { layout } = &actions[2] {
            assert!(matches!(layout, LayoutSwitchTarget::Index(1)));
        } else {
            panic!("Expected SwitchLayout Index");
        }
    }

    #[test]
    fn test_move_floating_window() {
        let (lua, actions) = create_test_env();

        lua.load(r#"niri.action:move_floating_window("+10", "-5%")"#)
            .exec()
            .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::MoveFloatingWindow { id, x, y } = &actions[0] {
            assert!(id.is_none());
            assert!(matches!(x, PositionChange::AdjustFixed(f) if (*f - 10.0).abs() < 0.001));
            if let PositionChange::AdjustProportion(p) = y {
                assert!((p + 0.05).abs() < 0.001);
            } else {
                panic!("Expected AdjustProportion for y");
            }
        } else {
            panic!("Expected MoveFloatingWindow action");
        }
    }

    #[test]
    fn test_overview_actions() {
        let (lua, actions) = create_test_env();

        lua.load("niri.action:toggle_overview()").exec().unwrap();
        lua.load("niri.action:open_overview()").exec().unwrap();
        lua.load("niri.action:close_overview()").exec().unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 3);
        assert!(matches!(actions[0], Action::ToggleOverview {}));
        assert!(matches!(actions[1], Action::OpenOverview {}));
        assert!(matches!(actions[2], Action::CloseOverview {}));
    }

    #[test]
    fn test_close_window_with_id() {
        let (lua, actions) = create_test_env();

        lua.load("niri.action:close_window(42)").exec().unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 1);
        if let Action::CloseWindow { id } = &actions[0] {
            assert_eq!(*id, Some(42));
        } else {
            panic!("Expected CloseWindow action");
        }
    }

    #[test]
    fn test_multiple_actions_sequence() {
        let (lua, actions) = create_test_env();

        lua.load(
            r#"
            niri.action:focus_column_left()
            niri.action:focus_column_right()
            niri.action:move_column_left()
        "#,
        )
        .exec()
        .unwrap();

        let actions = actions.lock().unwrap();
        assert_eq!(actions.len(), 3);
        assert!(matches!(actions[0], Action::FocusColumnLeft {}));
        assert!(matches!(actions[1], Action::FocusColumnRight {}));
        assert!(matches!(actions[2], Action::MoveColumnLeft {}));
    }

    // ============================================================
    // Snapshot Tests
    // ============================================================

    #[test]
    fn snapshot_action_spawn_with_args() {
        let action = Action::Spawn {
            command: vec!["firefox".into(), "--new-window".into(), "https://example.com".into()],
        };
        insta::assert_debug_snapshot!("action_proxy_spawn_with_args", action);
    }

    #[test]
    fn snapshot_action_focus_workspace_index() {
        let action = Action::FocusWorkspace {
            reference: WorkspaceReferenceArg::Index(3),
        };
        insta::assert_debug_snapshot!("action_proxy_focus_workspace_index", action);
    }

    #[test]
    fn snapshot_action_focus_workspace_name() {
        let action = Action::FocusWorkspace {
            reference: WorkspaceReferenceArg::Name("development".into()),
        };
        insta::assert_debug_snapshot!("action_proxy_focus_workspace_name", action);
    }

    #[test]
    fn snapshot_action_set_column_width_fixed() {
        let action = Action::SetColumnWidth {
            change: SizeChange::SetFixed(800),
        };
        insta::assert_debug_snapshot!("action_proxy_set_column_width_fixed", action);
    }

    #[test]
    fn snapshot_action_set_column_width_proportion() {
        let action = Action::SetColumnWidth {
            change: SizeChange::SetProportion(0.5),
        };
        insta::assert_debug_snapshot!("action_proxy_set_column_width_proportion", action);
    }

    #[test]
    fn snapshot_action_set_column_width_adjust() {
        let action = Action::SetColumnWidth {
            change: SizeChange::AdjustProportion(0.1),
        };
        insta::assert_debug_snapshot!("action_proxy_set_column_width_adjust", action);
    }

    #[test]
    fn snapshot_action_move_floating_window_complex() {
        let action = Action::MoveFloatingWindow {
            id: Some(12345),
            x: PositionChange::AdjustFixed(50.0),
            y: PositionChange::AdjustProportion(-0.15),
        };
        insta::assert_debug_snapshot!("action_proxy_move_floating_window_complex", action);
    }

    #[test]
    fn snapshot_action_move_window_to_workspace() {
        let action = Action::MoveWindowToWorkspace {
            window_id: Some(9876),
            reference: WorkspaceReferenceArg::Name("browser".into()),
            focus: false,
        };
        insta::assert_debug_snapshot!("action_proxy_move_window_to_workspace", action);
    }
}
