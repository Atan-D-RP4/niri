//! Collection proxies for Lua config API
//!
//! Provides CRUD operations for collection-type configuration:
//! - outputs, binds, window_rules, workspaces, environment, layer_rules

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mlua::prelude::*;
use niri_config::binds::{Bind, Key};
use niri_config::output::{Mode, Output, Position, Vrr};
use niri_config::Config;
use niri_ipc::{ConfiguredMode, SizeChange, Transform};

use crate::config_dirty::ConfigDirtyFlags;
use crate::extractors::*;

pub trait CollectionProxyBase<T: Clone> {
    fn config(&self) -> Arc<Mutex<Config>>;

    fn dirty(&self) -> Arc<Mutex<ConfigDirtyFlags>>;

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<T>;

    fn with_config<R>(&self, f: impl FnOnce(&Config) -> R) -> R {
        let config_handle = self.config();
        let config = config_handle.lock().unwrap();
        f(&config)
    }

    fn with_dirty_config<R, E: From<LuaError>>(
        &self,
        f: impl FnOnce(&mut Config, &mut ConfigDirtyFlags) -> Result<R, E>,
    ) -> Result<R, E> {
        let config_handle = self.config();
        let dirty_handle = self.dirty();
        let mut config = config_handle.lock().unwrap();
        let mut dirty = dirty_handle.lock().unwrap();
        f(&mut config, &mut dirty)
    }

    fn list(&self) -> Vec<T> {
        self.with_config(|config| self.collection(config).clone())
    }

    fn len(&self) -> usize {
        self.with_config(|config| self.collection(config).len())
    }

    fn is_empty(&self) -> bool {
        self.with_config(|config| self.collection(config).is_empty())
    }
}

// ============================================================================
// OutputsCollection - CRUD for output configurations
// ============================================================================

/// Proxy for `niri.config.outputs` collection
#[derive(Clone)]
pub struct OutputsCollection {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl CollectionProxyBase<Output> for OutputsCollection {
    fn config(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    fn dirty(&self) -> Arc<Mutex<ConfigDirtyFlags>> {
        self.dirty.clone()
    }

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<Output> {
        &config.outputs.0
    }
}

impl LuaUserData for OutputsCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| -> LuaResult<()> {
            this.with_dirty_config(|config, dirty| {
                match &value {
                    LuaValue::Table(tbl) => {
                        if tbl.contains_key(1)? {
                            for pair in tbl.clone().pairs::<i64, LuaTable>() {
                                let (_, output_tbl) = pair?;
                                let output = extract_output(lua, &output_tbl)?;
                                config.outputs.0.push(output);
                            }
                        } else {
                            let output = extract_output(lua, tbl)?;
                            config.outputs.0.push(output);
                        }
                        dirty.outputs = true;
                        Ok(())
                    }
                    _ => Err(LuaError::external("outputs:add() expects a table")),
                }
            })
        });

        methods.add_method("list", |lua, this, ()| -> LuaResult<LuaTable> {
            this.with_config(|config| {
                let result = lua.create_table()?;

                for (i, output) in config.outputs.0.iter().enumerate() {
                    let tbl = lua.create_table()?;
                    tbl.set("name", output.name.clone())?;
                    if let Some(ref mode) = output.mode {
                        tbl.set(
                            "mode",
                            format!(
                                "{}x{}{}",
                                mode.mode.width,
                                mode.mode.height,
                                mode.mode
                                    .refresh
                                    .map(|r| format!("@{}", r))
                                    .unwrap_or_default()
                            ),
                        )?;
                    }
                    if let Some(ref scale) = output.scale {
                        tbl.set("scale", scale.0)?;
                    }
                    result.set(i + 1, tbl)?;
                }

                Ok(result)
            })
        });

        methods.add_method("get", |lua, this, name: String| -> LuaResult<LuaValue> {
            this.with_config(|config| {
                for output in &config.outputs.0 {
                    if output.name == name {
                        let tbl = lua.create_table()?;
                        tbl.set("name", output.name.clone())?;
                        tbl.set("off", output.off)?;
                        if let Some(ref scale) = output.scale {
                            tbl.set("scale", scale.0)?;
                        }
                        tbl.set("focus_at_startup", output.focus_at_startup)?;
                        return Ok(LuaValue::Table(tbl));
                    }
                }

                Ok(LuaValue::Nil)
            })
        });

