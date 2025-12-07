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

use log::debug;
use mlua::prelude::*;
use niri_ipc::{Action, LayoutSwitchTarget, PositionChange, SizeChange, WorkspaceReferenceArg};

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
            parse_size_change_str(&s)
        }
        _ => Err(LuaError::external("size change must be a number or string")),
    }
}

fn parse_size_change_str(s: &str) -> LuaResult<SizeChange> {
    let s = s.trim();
    if s.is_empty() {
        return Err(LuaError::external("size change cannot be empty"));
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
                Ok(SizeChange::AdjustProportion(-proportion))
            } else {
                Ok(SizeChange::AdjustProportion(proportion))
            }
        } else {
            Ok(SizeChange::SetProportion(proportion))
        }
    } else {
        let value: i32 = num_str
            .parse()
            .map_err(|_| LuaError::external(format!("invalid size: {}", s)))?;
        if is_relative {
            if s.starts_with('-') {
                Ok(SizeChange::AdjustFixed(-value))
            } else {
                Ok(SizeChange::AdjustFixed(value))
            }
        } else {
            Ok(SizeChange::SetFixed(value))
        }
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
        // System Actions
        // ============================================================

        // quit(skip_confirmation?)
        methods.add_method("quit", |_lua, this, skip_confirmation: Option<bool>| {
            this.execute(Action::Quit {
                skip_confirmation: skip_confirmation.unwrap_or(false),
            })
        });

        // power_off_monitors()
        methods.add_method("power_off_monitors", |_lua, this, ()| {
            this.execute(Action::PowerOffMonitors {})
        });

        // power_on_monitors()
        methods.add_method("power_on_monitors", |_lua, this, ()| {
            this.execute(Action::PowerOnMonitors {})
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

        // load_config_file()
        methods.add_method("load_config_file", |_lua, this, ()| {
            this.execute(Action::LoadConfigFile {})
        });

        // ============================================================
        // Screenshot Actions
        // ============================================================

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

        // ============================================================
        // Window Actions
        // ============================================================

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

        // focus_window_previous()
        methods.add_method("focus_window_previous", |_lua, this, ()| {
            this.execute(Action::FocusWindowPrevious {})
        });

        // toggle_keyboard_shortcuts_inhibit()
        methods.add_method("toggle_keyboard_shortcuts_inhibit", |_lua, this, ()| {
            this.execute(Action::ToggleKeyboardShortcutsInhibit {})
        });

        // ============================================================
        // Column Focus Actions
        // ============================================================

        // focus_column_left()
        methods.add_method("focus_column_left", |_lua, this, ()| {
            this.execute(Action::FocusColumnLeft {})
        });

        // focus_column_right()
        methods.add_method("focus_column_right", |_lua, this, ()| {
            this.execute(Action::FocusColumnRight {})
        });

        // focus_column_first()
        methods.add_method("focus_column_first", |_lua, this, ()| {
            this.execute(Action::FocusColumnFirst {})
        });

        // focus_column_last()
        methods.add_method("focus_column_last", |_lua, this, ()| {
            this.execute(Action::FocusColumnLast {})
        });

        // focus_column_right_or_first()
        methods.add_method("focus_column_right_or_first", |_lua, this, ()| {
            this.execute(Action::FocusColumnRightOrFirst {})
        });

        // focus_column_left_or_last()
        methods.add_method("focus_column_left_or_last", |_lua, this, ()| {
            this.execute(Action::FocusColumnLeftOrLast {})
        });

        // focus_column(index)
        methods.add_method("focus_column", |_lua, this, index: usize| {
            this.execute(Action::FocusColumn { index })
        });

        // ============================================================
        // Window Focus Actions (vertical)
        // ============================================================

        // focus_window_down()
        methods.add_method("focus_window_down", |_lua, this, ()| {
            this.execute(Action::FocusWindowDown {})
        });

        // focus_window_up()
        methods.add_method("focus_window_up", |_lua, this, ()| {
            this.execute(Action::FocusWindowUp {})
        });

        // focus_window_or_monitor_up()
        methods.add_method("focus_window_or_monitor_up", |_lua, this, ()| {
            this.execute(Action::FocusWindowOrMonitorUp {})
        });

        // focus_window_or_monitor_down()
        methods.add_method("focus_window_or_monitor_down", |_lua, this, ()| {
            this.execute(Action::FocusWindowOrMonitorDown {})
        });

        // focus_column_or_monitor_left()
        methods.add_method("focus_column_or_monitor_left", |_lua, this, ()| {
            this.execute(Action::FocusColumnOrMonitorLeft {})
        });

        // focus_column_or_monitor_right()
        methods.add_method("focus_column_or_monitor_right", |_lua, this, ()| {
            this.execute(Action::FocusColumnOrMonitorRight {})
        });

        // focus_window_down_or_column_left()
        methods.add_method("focus_window_down_or_column_left", |_lua, this, ()| {
            this.execute(Action::FocusWindowDownOrColumnLeft {})
        });

        // focus_window_down_or_column_right()
        methods.add_method("focus_window_down_or_column_right", |_lua, this, ()| {
            this.execute(Action::FocusWindowDownOrColumnRight {})
        });

        // focus_window_up_or_column_left()
        methods.add_method("focus_window_up_or_column_left", |_lua, this, ()| {
            this.execute(Action::FocusWindowUpOrColumnLeft {})
        });

        // focus_window_up_or_column_right()
        methods.add_method("focus_window_up_or_column_right", |_lua, this, ()| {
            this.execute(Action::FocusWindowUpOrColumnRight {})
        });

        // focus_window_or_workspace_down()
        methods.add_method("focus_window_or_workspace_down", |_lua, this, ()| {
            this.execute(Action::FocusWindowOrWorkspaceDown {})
        });

        // focus_window_or_workspace_up()
        methods.add_method("focus_window_or_workspace_up", |_lua, this, ()| {
            this.execute(Action::FocusWindowOrWorkspaceUp {})
        });

        // focus_window_top()
        methods.add_method("focus_window_top", |_lua, this, ()| {
            this.execute(Action::FocusWindowTop {})
        });

        // focus_window_bottom()
        methods.add_method("focus_window_bottom", |_lua, this, ()| {
            this.execute(Action::FocusWindowBottom {})
        });

        // focus_window_down_or_top()
        methods.add_method("focus_window_down_or_top", |_lua, this, ()| {
            this.execute(Action::FocusWindowDownOrTop {})
        });

        // focus_window_up_or_bottom()
        methods.add_method("focus_window_up_or_bottom", |_lua, this, ()| {
            this.execute(Action::FocusWindowUpOrBottom {})
        });

        // ============================================================
        // Column Move Actions
        // ============================================================

        // move_column_left()
        methods.add_method("move_column_left", |_lua, this, ()| {
            this.execute(Action::MoveColumnLeft {})
        });

        // move_column_right()
        methods.add_method("move_column_right", |_lua, this, ()| {
            this.execute(Action::MoveColumnRight {})
        });

        // move_column_to_first()
        methods.add_method("move_column_to_first", |_lua, this, ()| {
            this.execute(Action::MoveColumnToFirst {})
        });

        // move_column_to_last()
        methods.add_method("move_column_to_last", |_lua, this, ()| {
            this.execute(Action::MoveColumnToLast {})
        });

        // move_column_left_or_to_monitor_left()
        methods.add_method("move_column_left_or_to_monitor_left", |_lua, this, ()| {
            this.execute(Action::MoveColumnLeftOrToMonitorLeft {})
        });

        // move_column_right_or_to_monitor_right()
        methods.add_method("move_column_right_or_to_monitor_right", |_lua, this, ()| {
            this.execute(Action::MoveColumnRightOrToMonitorRight {})
        });

        // move_column_to_index(index)
        methods.add_method("move_column_to_index", |_lua, this, index: usize| {
            this.execute(Action::MoveColumnToIndex { index })
        });

        // ============================================================
        // Window Move Actions (vertical)
        // ============================================================

        // move_window_down()
        methods.add_method("move_window_down", |_lua, this, ()| {
            this.execute(Action::MoveWindowDown {})
        });

        // move_window_up()
        methods.add_method("move_window_up", |_lua, this, ()| {
            this.execute(Action::MoveWindowUp {})
        });

        // move_window_down_or_to_workspace_down()
        methods.add_method("move_window_down_or_to_workspace_down", |_lua, this, ()| {
            this.execute(Action::MoveWindowDownOrToWorkspaceDown {})
        });

        // move_window_up_or_to_workspace_up()
        methods.add_method("move_window_up_or_to_workspace_up", |_lua, this, ()| {
            this.execute(Action::MoveWindowUpOrToWorkspaceUp {})
        });

        // ============================================================
        // Consume/Expel Window Actions
        // ============================================================

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

        // consume_window_into_column()
        methods.add_method("consume_window_into_column", |_lua, this, ()| {
            this.execute(Action::ConsumeWindowIntoColumn {})
        });

        // expel_window_from_column()
        methods.add_method("expel_window_from_column", |_lua, this, ()| {
            this.execute(Action::ExpelWindowFromColumn {})
        });

        // swap_window_right()
        methods.add_method("swap_window_right", |_lua, this, ()| {
            this.execute(Action::SwapWindowRight {})
        });

        // swap_window_left()
        methods.add_method("swap_window_left", |_lua, this, ()| {
            this.execute(Action::SwapWindowLeft {})
        });

        // ============================================================
        // Column Display Actions
        // ============================================================

        // toggle_column_tabbed_display()
        methods.add_method("toggle_column_tabbed_display", |_lua, this, ()| {
            this.execute(Action::ToggleColumnTabbedDisplay {})
        });

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

        // center_column()
        methods.add_method("center_column", |_lua, this, ()| {
            this.execute(Action::CenterColumn {})
        });

        // center_window(id?)
        methods.add_method("center_window", |_lua, this, id: Option<u64>| {
            this.execute(Action::CenterWindow { id })
        });

        // center_visible_columns()
        methods.add_method("center_visible_columns", |_lua, this, ()| {
            this.execute(Action::CenterVisibleColumns {})
        });

        // ============================================================
        // Workspace Actions
        // ============================================================

        // focus_workspace_down()
        methods.add_method("focus_workspace_down", |_lua, this, ()| {
            this.execute(Action::FocusWorkspaceDown {})
        });

        // focus_workspace_up()
        methods.add_method("focus_workspace_up", |_lua, this, ()| {
            this.execute(Action::FocusWorkspaceUp {})
        });

        // focus_workspace(reference) - index, name, or {id=N}
        methods.add_method("focus_workspace", |_lua, this, reference: LuaValue| {
            let reference = parse_workspace_reference(reference)?;
            this.execute(Action::FocusWorkspace { reference })
        });

        // focus_workspace_previous()
        methods.add_method("focus_workspace_previous", |_lua, this, ()| {
            this.execute(Action::FocusWorkspacePrevious {})
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

        // move_workspace_down()
        methods.add_method("move_workspace_down", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceDown {})
        });

        // move_workspace_up()
        methods.add_method("move_workspace_up", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceUp {})
        });

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

        // ============================================================
        // Monitor Focus Actions
        // ============================================================

        // focus_monitor_left()
        methods.add_method("focus_monitor_left", |_lua, this, ()| {
            this.execute(Action::FocusMonitorLeft {})
        });

        // focus_monitor_right()
        methods.add_method("focus_monitor_right", |_lua, this, ()| {
            this.execute(Action::FocusMonitorRight {})
        });

        // focus_monitor_down()
        methods.add_method("focus_monitor_down", |_lua, this, ()| {
            this.execute(Action::FocusMonitorDown {})
        });

        // focus_monitor_up()
        methods.add_method("focus_monitor_up", |_lua, this, ()| {
            this.execute(Action::FocusMonitorUp {})
        });

        // focus_monitor_previous()
        methods.add_method("focus_monitor_previous", |_lua, this, ()| {
            this.execute(Action::FocusMonitorPrevious {})
        });

        // focus_monitor_next()
        methods.add_method("focus_monitor_next", |_lua, this, ()| {
            this.execute(Action::FocusMonitorNext {})
        });

        // focus_monitor(output)
        methods.add_method("focus_monitor", |_lua, this, output: String| {
            this.execute(Action::FocusMonitor { output })
        });

        // ============================================================
        // Window to Monitor Actions
        // ============================================================

        // move_window_to_monitor_left()
        methods.add_method("move_window_to_monitor_left", |_lua, this, ()| {
            this.execute(Action::MoveWindowToMonitorLeft {})
        });

        // move_window_to_monitor_right()
        methods.add_method("move_window_to_monitor_right", |_lua, this, ()| {
            this.execute(Action::MoveWindowToMonitorRight {})
        });

        // move_window_to_monitor_down()
        methods.add_method("move_window_to_monitor_down", |_lua, this, ()| {
            this.execute(Action::MoveWindowToMonitorDown {})
        });

        // move_window_to_monitor_up()
        methods.add_method("move_window_to_monitor_up", |_lua, this, ()| {
            this.execute(Action::MoveWindowToMonitorUp {})
        });

        // move_window_to_monitor_previous()
        methods.add_method("move_window_to_monitor_previous", |_lua, this, ()| {
            this.execute(Action::MoveWindowToMonitorPrevious {})
        });

        // move_window_to_monitor_next()
        methods.add_method("move_window_to_monitor_next", |_lua, this, ()| {
            this.execute(Action::MoveWindowToMonitorNext {})
        });

        // move_window_to_monitor(output, id?)
        methods.add_method(
            "move_window_to_monitor",
            |_lua, this, (output, id): (String, Option<u64>)| {
                this.execute(Action::MoveWindowToMonitor { id, output })
            },
        );

        // ============================================================
        // Column to Monitor Actions
        // ============================================================

        // move_column_to_monitor_left()
        methods.add_method("move_column_to_monitor_left", |_lua, this, ()| {
            this.execute(Action::MoveColumnToMonitorLeft {})
        });

        // move_column_to_monitor_right()
        methods.add_method("move_column_to_monitor_right", |_lua, this, ()| {
            this.execute(Action::MoveColumnToMonitorRight {})
        });

        // move_column_to_monitor_down()
        methods.add_method("move_column_to_monitor_down", |_lua, this, ()| {
            this.execute(Action::MoveColumnToMonitorDown {})
        });

        // move_column_to_monitor_up()
        methods.add_method("move_column_to_monitor_up", |_lua, this, ()| {
            this.execute(Action::MoveColumnToMonitorUp {})
        });

        // move_column_to_monitor_previous()
        methods.add_method("move_column_to_monitor_previous", |_lua, this, ()| {
            this.execute(Action::MoveColumnToMonitorPrevious {})
        });

        // move_column_to_monitor_next()
        methods.add_method("move_column_to_monitor_next", |_lua, this, ()| {
            this.execute(Action::MoveColumnToMonitorNext {})
        });

        // move_column_to_monitor(output)
        methods.add_method("move_column_to_monitor", |_lua, this, output: String| {
            this.execute(Action::MoveColumnToMonitor { output })
        });

        // ============================================================
        // Size/Width/Height Actions
        // ============================================================

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

        // switch_preset_column_width()
        methods.add_method("switch_preset_column_width", |_lua, this, ()| {
            this.execute(Action::SwitchPresetColumnWidth {})
        });

        // switch_preset_column_width_back()
        methods.add_method("switch_preset_column_width_back", |_lua, this, ()| {
            this.execute(Action::SwitchPresetColumnWidthBack {})
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

        // maximize_column()
        methods.add_method("maximize_column", |_lua, this, ()| {
            this.execute(Action::MaximizeColumn {})
        });

        // maximize_window_to_edges(id?)
        methods.add_method("maximize_window_to_edges", |_lua, this, id: Option<u64>| {
            this.execute(Action::MaximizeWindowToEdges { id })
        });

        // set_column_width(change)
        methods.add_method("set_column_width", |_lua, this, change: LuaValue| {
            let change = parse_size_change(change)?;
            this.execute(Action::SetColumnWidth { change })
        });

        // expand_column_to_available_width()
        methods.add_method("expand_column_to_available_width", |_lua, this, ()| {
            this.execute(Action::ExpandColumnToAvailableWidth {})
        });

        // ============================================================
        // Layout Actions
        // ============================================================

        // switch_layout(layout) - "next", "prev", or index
        methods.add_method("switch_layout", |_lua, this, layout: LuaValue| {
            let layout = parse_layout_switch_target(layout)?;
            this.execute(Action::SwitchLayout { layout })
        });

        // show_hotkey_overlay()
        methods.add_method("show_hotkey_overlay", |_lua, this, ()| {
            this.execute(Action::ShowHotkeyOverlay {})
        });

        // ============================================================
        // Workspace to Monitor Actions
        // ============================================================

        // move_workspace_to_monitor_left()
        methods.add_method("move_workspace_to_monitor_left", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceToMonitorLeft {})
        });

        // move_workspace_to_monitor_right()
        methods.add_method("move_workspace_to_monitor_right", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceToMonitorRight {})
        });

        // move_workspace_to_monitor_down()
        methods.add_method("move_workspace_to_monitor_down", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceToMonitorDown {})
        });

        // move_workspace_to_monitor_up()
        methods.add_method("move_workspace_to_monitor_up", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceToMonitorUp {})
        });

        // move_workspace_to_monitor_previous()
        methods.add_method("move_workspace_to_monitor_previous", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceToMonitorPrevious {})
        });

        // move_workspace_to_monitor_next()
        methods.add_method("move_workspace_to_monitor_next", |_lua, this, ()| {
            this.execute(Action::MoveWorkspaceToMonitorNext {})
        });

        // move_workspace_to_monitor(output, reference?)
        methods.add_method(
            "move_workspace_to_monitor",
            |_lua, this, (output, reference): (String, Option<LuaValue>)| {
                let reference = reference.map(parse_workspace_reference).transpose()?;
                this.execute(Action::MoveWorkspaceToMonitor { output, reference })
            },
        );

        // ============================================================
        // Debug Actions
        // ============================================================

        // toggle_debug_tint()
        methods.add_method("toggle_debug_tint", |_lua, this, ()| {
            this.execute(Action::ToggleDebugTint {})
        });

        // debug_toggle_opaque_regions()
        methods.add_method("debug_toggle_opaque_regions", |_lua, this, ()| {
            this.execute(Action::DebugToggleOpaqueRegions {})
        });

        // debug_toggle_damage()
        methods.add_method("debug_toggle_damage", |_lua, this, ()| {
            this.execute(Action::DebugToggleDamage {})
        });

        // ============================================================
        // Floating Window Actions
        // ============================================================

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

        // focus_floating()
        methods.add_method("focus_floating", |_lua, this, ()| {
            this.execute(Action::FocusFloating {})
        });

        // focus_tiling()
        methods.add_method("focus_tiling", |_lua, this, ()| {
            this.execute(Action::FocusTiling {})
        });

        // switch_focus_between_floating_and_tiling()
        methods.add_method(
            "switch_focus_between_floating_and_tiling",
            |_lua, this, ()| this.execute(Action::SwitchFocusBetweenFloatingAndTiling {}),
        );

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

        // ============================================================
        // Dynamic Cast Actions
        // ============================================================

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

        // clear_dynamic_cast_target()
        methods.add_method("clear_dynamic_cast_target", |_lua, this, ()| {
            this.execute(Action::ClearDynamicCastTarget {})
        });

        // ============================================================
        // Overview Actions
        // ============================================================

        // toggle_overview()
        methods.add_method("toggle_overview", |_lua, this, ()| {
            this.execute(Action::ToggleOverview {})
        });

        // open_overview()
        methods.add_method("open_overview", |_lua, this, ()| {
            this.execute(Action::OpenOverview {})
        });

        // close_overview()
        methods.add_method("close_overview", |_lua, this, ()| {
            this.execute(Action::CloseOverview {})
        });

        // ============================================================
        // Window Urgent Actions
        // ============================================================

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
}