        methods.add_method("remove", |_, this, name: String| -> LuaResult<()> {
            this.with_dirty_config(|config, dirty| {
                let len_before = config.outputs.0.len();
                config.outputs.0.retain(|o| o.name != name);

                if config.outputs.0.len() < len_before {
                    dirty.outputs = true;
                }

                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| -> LuaResult<()> {
            this.with_dirty_config(|config, dirty| {
                if !config.outputs.0.is_empty() {
                    config.outputs.0.clear();
                    dirty.outputs = true;
                }

                Ok(())
            })
        });
    }
}

fn extract_output(_lua: &Lua, tbl: &LuaTable) -> LuaResult<Output> {
    use niri_config::FloatOrInt;

    let name = extract_string_opt(tbl, "name")?
        .ok_or_else(|| LuaError::external("output requires 'name' field"))?;

    let mode = if let Some(mode_str) = extract_string_opt(tbl, "mode")? {
        Some(parse_mode_string(&mode_str)?)
    } else {
        None
    };

    let scale = extract_float_opt(tbl, "scale")?.map(FloatOrInt);

    let transform = if let Some(t_str) = extract_string_opt(tbl, "transform")? {
        parse_transform(&t_str)?
    } else {
        Transform::Normal
    };

    let position = if let Some(pos_tbl) = extract_table_opt(tbl, "position")? {
        let x = extract_int_opt(&pos_tbl, "x")?
            .map(|v| v as i32)
            .unwrap_or(0);
        let y = extract_int_opt(&pos_tbl, "y")?
            .map(|v| v as i32)
            .unwrap_or(0);
        Some(Position { x, y })
    } else {
        None
    };

    let variable_refresh_rate =
        if let Some(vrr_str) = extract_string_opt(tbl, "variable_refresh_rate")? {
            match vrr_str.as_str() {
                "on-demand" | "on_demand" => Some(Vrr { on_demand: true }),
                "always" => Some(Vrr { on_demand: false }),
                _ => None,
            }
        } else if let Some(vrr_bool) = extract_bool_opt(tbl, "variable_refresh_rate")? {
            if vrr_bool {
                Some(Vrr { on_demand: true })
            } else {
                None
            }
        } else {
            None
        };

    let off = extract_bool_opt(tbl, "off")?.unwrap_or(false);
    let focus_at_startup = extract_bool_opt(tbl, "focus_at_startup")?.unwrap_or(false);

    Ok(Output {
        name,
        mode,
        scale,
        transform,
        position,
        variable_refresh_rate,
        off,
        focus_at_startup,
        ..Default::default()
    })
}

fn parse_mode_string(mode_str: &str) -> LuaResult<Mode> {
    let (resolution, refresh) = if let Some(at_pos) = mode_str.find('@') {
        let (res, rate) = mode_str.split_at(at_pos);
        let rate_str = &rate[1..];
        let refresh: f64 = rate_str
            .parse()
            .map_err(|_| LuaError::external(format!("Invalid refresh rate: {}", rate_str)))?;
        (res, Some(refresh))
    } else {
        (mode_str, None)
    };

    let parts: Vec<&str> = resolution.split('x').collect();
    if parts.len() != 2 {
        return Err(LuaError::external(format!(
            "Invalid mode format: {}. Expected WIDTHxHEIGHT[@REFRESH]",
            mode_str
        )));
    }

    let width: u16 = parts[0]
        .parse()
        .map_err(|_| LuaError::external(format!("Invalid width: {}", parts[0])))?;
    let height: u16 = parts[1]
        .parse()
        .map_err(|_| LuaError::external(format!("Invalid height: {}", parts[1])))?;

    Ok(Mode {
        custom: false,
        mode: ConfiguredMode {
            width,
            height,
            refresh,
        },
    })
}

fn parse_transform(s: &str) -> LuaResult<Transform> {
    match s.to_lowercase().as_str() {
        "normal" | "0" => Ok(Transform::Normal),
        "90" => Ok(Transform::_90),
        "180" => Ok(Transform::_180),
        "270" => Ok(Transform::_270),
        "flipped" => Ok(Transform::Flipped),
        "flipped-90" => Ok(Transform::Flipped90),
        "flipped-180" => Ok(Transform::Flipped180),
        "flipped-270" => Ok(Transform::Flipped270),
        _ => Err(LuaError::external(format!("Unknown transform: {}", s))),
    }
}

// ============================================================================
// BindsCollection - CRUD for keybindings
// ============================================================================

#[derive(Clone)]
pub struct BindsCollection {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl CollectionProxyBase<Bind> for BindsCollection {
    fn config(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    fn dirty(&self) -> Arc<Mutex<ConfigDirtyFlags>> {
        self.dirty.clone()
    }

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<Bind> {
        &config.binds.0
    }
}

impl LuaUserData for BindsCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| -> LuaResult<()> {
            this.with_dirty_config(|config, dirty| {
                match &value {
                    LuaValue::Table(tbl) => {
                        if tbl.contains_key(1)? {
                            for pair in tbl.clone().pairs::<i64, LuaTable>() {
                                let (_, output_tbl) = pair?;
                                let output = extract_output(lua, &output_tbl)?;
                                config.outputs.0.push(output);
                            }
                        } else {
                            let output = extract_output(lua, tbl)?;
                            config.outputs.0.push(output);
                        }
                        dirty.outputs = true;
                        Ok(())
                    }
                    _ => Err(LuaError::external("outputs:add() expects a table")),
                }
            })
        });


        methods.add_method("list", |lua, this, ()| {
            this.with_config(|config| {
                let result = lua.create_table()?;

                for (i, bind) in config.binds.0.iter().enumerate() {
                    let tbl = lua.create_table()?;
                    tbl.set("key", format!("{:?}", bind.key))?;
                    tbl.set("action", format!("{:?}", bind.action))?;
                    tbl.set("repeat", bind.repeat)?;
                    if let Some(cooldown) = bind.cooldown {
                        tbl.set("cooldown_ms", cooldown.as_millis() as u64)?;
                    }
                    result.set(i + 1, tbl)?;
                }

                Ok(result)
            })
        });

        methods.add_method("remove", |_, this, key_str: String| {
            this.with_dirty_config(|config, dirty| {
                let key: Key = key_str
                    .parse()
                    .map_err(|e| LuaError::external(format!("Invalid key: {}", e)))?;

                let len_before = config.binds.0.len();
                config.binds.0.retain(|b| b.key != key);

                if config.binds.0.len() < len_before {
                    dirty.binds = true;
                }

                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config, dirty| {
                if !config.binds.0.is_empty() {
                    config.binds.0.clear();
                    dirty.binds = true;
                }

                Ok(())
            })
        });
    }
}

fn extract_bind(_lua: &Lua, tbl: &LuaTable) -> LuaResult<Bind> {
    let key_str = extract_string_opt(tbl, "key")?
        .ok_or_else(|| LuaError::external("bind requires 'key' field"))?;

    let key: Key = key_str
        .parse()
        .map_err(|e| LuaError::external(format!("Invalid key '{}': {}", key_str, e)))?;

    let action_str = extract_string_opt(tbl, "action")?
        .ok_or_else(|| LuaError::external("bind requires 'action' field"))?;

    let args: Option<Vec<String>> = if let Some(args_tbl) = extract_table_opt(tbl, "args")? {
        let mut args_vec = Vec::new();
        for pair in args_tbl.pairs::<i64, String>() {
            let (_, arg) = pair?;
            args_vec.push(arg);
        }
        Some(args_vec)
    } else {
        None
    };

    let action = parse_action(&action_str, args.as_deref())?;

    let repeat = extract_bool_opt(tbl, "repeat")?.unwrap_or(false);
    let cooldown = extract_int_opt(tbl, "cooldown_ms")?.map(|v| Duration::from_millis(v as u64));
    let allow_when_locked = extract_bool_opt(tbl, "allow_when_locked")?.unwrap_or(false);
    let allow_inhibiting = extract_bool_opt(tbl, "allow_inhibiting")?.unwrap_or(true);

    Ok(Bind {
        key,
        action,
        repeat,
        cooldown,
        allow_when_locked,
        allow_inhibiting,
        hotkey_overlay_title: None,
    })
}

/// Parse a bind from string format like "Mod+Return spawn alacritty"
/// Format: <key> <action> [args...]
fn parse_bind_string(s: &str) -> LuaResult<Bind> {
    let parts: Vec<&str> = s.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(LuaError::external(
            "bind string requires at least '<key> <action>'",
        ));
    }

    let key_str = parts[0];
    let action_str = parts[1];
    let args: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();

    let key: Key = key_str
        .parse()
        .map_err(|e| LuaError::external(format!("Invalid key '{}': {}", key_str, e)))?;

    let args_opt = if args.is_empty() {
        None
    } else {
        Some(&args[..])
    };
    let action = parse_action(action_str, args_opt)?;

    Ok(Bind {
        key,
        action,
        repeat: false,
        cooldown: None,
        allow_when_locked: false,
        allow_inhibiting: true,
        hotkey_overlay_title: None,
    })
}

fn parse_action(
    action_str: &str,
    args: Option<&[String]>,
) -> LuaResult<niri_config::binds::Action> {
    use niri_config::binds::Action;

    let action_normalized = action_str.replace('-', "_");

    match action_normalized.as_str() {
        // Simple unit variants
        "quit" => Ok(Action::Quit(false)),
        "power_off_monitors" => Ok(Action::PowerOffMonitors),
        "power_on_monitors" => Ok(Action::PowerOnMonitors),
        "suspend" => Ok(Action::Suspend),
        "toggle_debug_tint" => Ok(Action::ToggleDebugTint),
        "screenshot" => Ok(Action::Screenshot(true, None)),
        "screenshot_screen" => Ok(Action::ScreenshotScreen(true, true, None)),
        "screenshot_window" => Ok(Action::ScreenshotWindow(true, None)),
        "close_window" => Ok(Action::CloseWindow),
        "fullscreen_window" => Ok(Action::FullscreenWindow),
        "focus_column_left" => Ok(Action::FocusColumnLeft),
        "focus_column_right" => Ok(Action::FocusColumnRight),
        "focus_column_first" => Ok(Action::FocusColumnFirst),
        "focus_column_last" => Ok(Action::FocusColumnLast),
        "focus_column_right_or_first" => Ok(Action::FocusColumnRightOrFirst),
        "focus_column_left_or_last" => Ok(Action::FocusColumnLeftOrLast),
        "focus_window_up" => Ok(Action::FocusWindowUp),
        "focus_window_down" => Ok(Action::FocusWindowDown),
        "focus_window_up_or_column_left" => Ok(Action::FocusWindowUpOrColumnLeft),
        "focus_window_up_or_column_right" => Ok(Action::FocusWindowUpOrColumnRight),
        "focus_window_down_or_column_left" => Ok(Action::FocusWindowDownOrColumnLeft),
        "focus_window_down_or_column_right" => Ok(Action::FocusWindowDownOrColumnRight),
        "focus_window_or_workspace_up" => Ok(Action::FocusWindowOrWorkspaceUp),
        "focus_window_or_workspace_down" => Ok(Action::FocusWindowOrWorkspaceDown),
        "move_column_left" => Ok(Action::MoveColumnLeft),
        "move_column_right" => Ok(Action::MoveColumnRight),
        "move_column_to_first" => Ok(Action::MoveColumnToFirst),
        "move_column_to_last" => Ok(Action::MoveColumnToLast),
        "move_column_left_or_to_monitor_left" => Ok(Action::MoveColumnLeftOrToMonitorLeft),
        "move_column_right_or_to_monitor_right" => Ok(Action::MoveColumnRightOrToMonitorRight),
        "move_window_up" => Ok(Action::MoveWindowUp),
        "move_window_down" => Ok(Action::MoveWindowDown),
        "move_window_up_or_to_workspace_up" => Ok(Action::MoveWindowUpOrToWorkspaceUp),
        "move_window_down_or_to_workspace_down" => Ok(Action::MoveWindowDownOrToWorkspaceDown),
        "consume_or_expel_window_left" => Ok(Action::ConsumeOrExpelWindowLeft),
        "consume_or_expel_window_right" => Ok(Action::ConsumeOrExpelWindowRight),
        "consume_window_into_column" => Ok(Action::ConsumeWindowIntoColumn),
        "expel_window_from_column" => Ok(Action::ExpelWindowFromColumn),
        "center_column" => Ok(Action::CenterColumn),
        "center_visible_columns" => Ok(Action::CenterVisibleColumns),
        "focus_workspace_up" => Ok(Action::FocusWorkspaceUp),
        "focus_workspace_down" => Ok(Action::FocusWorkspaceDown),
        "focus_workspace_previous" => Ok(Action::FocusWorkspacePrevious),
        "move_column_to_workspace_up" => Ok(Action::MoveColumnToWorkspaceUp(true)),
        "move_column_to_workspace_down" => Ok(Action::MoveColumnToWorkspaceDown(true)),
        "move_workspace_up" => Ok(Action::MoveWorkspaceUp),
        "move_workspace_down" => Ok(Action::MoveWorkspaceDown),
        "focus_monitor_left" => Ok(Action::FocusMonitorLeft),
        "focus_monitor_right" => Ok(Action::FocusMonitorRight),
        "focus_monitor_up" => Ok(Action::FocusMonitorUp),
        "focus_monitor_down" => Ok(Action::FocusMonitorDown),
        "focus_monitor_previous" => Ok(Action::FocusMonitorPrevious),
        "move_column_to_monitor_left" => Ok(Action::MoveColumnToMonitorLeft),
        "move_column_to_monitor_right" => Ok(Action::MoveColumnToMonitorRight),
        "move_column_to_monitor_up" => Ok(Action::MoveColumnToMonitorUp),
        "move_column_to_monitor_down" => Ok(Action::MoveColumnToMonitorDown),
        "move_workspace_to_monitor_left" => Ok(Action::MoveWorkspaceToMonitorLeft),
        "move_workspace_to_monitor_right" => Ok(Action::MoveWorkspaceToMonitorRight),
        "move_workspace_to_monitor_up" => Ok(Action::MoveWorkspaceToMonitorUp),
        "move_workspace_to_monitor_down" => Ok(Action::MoveWorkspaceToMonitorDown),
        "switch_preset_column_width" => Ok(Action::SwitchPresetColumnWidth),
        "switch_preset_window_height" => Ok(Action::SwitchPresetWindowHeight),
        "maximize_column" => Ok(Action::MaximizeColumn),
        "expand_column_to_available_width" => Ok(Action::ExpandColumnToAvailableWidth),
        "reset_window_height" => Ok(Action::ResetWindowHeight),
        "toggle_window_floating" => Ok(Action::ToggleWindowFloating),
        "switch_focus_between_floating_and_tiling" => {
            Ok(Action::SwitchFocusBetweenFloatingAndTiling)
        }
        "toggle_column_tabbed_display" => Ok(Action::ToggleColumnTabbedDisplay),
        "toggle_overview" => Ok(Action::ToggleOverview),
        "show_hotkey_overlay" => Ok(Action::ShowHotkeyOverlay),
        "toggle_keyboard_shortcuts_inhibit" => Ok(Action::ToggleKeyboardShortcutsInhibit),
        "switch_layout" => {
            let direction = args
                .and_then(|a| a.first())
                .map(|s| s.as_str())
                .unwrap_or("next");
            Ok(Action::SwitchLayout(parse_layout_switch_target(direction)?))
        }

        // Actions with args
        "spawn" => {
            let command = args
                .ok_or_else(|| LuaError::external("spawn requires args"))?
                .to_vec();
            Ok(Action::Spawn(command))
        }
        "spawn_sh" => {
            let sh = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("spawn_sh requires args"))?
                .clone();
            Ok(Action::SpawnSh(sh))
        }
        "focus_workspace" => {
            let ws = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("focus_workspace requires args"))?;
            Ok(Action::FocusWorkspace(parse_workspace_reference(ws)?))
        }
        "move_column_to_workspace" => {
            let ws = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("move_column_to_workspace requires args"))?;
            Ok(Action::MoveColumnToWorkspace(
                parse_workspace_reference(ws)?,
                true,
            ))
        }
        "move_window_to_workspace" => {
            let ws = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("move_window_to_workspace requires args"))?;
            Ok(Action::MoveWindowToWorkspace(
                parse_workspace_reference(ws)?,
                true,
            ))
        }
        "set_column_width" => {
            let change = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("set_column_width requires args"))?;
            Ok(Action::SetColumnWidth(parse_size_change(change)?))
        }
        "set_window_height" => {
            let change = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("set_window_height requires args"))?;
            Ok(Action::SetWindowHeight(parse_size_change(change)?))
        }
        "focus_monitor" => {
            let monitor = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("focus_monitor requires args"))?
                .clone();
            Ok(Action::FocusMonitor(monitor))
        }
        "move_column_to_monitor" => {
            let monitor = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("move_column_to_monitor requires args"))?
                .clone();
            Ok(Action::MoveColumnToMonitor(monitor))
        }
        "move_workspace_to_monitor" => {
            let monitor = args
                .and_then(|a| a.first())
                .ok_or_else(|| LuaError::external("move_workspace_to_monitor requires args"))?
                .clone();
            Ok(Action::MoveWorkspaceToMonitor(monitor))
        }

        _ => Err(LuaError::external(format!(
            "Unknown action: {}",
            action_str
        ))),
    }
}

fn parse_workspace_reference(s: &str) -> LuaResult<niri_config::binds::WorkspaceReference> {
    use niri_config::binds::WorkspaceReference;

    if let Ok(idx) = s.parse::<i32>() {
        if idx > 0 {
            return Ok(WorkspaceReference::Index(idx as u8));
        }
    }

    Ok(WorkspaceReference::Name(s.to_string()))
}

fn parse_size_change(s: &str) -> LuaResult<SizeChange> {
    let s = s.trim();

    if let Some(rest) = s.strip_prefix('+') {
        let val = parse_proportion_or_fixed(rest)?;
        Ok(SizeChange::AdjustProportion(val))
    } else if let Some(rest) = s.strip_prefix('-') {
        let val = parse_proportion_or_fixed(rest)?;
        Ok(SizeChange::AdjustProportion(-val))
    } else {
        let val = parse_proportion_or_fixed(s)?;
        Ok(SizeChange::SetProportion(val))
    }
}

fn parse_proportion_or_fixed(s: &str) -> LuaResult<f64> {
    let s = s.trim();

    if let Some(pct) = s.strip_suffix('%') {
        let val: f64 = pct
            .trim()
            .parse()
            .map_err(|_| LuaError::external(format!("Invalid percentage: {}", s)))?;
        Ok(val / 100.0)
    } else if let Some(px) = s.strip_suffix("px") {
        px.trim()
            .parse()
            .map_err(|_| LuaError::external(format!("Invalid pixel value: {}", s)))
    } else {
        s.parse()
            .map_err(|_| LuaError::external(format!("Invalid value: {}", s)))
    }
}

fn parse_layout_switch_target(s: &str) -> LuaResult<niri_ipc::LayoutSwitchTarget> {
    use niri_ipc::LayoutSwitchTarget;

    match s.to_lowercase().as_str() {
        "next" => Ok(LayoutSwitchTarget::Next),
        "prev" | "previous" => Ok(LayoutSwitchTarget::Prev),
        _ => {
            if let Ok(idx) = s.parse::<u8>() {
                Ok(LayoutSwitchTarget::Index(idx))
            } else {
                Err(LuaError::external(format!(
                    "Invalid layout switch target: {}",
                    s
                )))
            }
        }
    }
}

// ============================================================================
// WindowRulesCollection - CRUD for window rules
// ============================================================================

#[derive(Clone)]
pub struct WindowRulesCollection {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl CollectionProxyBase<niri_config::WindowRule> for WindowRulesCollection {
    fn config(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    fn dirty(&self) -> Arc<Mutex<ConfigDirtyFlags>> {
        self.dirty.clone()
    }

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<niri_config::WindowRule> {
        &config.window_rules
    }
}

impl LuaUserData for WindowRulesCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config, dirty| {
                match &value {
                    LuaValue::Table(tbl) => {
                        if tbl.contains_key(1)? {
                            for pair in tbl.clone().pairs::<i64, LuaTable>() {
                                let (_, rule_tbl) = pair?;
                                let rule = extract_window_rule(lua, &rule_tbl)?;
                                config.window_rules.push(rule);
                            }
                        } else {
                            let rule = extract_window_rule(lua, tbl)?;
                            config.window_rules.push(rule);
                        }
                        dirty.window_rules = true;
                        Ok(())
                    }
                    _ => Err(LuaError::external("window_rules:add() expects a table")),
                }
            })
        });

        methods.add_method("list", |lua, this, ()| {
            this.with_config(|config| {
                let result = lua.create_table()?;

                for (i, rule) in config.window_rules.iter().enumerate() {
                    let tbl = lua.create_table()?;
                    if !rule.matches.is_empty() {
                        let matches_tbl = lua.create_table()?;
                        for (j, m) in rule.matches.iter().enumerate() {
                            let match_tbl = lua.create_table()?;
                            if let Some(ref app_id) = m.app_id {
                                match_tbl.set("app_id", app_id.0.to_string())?;
                            }
                            if let Some(ref title) = m.title {
                                match_tbl.set("title", title.0.to_string())?;
                            }
                            matches_tbl.set(j + 1, match_tbl)?;
                        }
                        tbl.set("matches", matches_tbl)?;
                    }
                    result.set(i + 1, tbl)?;
                }

                Ok(result)
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config, dirty| {
                if !config.window_rules.is_empty() {
                    config.window_rules.clear();
                    dirty.window_rules = true;
                }

                Ok(())
            })
        });
    }
}

fn extract_window_rule(_lua: &Lua, tbl: &LuaTable) -> LuaResult<niri_config::WindowRule> {
    use niri_config::window_rule::WindowRule;

    let mut rule = WindowRule::default();

    if let Some(match_tbl) = extract_table_opt(tbl, "match")? {
        let m = extract_match(&match_tbl)?;
        rule.matches.push(m);
    }

    if let Some(exclude_tbl) = extract_table_opt(tbl, "exclude")? {
        let m = extract_match(&exclude_tbl)?;
        rule.excludes.push(m);
    }

    if let Some(default_width) = extract_table_opt(tbl, "default_column_width")? {
        if default_width.is_empty() {
            rule.default_column_width = Some(niri_config::DefaultPresetSize(None));
        }
    }

    if let Some(open_floating) = extract_bool_opt(tbl, "open_floating")? {
        rule.open_floating = Some(open_floating);
    }

    if let Some(open_fullscreen) = extract_bool_opt(tbl, "open_fullscreen")? {
        rule.open_fullscreen = Some(open_fullscreen);
    }

    if let Some(open_maximized) = extract_bool_opt(tbl, "open_maximized")? {
        rule.open_maximized = Some(open_maximized);
    }

    Ok(rule)
}

fn extract_match(tbl: &LuaTable) -> LuaResult<niri_config::window_rule::Match> {
    use niri_config::window_rule::Match;

    let mut m = Match::default();

    if let Some(app_id) = extract_string_opt(tbl, "app_id")? {
        m.app_id = Some(niri_config::utils::RegexEq(
            regex::Regex::new(&app_id)
                .map_err(|e| LuaError::external(format!("Invalid app_id regex: {}", e)))?,
        ));
    }

    if let Some(title) = extract_string_opt(tbl, "title")? {
        m.title = Some(niri_config::utils::RegexEq(
            regex::Regex::new(&title)
                .map_err(|e| LuaError::external(format!("Invalid title regex: {}", e)))?,
        ));
    }

    if let Some(at_startup) = extract_bool_opt(tbl, "at_startup")? {
        m.at_startup = Some(at_startup);
    }

    if let Some(is_floating) = extract_bool_opt(tbl, "is_floating")? {
        m.is_floating = Some(is_floating);
    }

    Ok(m)
}

// ============================================================================
// WorkspacesCollection - CRUD for named workspaces
// ============================================================================

#[derive(Clone)]
pub struct WorkspacesCollection {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl CollectionProxyBase<niri_config::Workspace> for WorkspacesCollection {
    fn config(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    fn dirty(&self) -> Arc<Mutex<ConfigDirtyFlags>> {
        self.dirty.clone()
    }

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<niri_config::Workspace> {
        &config.workspaces
    }
}

impl LuaUserData for WorkspacesCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config, dirty| {
                match &value {
                    LuaValue::Table(tbl) => {
                        if tbl.contains_key(1)? {
                            for pair in tbl.clone().pairs::<i64, LuaTable>() {
                                let (_, ws_tbl) = pair?;
                                let ws = extract_workspace(lua, &ws_tbl)?;
                                config.workspaces.push(ws);
                            }
                        } else {
                            let ws = extract_workspace(lua, tbl)?;
                            config.workspaces.push(ws);
                        }
                        dirty.workspaces = true;
                        Ok(())
                    }
                    _ => Err(LuaError::external("workspaces:add() expects a table")),
                }
            })
        });

        methods.add_method("list", |lua, this, ()| {
            this.with_config(|config| {
                let result = lua.create_table()?;

                for (i, ws) in config.workspaces.iter().enumerate() {
                    let tbl = lua.create_table()?;
                    tbl.set("name", ws.name.0.clone())?;
                    if let Some(ref output) = ws.open_on_output {
                        tbl.set("open_on_output", output.clone())?;
                    }
                    result.set(i + 1, tbl)?;
                }

                Ok(result)
            })
        });

        methods.add_method("get", |lua, this, key: LuaValue| {
            this.with_config(|config| {
                // Accept both index (number) and name (string)
                match key {
                    LuaValue::Integer(idx) => {
                        // 1-based indexing for Lua
                        if idx < 1 {
                            return Ok(LuaValue::Nil);
                        }
                        let idx = (idx - 1) as usize;
                        if idx >= config.workspaces.len() {
                            return Ok(LuaValue::Nil);
                        }
                        let ws = &config.workspaces[idx];
                        let tbl = lua.create_table()?;
                        tbl.set("name", ws.name.0.clone())?;
                        if let Some(ref output) = ws.open_on_output {
                            tbl.set("open_on_output", output.clone())?;
                        }
                        Ok(LuaValue::Table(tbl))
                    }
                    LuaValue::String(name) => {
                        let name = name.to_str()?;
                        for ws in &config.workspaces {
                            if name == ws.name.0 {
                                let tbl = lua.create_table()?;
                                tbl.set("name", ws.name.0.clone())?;
                                if let Some(ref output) = ws.open_on_output {
                                    tbl.set("open_on_output", output.clone())?;
                                }
                                return Ok(LuaValue::Table(tbl));
                            }
                        }
                        Ok(LuaValue::Nil)
                    }
                    _ => Err(LuaError::external(
                        "get() expects an index (number) or workspace name (string)",
                    )),
                }
            })
        });

        methods.add_method("remove", |_, this, name: String| {
            this.with_dirty_config(|config, dirty| {
                let len_before = config.workspaces.len();
                config.workspaces.retain(|ws| ws.name.0 != name);

                if config.workspaces.len() < len_before {
                    dirty.workspaces = true;
                }

                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config, dirty| {
                if !config.workspaces.is_empty() {
                    config.workspaces.clear();
                    dirty.workspaces = true;
                }

                Ok(())
            })
        });
    }
}

fn extract_workspace(_lua: &Lua, tbl: &LuaTable) -> LuaResult<niri_config::Workspace> {
    use niri_config::workspace::WorkspaceName;
    use niri_config::Workspace;

    let name = extract_string_opt(tbl, "name")?
        .ok_or_else(|| LuaError::external("workspace requires 'name' field"))?;

    let open_on_output = extract_string_opt(tbl, "open_on_output")?;

    Ok(Workspace {
        name: WorkspaceName(name),
        open_on_output,
        layout: None,
    })
}

// ============================================================================
// EnvironmentCollection - CRUD for environment variables
// ============================================================================

#[derive(Clone)]
pub struct EnvironmentCollection {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl CollectionProxyBase<niri_config::EnvironmentVariable> for EnvironmentCollection {
    fn config(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    fn dirty(&self) -> Arc<Mutex<ConfigDirtyFlags>> {
        self.dirty.clone()
    }

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<niri_config::EnvironmentVariable> {
        &config.environment.0
    }
}

impl LuaUserData for EnvironmentCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config, dirty| {
                match &value {
                    LuaValue::Table(tbl) => {
                        if tbl.contains_key(1)? {
                            for pair in tbl.clone().pairs::<i64, LuaTable>() {
                                let (_, env_tbl) = pair?;
                                let env = extract_environment_variable(lua, &env_tbl)?;
                                config.environment.0.push(env);
                            }
                        } else {
                            let env = extract_environment_variable(lua, tbl)?;
                            config.environment.0.push(env);
                        }
                        dirty.environment = true;
                        Ok(())
                    }
                    _ => Err(LuaError::external("environment:add() expects a table")),
                }
            })
        });

        methods.add_method("list", |lua, this, ()| {
            this.with_config(|config| {
                let result = lua.create_table()?;

                for (i, env) in config.environment.0.iter().enumerate() {
                    let tbl = lua.create_table()?;
                    tbl.set("name", env.name.clone())?;
                    if let Some(ref value) = env.value {
                        tbl.set("value", value.clone())?;
                    }
                    result.set(i + 1, tbl)?;
                }

                Ok(result)
            })
        });

        methods.add_method("get", |lua, this, name: String| {
            this.with_config(|config| {
                for env in &config.environment.0 {
                    if env.name == name {
                        let tbl = lua.create_table()?;
                        tbl.set("name", env.name.clone())?;
                        if let Some(ref value) = env.value {
                            tbl.set("value", value.clone())?;
                        }
                        return Ok(LuaValue::Table(tbl));
                    }
                }

                Ok(LuaValue::Nil)
            })
        });

        methods.add_method("set", |_, this, (name, value): (String, Option<String>)| {
            this.with_dirty_config(|config, dirty| {
                if let Some(env) = config.environment.0.iter_mut().find(|e| e.name == name) {
                    env.value = value;
                } else {
                    config
                        .environment
                        .0
                        .push(niri_config::EnvironmentVariable { name, value });
                }
                dirty.environment = true;

                Ok(())
            })
        });

        methods.add_method("remove", |_, this, name: String| {
            this.with_dirty_config(|config, dirty| {
                let len_before = config.environment.0.len();
                config.environment.0.retain(|e| e.name != name);

                if config.environment.0.len() < len_before {
                    dirty.environment = true;
                }

                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config, dirty| {
                if !config.environment.0.is_empty() {
                    config.environment.0.clear();
                    dirty.environment = true;
                }

                Ok(())
            })
        });
    }
}

fn extract_environment_variable(
    _lua: &Lua,
    tbl: &LuaTable,
) -> LuaResult<niri_config::EnvironmentVariable> {
    let name = extract_string_opt(tbl, "name")?
        .ok_or_else(|| LuaError::external("environment requires 'name' field"))?;

    let value = extract_string_opt(tbl, "value")?;

    Ok(niri_config::EnvironmentVariable { name, value })
}

// ============================================================================
// LayerRulesCollection - CRUD for layer shell rules
// ============================================================================

#[derive(Clone)]
pub struct LayerRulesCollection {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl CollectionProxyBase<niri_config::LayerRule> for LayerRulesCollection {
    fn config(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    fn dirty(&self) -> Arc<Mutex<ConfigDirtyFlags>> {
        self.dirty.clone()
    }

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<niri_config::LayerRule> {
        &config.layer_rules
    }
}

impl LuaUserData for LayerRulesCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config, dirty| {
                match &value {
                    LuaValue::Table(tbl) => {
                        if tbl.contains_key(1)? {
                            for pair in tbl.clone().pairs::<i64, LuaTable>() {
                                let (_, rule_tbl) = pair?;
                                let rule = extract_layer_rule(lua, &rule_tbl)?;
                                config.layer_rules.push(rule);
                            }
                        } else {
                            let rule = extract_layer_rule(lua, tbl)?;
                            config.layer_rules.push(rule);
                        }
                        dirty.layer_rules = true;
                        Ok(())
                    }
                    _ => Err(LuaError::external("layer_rules:add() expects a table")),
                }
            })
        });

        methods.add_method("list", |lua, this, ()| {
            this.with_config(|config| {
                let result = lua.create_table()?;

                for (i, rule) in config.layer_rules.iter().enumerate() {
                    let tbl = lua.create_table()?;
                    if !rule.matches.is_empty() {
                        let matches_tbl = lua.create_table()?;
                        for (j, m) in rule.matches.iter().enumerate() {
                            let match_tbl = lua.create_table()?;
                            if let Some(ref ns) = m.namespace {
                                match_tbl.set("namespace", ns.0.to_string())?;
                            }
                            matches_tbl.set(j + 1, match_tbl)?;
                        }
                        tbl.set("matches", matches_tbl)?;
                    }
                    result.set(i + 1, tbl)?;
                }

                Ok(result)
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config, dirty| {
                if !config.layer_rules.is_empty() {
                    config.layer_rules.clear();
                    dirty.layer_rules = true;
                }

                Ok(())
            })
        });
    }
}

fn extract_layer_rule(_lua: &Lua, tbl: &LuaTable) -> LuaResult<niri_config::LayerRule> {
    use niri_config::layer_rule::LayerRule;

    let mut rule = LayerRule::default();

    if let Some(match_tbl) = extract_table_opt(tbl, "match")? {
        let m = extract_layer_match(&match_tbl)?;
        rule.matches.push(m);
    }

    if let Some(exclude_tbl) = extract_table_opt(tbl, "exclude")? {
        let m = extract_layer_match(&exclude_tbl)?;
        rule.excludes.push(m);
    }

    if let Some(block_str) = extract_string_opt(tbl, "block_out_from")? {
        rule.block_out_from = Some(parse_block_out_from(&block_str)?);
    }

    Ok(rule)
}

fn extract_layer_match(tbl: &LuaTable) -> LuaResult<niri_config::layer_rule::Match> {
    use niri_config::layer_rule::Match;

    let mut m = Match::default();

    if let Some(ns) = extract_string_opt(tbl, "namespace")? {
        m.namespace = Some(niri_config::utils::RegexEq(
            regex::Regex::new(&ns)
                .map_err(|e| LuaError::external(format!("Invalid namespace regex: {}", e)))?,
        ));
    }

    if let Some(at_startup) = extract_bool_opt(tbl, "at_startup")? {
        m.at_startup = Some(at_startup);
    }

    Ok(m)
}

fn parse_block_out_from(s: &str) -> LuaResult<niri_config::BlockOutFrom> {
    use niri_config::BlockOutFrom;

    match s.to_lowercase().as_str() {
        "screencast" => Ok(BlockOutFrom::Screencast),
        "screen_capture" | "screen-capture" => Ok(BlockOutFrom::ScreenCapture),
        _ => Err(LuaError::external(format!(
            "Invalid block_out_from value: {}",
            s
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mode_string() {
        let mode = parse_mode_string("1920x1080").unwrap();
        assert_eq!(mode.mode.width, 1920);
        assert_eq!(mode.mode.height, 1080);
        assert_eq!(mode.mode.refresh, None);

        let mode = parse_mode_string("1920x1080@60").unwrap();
        assert_eq!(mode.mode.width, 1920);
        assert_eq!(mode.mode.height, 1080);
        assert_eq!(mode.mode.refresh, Some(60.0));

        let mode = parse_mode_string("3840x2160@59.94").unwrap();
        assert_eq!(mode.mode.width, 3840);
        assert_eq!(mode.mode.height, 2160);
        assert_eq!(mode.mode.refresh, Some(59.94));
    }

    #[test]
    fn test_parse_transform() {
        assert!(matches!(
            parse_transform("normal").unwrap(),
            Transform::Normal
        ));
        assert!(matches!(parse_transform("90").unwrap(), Transform::_90));
        assert!(matches!(parse_transform("180").unwrap(), Transform::_180));
        assert!(matches!(parse_transform("270").unwrap(), Transform::_270));
        assert!(matches!(
            parse_transform("flipped").unwrap(),
            Transform::Flipped
        ));
    }

    #[test]
    fn test_parse_size_change() {
        let change = parse_size_change("+10%").unwrap();
        assert!(matches!(change, SizeChange::AdjustProportion(v) if (v - 0.1).abs() < 0.001));

        let change = parse_size_change("-10%").unwrap();
        assert!(matches!(change, SizeChange::AdjustProportion(v) if (v + 0.1).abs() < 0.001));

        let change = parse_size_change("50%").unwrap();
        assert!(matches!(change, SizeChange::SetProportion(v) if (v - 0.5).abs() < 0.001));
    }
}
