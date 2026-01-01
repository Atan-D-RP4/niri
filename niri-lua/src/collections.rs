//! Collection proxies for Lua config API
//!
//! Provides CRUD operations for collection-type configuration:
//! - outputs, binds, window_rules, workspaces, environment, layer_rules

use std::time::Duration;

use mlua::prelude::*;
use niri_config::binds::{Action, Bind, Key, WorkspaceReference};
use niri_config::output::{Mode, Output, Position, Vrr};
use niri_config::Config;
use niri_ipc::{
    ColumnDisplay, ConfiguredMode, LayoutSwitchTarget, PositionChange, SizeChange, Transform,
    WorkspaceReferenceArg,
};

use crate::config_state::{ConfigState, DirtyFlag};
use crate::extractors::*;

pub trait CollectionProxyBase<T: Clone> {
    fn state(&self) -> &ConfigState;

    fn collection<'a>(&self, config: &'a Config) -> &'a Vec<T>;

    fn collection_mut<'a>(&self, config: &'a mut Config) -> &'a mut Vec<T>;

    fn dirty_flag(&self) -> DirtyFlag;

    fn with_config<R>(&self, f: impl FnOnce(&Config) -> mlua::Result<R>) -> mlua::Result<R> {
        let config = self
            .state()
            .try_borrow_config()
            .map_err(mlua::Error::external)?;
        f(&config)
    }

    fn with_dirty_config<R>(
        &self,
        f: impl FnOnce(&mut Config) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        let mut config = self.state().borrow_config_mut();
        let result = f(&mut config)?;
        drop(config);
        self.state().mark_dirty(self.dirty_flag());
        Ok(result)
    }

    fn list(&self) -> mlua::Result<Vec<T>> {
        self.with_config(|config| Ok(self.collection(config).clone()))
    }

    fn len(&self) -> mlua::Result<usize> {
        self.with_config(|config| Ok(self.collection(config).len()))
    }

    fn is_empty(&self) -> mlua::Result<bool> {
        self.with_config(|config| Ok(self.collection(config).is_empty()))
    }
}

macro_rules! define_collection {
    ($name:ident, $item:ty, $path:ident $(. $field:tt)*, $dirty:ident) => {
        #[derive(Clone)]
        pub struct $name {
            pub state: ConfigState,
        }

        impl CollectionProxyBase<$item> for $name {
            fn state(&self) -> &ConfigState {
                &self.state
            }

            fn collection<'a>(&self, config: &'a Config) -> &'a Vec<$item> {
                &config.$path $(.$field)*
            }

            fn collection_mut<'a>(&self, config: &'a mut Config) -> &'a mut Vec<$item> {
                &mut config.$path $(.$field)*
            }

            fn dirty_flag(&self) -> DirtyFlag {
                DirtyFlag::$dirty
            }
        }
    };
}

define_collection!(OutputsCollection, Output, outputs.0, Outputs);

impl LuaUserData for OutputsCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| -> LuaResult<()> {
            this.with_dirty_config(|config| match &value {
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

                    Ok(())
                }
                _ => Err(LuaError::external("outputs:add() expects a table")),
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
            this.with_dirty_config(|config| {
                config.outputs.0.retain(|o| o.name != name);
                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| -> LuaResult<()> {
            this.with_dirty_config(|config| {
                if !config.outputs.0.is_empty() {
                    config.outputs.0.clear();
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

define_collection!(BindsCollection, Bind, binds.0, Binds);

impl LuaUserData for BindsCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| -> LuaResult<()> {
            this.with_dirty_config(|config| {
                match &value {
                    LuaValue::Table(tbl) => {
                        // Check if it's an array of bindings or a single binding
                        if tbl.contains_key(1)? {
                            for pair in tbl.clone().pairs::<i64, LuaTable>() {
                                let (_, bind_tbl) = pair?;
                                let bind = extract_bind(lua, &bind_tbl)?;
                                config.binds.0.push(bind);
                            }
                        } else {
                            let bind = extract_bind(lua, tbl)?;
                            config.binds.0.push(bind);
                        }
                        Ok(())
                    }
                    _ => Err(LuaError::external("binds:add() expects a table")),
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
            this.with_dirty_config(|config| {
                let key: Key = key_str
                    .parse()
                    .map_err(|e| LuaError::external(format!("Invalid key: {}", e)))?;

                config.binds.0.retain(|b| b.key != key);

                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config| {
                if !config.binds.0.is_empty() {
                    config.binds.0.clear();
                }

                Ok(())
            })
        });
    }
}

define_collection!(
    WindowRulesCollection,
    niri_config::WindowRule,
    window_rules,
    WindowRules
);

impl LuaUserData for WindowRulesCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config| match &value {
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

                    Ok(())
                }
                _ => Err(LuaError::external("window_rules:add() expects a table")),
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
            this.with_dirty_config(|config| {
                if !config.window_rules.is_empty() {
                    config.window_rules.clear();
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

define_collection!(
    WorkspacesCollection,
    niri_config::Workspace,
    workspaces,
    Workspaces
);

impl LuaUserData for WorkspacesCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config| match &value {
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

                    Ok(())
                }
                _ => Err(LuaError::external("workspaces:add() expects a table")),
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
            this.with_dirty_config(|config| {
                config.workspaces.retain(|ws| ws.name.0 != name);
                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config| {
                if !config.workspaces.is_empty() {
                    config.workspaces.clear();
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

define_collection!(
    EnvironmentCollection,
    niri_config::EnvironmentVariable,
    environment.0,
    Environment
);

impl LuaUserData for EnvironmentCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config| match &value {
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

                    Ok(())
                }
                _ => Err(LuaError::external("environment:add() expects a table")),
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
            this.with_dirty_config(|config| {
                if let Some(env) = config.environment.0.iter_mut().find(|e| e.name == name) {
                    env.value = value;
                } else {
                    config
                        .environment
                        .0
                        .push(niri_config::EnvironmentVariable { name, value });
                }

                Ok(())
            })
        });

        methods.add_method("remove", |_, this, name: String| {
            this.with_dirty_config(|config| {
                config.environment.0.retain(|e| e.name != name);
                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config| {
                if !config.environment.0.is_empty() {
                    config.environment.0.clear();
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

define_collection!(
    LayerRulesCollection,
    niri_config::LayerRule,
    layer_rules,
    LayerRules
);

impl LuaUserData for LayerRulesCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |lua, this, value: LuaValue| {
            this.with_dirty_config(|config| match &value {
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

                    Ok(())
                }
                _ => Err(LuaError::external("layer_rules:add() expects a table")),
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
            this.with_dirty_config(|config| {
                if !config.layer_rules.is_empty() {
                    config.layer_rules.clear();
                }

                Ok(())
            })
        });
    }
}

define_collection!(
    SpawnAtStartupCollection,
    niri_config::SpawnAtStartup,
    spawn_at_startup,
    SpawnAtStartup
);

impl LuaUserData for SpawnAtStartupCollection {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(CollectionProxyBase::len(this)));

        methods.add_method("add", |_, this, value: LuaValue| {
            this.with_dirty_config(|config| match &value {
                LuaValue::Table(tbl) => {
                    if tbl.contains_key(1)? {
                        for pair in tbl.clone().pairs::<i64, LuaValue>() {
                            let (_, item) = pair?;
                            let spawn = extract_spawn_at_startup(&item)?;
                            config.spawn_at_startup.push(spawn);
                        }
                    } else {
                        let spawn = extract_spawn_at_startup_from_table(tbl)?;
                        config.spawn_at_startup.push(spawn);
                    }
                    Ok(())
                }
                LuaValue::String(s) => {
                    let cmd = s.to_str()?.to_string();
                    config
                        .spawn_at_startup
                        .push(niri_config::SpawnAtStartup { command: vec![cmd] });
                    Ok(())
                }
                _ => Err(LuaError::external(
                    "spawn_at_startup:add() expects a table or string",
                )),
            })
        });

        methods.add_method("list", |lua, this, ()| {
            this.with_config(|config| {
                let result = lua.create_table()?;
                for (i, spawn) in config.spawn_at_startup.iter().enumerate() {
                    let tbl = lua.create_table()?;
                    let cmd_tbl = lua.create_table()?;
                    for (j, arg) in spawn.command.iter().enumerate() {
                        cmd_tbl.set(j + 1, arg.as_str())?;
                    }
                    tbl.set("command", cmd_tbl)?;
                    result.set(i + 1, tbl)?;
                }
                Ok(result)
            })
        });

        methods.add_method("get", |lua, this, index: usize| {
            this.with_config(|config| {
                if index == 0 || index > config.spawn_at_startup.len() {
                    return Ok(LuaValue::Nil);
                }
                let spawn = &config.spawn_at_startup[index - 1];
                let tbl = lua.create_table()?;
                let cmd_tbl = lua.create_table()?;
                for (j, arg) in spawn.command.iter().enumerate() {
                    cmd_tbl.set(j + 1, arg.as_str())?;
                }
                tbl.set("command", cmd_tbl)?;
                Ok(LuaValue::Table(tbl))
            })
        });

        methods.add_method("remove", |_, this, index: usize| {
            this.with_dirty_config(|config| {
                if index == 0 || index > config.spawn_at_startup.len() {
                    return Err(LuaError::external(format!(
                        "spawn_at_startup index {} out of bounds (1-{})",
                        index,
                        config.spawn_at_startup.len()
                    )));
                }
                config.spawn_at_startup.remove(index - 1);
                Ok(())
            })
        });

        methods.add_method("clear", |_, this, ()| {
            this.with_dirty_config(|config| {
                if !config.spawn_at_startup.is_empty() {
                    config.spawn_at_startup.clear();
                }
                Ok(())
            })
        });
    }
}

fn extract_spawn_at_startup(value: &LuaValue) -> LuaResult<niri_config::SpawnAtStartup> {
    match value {
        LuaValue::Table(tbl) => extract_spawn_at_startup_from_table(tbl),
        LuaValue::String(s) => Ok(niri_config::SpawnAtStartup {
            command: vec![s.to_str()?.to_string()],
        }),
        _ => Err(LuaError::external(
            "spawn_at_startup item must be a table or string",
        )),
    }
}

fn extract_spawn_at_startup_from_table(tbl: &LuaTable) -> LuaResult<niri_config::SpawnAtStartup> {
    let cmd_val = tbl.get::<LuaValue>("command")?;
    let command = match cmd_val {
        LuaValue::Table(cmd_tbl) => {
            let mut args = Vec::new();
            for pair in cmd_tbl.pairs::<i64, String>() {
                let (_, arg) = pair?;
                args.push(arg);
            }
            args
        }
        LuaValue::String(s) => vec![s.to_str()?.to_string()],
        LuaValue::Nil => {
            if tbl.contains_key(1)? {
                let mut args = Vec::new();
                for pair in tbl.clone().pairs::<i64, String>() {
                    let (_, arg) = pair?;
                    args.push(arg);
                }
                args
            } else {
                return Err(LuaError::external(
                    "spawn_at_startup requires a 'command' field or array of strings",
                ));
            }
        }
        _ => {
            return Err(LuaError::external(
                "spawn_at_startup command must be a string or array of strings",
            ))
        }
    };

    Ok(niri_config::SpawnAtStartup { command })
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

fn extract_bind(_lua: &Lua, tbl: &LuaTable) -> LuaResult<Bind> {
    let key_str = extract_string_opt(tbl, "key")?
        .ok_or_else(|| LuaError::external("bind requires 'key' field (e.g., 'Mod+Return')"))?;

    let key: Key = key_str
        .parse()
        .map_err(|e| LuaError::external(format!("Invalid key '{}': {}", key_str, e)))?;

    let action = extract_action(tbl)?;

    let repeat = extract_bool_opt(tbl, "repeat")?.unwrap_or(false);
    let cooldown = extract_int_opt(tbl, "cooldown_ms")?.map(|ms| Duration::from_millis(ms as u64));
    let allow_when_locked = extract_bool_opt(tbl, "allow_when_locked")?.unwrap_or(false);
    let allow_inhibiting = extract_bool_opt(tbl, "allow_inhibiting")?.unwrap_or(true);
    let hotkey_overlay_title = extract_string_opt(tbl, "hotkey_overlay_title")?.map(Some);

    Ok(Bind {
        key,
        action,
        repeat,
        cooldown,
        allow_when_locked,
        allow_inhibiting,
        hotkey_overlay_title,
    })
}

fn extract_action(tbl: &LuaTable) -> LuaResult<Action> {
    let action_str = extract_string_opt(tbl, "action")?
        .ok_or_else(|| LuaError::external("bind requires 'action' field"))?;

    let action_lower = action_str.to_lowercase().replace(['-', ' '], "_");

    match action_lower.as_str() {
        // system / meta
        "quit" => {
            let skip = extract_bool_opt(tbl, "skip_confirmation")?.unwrap_or(false);
            Ok(Action::Quit(skip))
        }
        "suspend" => Ok(Action::Suspend),
        "power_off_monitors" | "poweroffmonitors" => Ok(Action::PowerOffMonitors),
        "power_on_monitors" | "poweronmonitors" => Ok(Action::PowerOnMonitors),
        "toggle_debug_tint" | "toggledebugtint" => Ok(Action::ToggleDebugTint),
        "debug_toggle_opaque_regions" | "debugtoggleopaqueregions" => {
            Ok(Action::DebugToggleOpaqueRegions)
        }
        "debug_toggle_damage" | "debugtoggledamage" => Ok(Action::DebugToggleDamage),
        "spawn" => {
            let args = extract_string_array(tbl, "args")?;
            if args.is_empty() {
                return Err(LuaError::external("spawn action requires 'args' array"));
            }
            Ok(Action::Spawn(args))
        }
        "spawn_sh" | "spawnsh" => {
            let cmd = extract_string_opt(tbl, "command")?
                .or_else(|| {
                    extract_string_array(tbl, "args")
                        .ok()
                        .and_then(|a| a.first().cloned())
                })
                .ok_or_else(|| LuaError::external("spawn_sh requires 'command' string"))?;
            Ok(Action::SpawnSh(cmd))
        }
        "do_screen_transition" | "doscreentransition" => {
            let delay_ms = extract_int_opt(tbl, "delay_ms")?.map(|v| v as u16);
            Ok(Action::DoScreenTransition(delay_ms))
        }
        "confirm_screenshot" | "confirmscreenshot" => {
            let write_to_disk = extract_bool_opt(tbl, "write_to_disk")?.unwrap_or(true);
            Ok(Action::ConfirmScreenshot { write_to_disk })
        }
        "cancel_screenshot" | "cancelscreenshot" => Ok(Action::CancelScreenshot),
        "screenshot_toggle_pointer" | "screenshottogglepointer" => {
            Ok(Action::ScreenshotTogglePointer)
        }
        "screenshot" => {
            let show_pointer = extract_bool_opt(tbl, "show_pointer")?.unwrap_or(true);
            let path = extract_string_opt(tbl, "path")?;
            Ok(Action::Screenshot(show_pointer, path))
        }
        "screenshot_screen" | "screenshotscreen" => {
            let write_to_disk = extract_bool_opt(tbl, "write_to_disk")?.unwrap_or(true);
            let show_pointer = extract_bool_opt(tbl, "show_pointer")?.unwrap_or(true);
            let path = extract_string_opt(tbl, "path")?;
            Ok(Action::ScreenshotScreen(write_to_disk, show_pointer, path))
        }
        "screenshot_window" | "screenshotwindow" => {
            let write_to_disk = extract_bool_opt(tbl, "write_to_disk")?.unwrap_or(true);
            let path = extract_string_opt(tbl, "path")?;
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::ScreenshotWindowById {
                    id: id as u64,
                    write_to_disk,
                    path,
                })
            } else {
                Ok(Action::ScreenshotWindow(write_to_disk, path))
            }
        }
        "toggle_keyboard_shortcuts_inhibit" | "togglekeyboardshortcutsinhibit" => {
            Ok(Action::ToggleKeyboardShortcutsInhibit)
        }
        "load_config_file" | "loadconfigfile" => Ok(Action::LoadConfigFile),

        // window focus and movement
        "close_window" | "closewindow" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::CloseWindowById(id as u64))
            } else {
                Ok(Action::CloseWindow)
            }
        }
        "fullscreen_window" | "fullscreenwindow" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::FullscreenWindowById(id as u64))
            } else {
                Ok(Action::FullscreenWindow)
            }
        }
        "toggle_windowed_fullscreen" | "togglewindowedfullscreen" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::ToggleWindowedFullscreenById(id as u64))
            } else {
                Ok(Action::ToggleWindowedFullscreen)
            }
        }
        "focus_window" | "focuswindow" => {
            let id = extract_int_opt(tbl, "id")?
                .ok_or_else(|| LuaError::external("focus_window requires 'id' field (u64)"))?
                as u64;
            Ok(Action::FocusWindow(id))
        }
        "focus_window_in_column" | "focuswindowincolumn" => {
            let index = extract_int_opt(tbl, "index")?
                .ok_or_else(|| LuaError::external("focus_window_in_column requires 'index'"))?
                as u8;
            Ok(Action::FocusWindowInColumn(index))
        }
        "focus_window_previous" | "focuswindowprevious" => Ok(Action::FocusWindowPrevious),
        "focus_column_left" | "focuscolumnleft" => Ok(Action::FocusColumnLeft),
        "focus_column_left_under_mouse" | "focuscolumnleftundermouse" => {
            Ok(Action::FocusColumnLeftUnderMouse)
        }
        "focus_column_right" | "focuscolumnright" => Ok(Action::FocusColumnRight),
        "focus_column_right_under_mouse" | "focuscolumnrightundermouse" => {
            Ok(Action::FocusColumnRightUnderMouse)
        }
        "focus_column_first" | "focuscolumnfirst" => Ok(Action::FocusColumnFirst),
        "focus_column_last" | "focuscolumnlast" => Ok(Action::FocusColumnLast),
        "focus_column_right_or_first" | "focuscolumnrightorfirst" => {
            Ok(Action::FocusColumnRightOrFirst)
        }
        "focus_column_left_or_last" | "focuscolumnleftorlast" => Ok(Action::FocusColumnLeftOrLast),
        "focus_column" | "focuscolumn" => {
            let index = extract_int_opt(tbl, "index")?
                .ok_or_else(|| LuaError::external("focus_column requires 'index'"))?
                as usize;
            Ok(Action::FocusColumn(index))
        }
        "focus_window_or_monitor_up" | "focuswindowormonitorup" => {
            Ok(Action::FocusWindowOrMonitorUp)
        }
        "focus_window_or_monitor_down" | "focuswindowormonitordown" => {
            Ok(Action::FocusWindowOrMonitorDown)
        }
        "focus_column_or_monitor_left" | "focuscolumnormonitorleft" => {
            Ok(Action::FocusColumnOrMonitorLeft)
        }
        "focus_column_or_monitor_right" | "focuscolumnormonitorright" => {
            Ok(Action::FocusColumnOrMonitorRight)
        }
        "focus_window_down" | "focuswindowdown" => Ok(Action::FocusWindowDown),
        "focus_window_up" | "focuswindowup" => Ok(Action::FocusWindowUp),
        "focus_window_down_or_column_left" | "focuswindowdownorcolumnleft" => {
            Ok(Action::FocusWindowDownOrColumnLeft)
        }
        "focus_window_down_or_column_right" | "focuswindowdownorcolumnright" => {
            Ok(Action::FocusWindowDownOrColumnRight)
        }
        "focus_window_up_or_column_left" | "focuswindowuporcolumnleft" => {
            Ok(Action::FocusWindowUpOrColumnLeft)
        }
        "focus_window_up_or_column_right" | "focuswindowuporcolumnright" => {
            Ok(Action::FocusWindowUpOrColumnRight)
        }
        "focus_window_or_workspace_up" | "focuswindoworworkspaceup" => {
            Ok(Action::FocusWindowOrWorkspaceUp)
        }
        "focus_window_or_workspace_down" | "focuswindoworworkspacedown" => {
            Ok(Action::FocusWindowOrWorkspaceDown)
        }
        "focus_window_top" | "focuswindowtop" => Ok(Action::FocusWindowTop),
        "focus_window_bottom" | "focuswindowbottom" => Ok(Action::FocusWindowBottom),
        "focus_window_down_or_top" | "focuswindowdownortop" => Ok(Action::FocusWindowDownOrTop),
        "focus_window_up_or_bottom" | "focuswindowuporbottom" => Ok(Action::FocusWindowUpOrBottom),

        // column / window movement
        "move_column_left" | "movecolumnleft" => Ok(Action::MoveColumnLeft),
        "move_column_right" | "movecolumnright" => Ok(Action::MoveColumnRight),
        "move_column_to_first" | "movecolumntofirst" => Ok(Action::MoveColumnToFirst),
        "move_column_to_last" | "movecolumntolast" => Ok(Action::MoveColumnToLast),
        "move_column_left_or_to_monitor_left" | "movecolumnleftortomonitorleft" => {
            Ok(Action::MoveColumnLeftOrToMonitorLeft)
        }
        "move_column_right_or_to_monitor_right" | "movecolumnrightortomonitorright" => {
            Ok(Action::MoveColumnRightOrToMonitorRight)
        }
        "move_column_to_index" | "movecolumntoindex" => {
            let index = extract_int_opt(tbl, "index")?
                .ok_or_else(|| LuaError::external("move_column_to_index requires 'index'"))?
                as usize;
            Ok(Action::MoveColumnToIndex(index))
        }
        "move_window_down" | "movewindowdown" => Ok(Action::MoveWindowDown),
        "move_window_up" | "movewindowup" => Ok(Action::MoveWindowUp),
        "move_window_down_or_to_workspace_down" | "movewindowdownortoworkspacedown" => {
            Ok(Action::MoveWindowDownOrToWorkspaceDown)
        }
        "move_window_up_or_to_workspace_up" | "movewindowuportoworkspaceup" => {
            Ok(Action::MoveWindowUpOrToWorkspaceUp)
        }
        "consume_or_expel_window_left" | "consumeorexpelwindowleft" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::ConsumeOrExpelWindowLeftById(id as u64))
            } else {
                Ok(Action::ConsumeOrExpelWindowLeft)
            }
        }
        "consume_or_expel_window_right" | "consumeorexpelwindowright" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::ConsumeOrExpelWindowRightById(id as u64))
            } else {
                Ok(Action::ConsumeOrExpelWindowRight)
            }
        }
        "consume_window_into_column" | "consumewindowintocolumn" => {
            Ok(Action::ConsumeWindowIntoColumn)
        }
        "expel_window_from_column" | "expelwindowfromcolumn" => Ok(Action::ExpelWindowFromColumn),
        "swap_window_left" | "swapwindowleft" => Ok(Action::SwapWindowLeft),
        "swap_window_right" | "swapwindowright" => Ok(Action::SwapWindowRight),
        "toggle_column_tabbed_display" | "togglecolumntabbeddisplay" => {
            Ok(Action::ToggleColumnTabbedDisplay)
        }
        "set_column_display" | "setcolumndisplay" => {
            let display_str = extract_string_opt(tbl, "display")?
                .ok_or_else(|| LuaError::external("set_column_display requires 'display'"))?;
            let display = match display_str.to_lowercase().as_str() {
                "normal" => ColumnDisplay::Normal,
                "tabbed" => ColumnDisplay::Tabbed,
                _ => {
                    return Err(LuaError::external(format!(
                        "Invalid column display: {} (use 'normal' or 'tabbed')",
                        display_str
                    )))
                }
            };
            Ok(Action::SetColumnDisplay(display))
        }
        "center_column" | "centercolumn" => Ok(Action::CenterColumn),
        "center_window" | "centerwindow" | "center_window_focused" | "centerwindowfocused" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::CenterWindowById(id as u64))
            } else {
                Ok(Action::CenterWindow)
            }
        }
        "center_visible_columns" | "centervisiblecolumns" => Ok(Action::CenterVisibleColumns),
        "expand_column_to_available_width" | "expandcolumntoavailablewidth" => {
            Ok(Action::ExpandColumnToAvailableWidth)
        }

        // workspace
        "focus_workspace_down" | "focusworkspacedown" => Ok(Action::FocusWorkspaceDown),
        "focus_workspace_down_under_mouse" | "focusworkspacedownundermouse" => {
            Ok(Action::FocusWorkspaceDownUnderMouse)
        }
        "focus_workspace_up" | "focusworkspaceup" => Ok(Action::FocusWorkspaceUp),
        "focus_workspace_up_under_mouse" | "focusworkspaceupundermouse" => {
            Ok(Action::FocusWorkspaceUpUnderMouse)
        }
        "focus_workspace" | "focusworkspace" => {
            let ws_ref = extract_workspace_reference_config(tbl)?;
            Ok(Action::FocusWorkspace(ws_ref))
        }
        "focus_workspace_previous" | "focusworkspaceprevious" => Ok(Action::FocusWorkspacePrevious),
        "move_window_to_workspace_down" | "movewindowtoworkspacedown" => {
            let focus = extract_bool_opt(tbl, "focus")?.unwrap_or(true);
            Ok(Action::MoveWindowToWorkspaceDown(focus))
        }
        "move_window_to_workspace_up" | "movewindowtoworkspaceup" => {
            let focus = extract_bool_opt(tbl, "focus")?.unwrap_or(true);
            Ok(Action::MoveWindowToWorkspaceUp(focus))
        }
        "move_window_to_workspace" | "movewindowtoworkspace" => {
            let reference = extract_workspace_reference_config(tbl)?;
            let focus = extract_bool_opt(tbl, "focus")?.unwrap_or(true);
            if let Some(window_id) = extract_int_opt(tbl, "window_id")?.map(|v| v as u64) {
                Ok(Action::MoveWindowToWorkspaceById {
                    window_id,
                    reference,
                    focus,
                })
            } else {
                Ok(Action::MoveWindowToWorkspace(reference, focus))
            }
        }
        "move_column_to_workspace_down" | "movecolumntoworkspacedown" => {
            let focus = extract_bool_opt(tbl, "focus")?.unwrap_or(true);
            Ok(Action::MoveColumnToWorkspaceDown(focus))
        }
        "move_column_to_workspace_up" | "movecolumntoworkspaceup" => {
            let focus = extract_bool_opt(tbl, "focus")?.unwrap_or(true);
            Ok(Action::MoveColumnToWorkspaceUp(focus))
        }
        "move_column_to_workspace" | "movecolumntoworkspace" => {
            let reference = extract_workspace_reference_config(tbl)?;
            let focus = extract_bool_opt(tbl, "focus")?.unwrap_or(true);
            Ok(Action::MoveColumnToWorkspace(reference, focus))
        }
        "move_workspace_down" | "moveworkspacedown" => Ok(Action::MoveWorkspaceDown),
        "move_workspace_up" | "moveworkspaceup" => Ok(Action::MoveWorkspaceUp),
        "move_workspace_to_index" | "moveworkspacetoindex" => {
            let index = extract_int_opt(tbl, "index")?
                .ok_or_else(|| LuaError::external("move_workspace_to_index requires 'index'"))?
                as usize;
            let reference = extract_table_opt(tbl, "reference")?
                .map(|t| extract_workspace_reference_config(&t))
                .transpose()?;
            if let Some(reference) = reference {
                Ok(Action::MoveWorkspaceToIndexByRef {
                    new_idx: index,
                    reference,
                })
            } else {
                Ok(Action::MoveWorkspaceToIndex(index))
            }
        }
        "set_workspace_name" | "setworkspacename" => {
            let name = extract_string_opt(tbl, "name")?
                .ok_or_else(|| LuaError::external("set_workspace_name requires 'name'"))?;
            let reference = extract_table_opt(tbl, "reference")?
                .map(|t| extract_workspace_reference_config(&t))
                .transpose()?;
            if let Some(reference) = reference {
                Ok(Action::SetWorkspaceNameByRef { name, reference })
            } else {
                Ok(Action::SetWorkspaceName(name))
            }
        }
        "unset_workspace_name" | "unsetworkspacename" => {
            let reference = extract_table_opt(tbl, "reference")?
                .map(|t| extract_workspace_reference_config(&t))
                .transpose()?;
            if let Some(reference) = reference {
                Ok(Action::UnsetWorkSpaceNameByRef(reference))
            } else {
                Ok(Action::UnsetWorkspaceName)
            }
        }

        // monitor
        "focus_monitor_left" | "focusmonitorleft" => Ok(Action::FocusMonitorLeft),
        "focus_monitor_right" | "focusmonitorright" => Ok(Action::FocusMonitorRight),
        "focus_monitor_down" | "focusmonitordown" => Ok(Action::FocusMonitorDown),
        "focus_monitor_up" | "focusmonitorup" => Ok(Action::FocusMonitorUp),
        "focus_monitor_previous" | "focusmonitorprevious" => Ok(Action::FocusMonitorPrevious),
        "focus_monitor_next" | "focusmonitornext" => Ok(Action::FocusMonitorNext),
        "focus_monitor" | "focusmonitor" => {
            let output = extract_string_opt(tbl, "output")?
                .ok_or_else(|| LuaError::external("focus_monitor requires 'output'"))?;
            Ok(Action::FocusMonitor(output))
        }
        "move_window_to_monitor_left" | "movewindowtomonitorleft" => {
            Ok(Action::MoveWindowToMonitorLeft)
        }
        "move_window_to_monitor_right" | "movewindowtomonitorright" => {
            Ok(Action::MoveWindowToMonitorRight)
        }
        "move_window_to_monitor_down" | "movewindowtomonitordown" => {
            Ok(Action::MoveWindowToMonitorDown)
        }
        "move_window_to_monitor_up" | "movewindowtomonitorup" => Ok(Action::MoveWindowToMonitorUp),
        "move_window_to_monitor_previous" | "movewindowtomonitorprevious" => {
            Ok(Action::MoveWindowToMonitorPrevious)
        }
        "move_window_to_monitor_next" | "movewindowtomonitornext" => {
            Ok(Action::MoveWindowToMonitorNext)
        }
        "move_window_to_monitor" | "movewindowtomonitor" => {
            let output = extract_string_opt(tbl, "output")?
                .ok_or_else(|| LuaError::external("move_window_to_monitor requires 'output'"))?;
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::MoveWindowToMonitorById {
                    id: id as u64,
                    output,
                })
            } else {
                Ok(Action::MoveWindowToMonitor(output))
            }
        }
        "move_column_to_monitor_left" | "movecolumntomonitorleft" => {
            Ok(Action::MoveColumnToMonitorLeft)
        }
        "move_column_to_monitor_right" | "movecolumntomonitorright" => {
            Ok(Action::MoveColumnToMonitorRight)
        }
        "move_column_to_monitor_down" | "movecolumntomonitordown" => {
            Ok(Action::MoveColumnToMonitorDown)
        }
        "move_column_to_monitor_up" | "movecolumntomonitorup" => Ok(Action::MoveColumnToMonitorUp),
        "move_column_to_monitor_previous" | "movecolumntomonitorprevious" => {
            Ok(Action::MoveColumnToMonitorPrevious)
        }
        "move_column_to_monitor_next" | "movecolumntomonitornext" => {
            Ok(Action::MoveColumnToMonitorNext)
        }
        "move_column_to_monitor" | "movecolumntomonitor" => {
            let output = extract_string_opt(tbl, "output")?
                .ok_or_else(|| LuaError::external("move_column_to_monitor requires 'output'"))?;
            Ok(Action::MoveColumnToMonitor(output))
        }
        "move_workspace_to_monitor_left" | "moveworkspacetomonitorleft" => {
            Ok(Action::MoveWorkspaceToMonitorLeft)
        }
        "move_workspace_to_monitor_right" | "moveworkspacetomonitorright" => {
            Ok(Action::MoveWorkspaceToMonitorRight)
        }
        "move_workspace_to_monitor_down" | "moveworkspacetomonitordown" => {
            Ok(Action::MoveWorkspaceToMonitorDown)
        }
        "move_workspace_to_monitor_up" | "moveworkspacetomonitorup" => {
            Ok(Action::MoveWorkspaceToMonitorUp)
        }
        "move_workspace_to_monitor_previous" | "moveworkspacetomonitorprevious" => {
            Ok(Action::MoveWorkspaceToMonitorPrevious)
        }
        "move_workspace_to_monitor_next" | "moveworkspacetomonitornext" => {
            Ok(Action::MoveWorkspaceToMonitorNext)
        }
        "move_workspace_to_monitor" | "moveworkspacetomonitor" => {
            let output = extract_string_opt(tbl, "output")?
                .ok_or_else(|| LuaError::external("move_workspace_to_monitor requires 'output'"))?;
            let reference = extract_table_opt(tbl, "reference")?
                .map(|t| extract_workspace_reference_config(&t))
                .transpose()?;
            if let Some(reference) = reference {
                Ok(Action::MoveWorkspaceToMonitorByRef {
                    output_name: output,
                    reference,
                })
            } else {
                Ok(Action::MoveWorkspaceToMonitor(output))
            }
        }

        // sizing / layout
        "set_window_width" | "setwindowwidth" => {
            let change = extract_size_change(tbl)?;
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::SetWindowWidthById {
                    id: id as u64,
                    change,
                })
            } else {
                Ok(Action::SetWindowWidth(change))
            }
        }
        "set_window_height" | "setwindowheight" => {
            let change = extract_size_change(tbl)?;
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::SetWindowHeightById {
                    id: id as u64,
                    change,
                })
            } else {
                Ok(Action::SetWindowHeight(change))
            }
        }
        "reset_window_height" | "resetwindowheight" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::ResetWindowHeightById(id as u64))
            } else {
                Ok(Action::ResetWindowHeight)
            }
        }
        "switch_preset_column_width" | "switchpresetcolumnwidth" => {
            Ok(Action::SwitchPresetColumnWidth)
        }
        "switch_preset_column_width_back" | "switchpresetcolumnwidthback" => {
            Ok(Action::SwitchPresetColumnWidthBack)
        }
        "switch_preset_window_width" | "switchpresetwindowwidth" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::SwitchPresetWindowWidthById(id as u64))
            } else {
                Ok(Action::SwitchPresetWindowWidth)
            }
        }
        "switch_preset_window_width_back" | "switchpresetwindowwidthback" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::SwitchPresetWindowWidthBackById(id as u64))
            } else {
                Ok(Action::SwitchPresetWindowWidthBack)
            }
        }
        "switch_preset_window_height" | "switchpresetwindowheight" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::SwitchPresetWindowHeightById(id as u64))
            } else {
                Ok(Action::SwitchPresetWindowHeight)
            }
        }
        "switch_preset_window_height_back" | "switchpresetwindowheightback" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::SwitchPresetWindowHeightBackById(id as u64))
            } else {
                Ok(Action::SwitchPresetWindowHeightBack)
            }
        }
        "maximize_column" | "maximizecolumn" => Ok(Action::MaximizeColumn),
        "maximize_window_to_edges" | "maximizewindowtoedges" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::MaximizeWindowToEdgesById(id as u64))
            } else {
                Ok(Action::MaximizeWindowToEdges)
            }
        }
        "set_column_width" | "setcolumnwidth" => {
            let change = extract_size_change(tbl)?;
            Ok(Action::SetColumnWidth(change))
        }
        "switch_layout" | "switchlayout" => {
            let target = extract_layout_switch_target(tbl)?;
            Ok(Action::SwitchLayout(target))
        }
        "show_hotkey_overlay" | "showhotkeyoverlay" => Ok(Action::ShowHotkeyOverlay),

        // floating / tiling / overview
        "toggle_window_floating" | "togglewindowfloating" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::ToggleWindowFloatingById(id as u64))
            } else {
                Ok(Action::ToggleWindowFloating)
            }
        }
        "move_window_to_floating" | "movewindowtofloating" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::MoveWindowToFloatingById(id as u64))
            } else {
                Ok(Action::MoveWindowToFloating)
            }
        }
        "move_window_to_tiling" | "movewindowtotiling" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::MoveWindowToTilingById(id as u64))
            } else {
                Ok(Action::MoveWindowToTiling)
            }
        }
        "focus_floating" | "focusfloating" => Ok(Action::FocusFloating),
        "focus_tiling" | "focustiling" => Ok(Action::FocusTiling),
        "switch_focus_between_floating_and_tiling" | "switchfocusbetweenfloatingandtiling" => {
            Ok(Action::SwitchFocusBetweenFloatingAndTiling)
        }
        "move_floating_window" | "movefloatingwindow" => {
            let x = parse_position_change_value(tbl.get("x")?)?;
            let y = parse_position_change_value(tbl.get("y")?)?;
            let id = extract_int_opt(tbl, "id")?.map(|v| v as u64);
            Ok(Action::MoveFloatingWindowById { id, x, y })
        }
        "toggle_window_rule_opacity" | "togglewindowruleopacity" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::ToggleWindowRuleOpacityById(id as u64))
            } else {
                Ok(Action::ToggleWindowRuleOpacity)
            }
        }
        "toggle_overview" | "toggleoverview" => Ok(Action::ToggleOverview),
        "open_overview" | "openoverview" => Ok(Action::OpenOverview),
        "close_overview" | "closeoverview" => Ok(Action::CloseOverview),

        // casting
        "set_dynamic_cast_window" | "setdynamiccastwindow" => {
            if let Some(id) = extract_int_opt(tbl, "id")? {
                Ok(Action::SetDynamicCastWindowById(id as u64))
            } else {
                Ok(Action::SetDynamicCastWindow)
            }
        }
        "set_dynamic_cast_monitor" | "setdynamiccastmonitor" => {
            let output = extract_string_opt(tbl, "output")?;
            Ok(Action::SetDynamicCastMonitor(output))
        }
        "clear_dynamic_cast_target" | "cleardynamiccasttarget" => {
            Ok(Action::ClearDynamicCastTarget)
        }

        // urgency
        "toggle_window_urgent" | "togglewindowurgent" => {
            let id = extract_int_opt(tbl, "id")?
                .ok_or_else(|| LuaError::external("toggle_window_urgent requires 'id'"))?
                as u64;
            Ok(Action::ToggleWindowUrgent(id))
        }
        "set_window_urgent" | "setwindowurgent" => {
            let id = extract_int_opt(tbl, "id")?
                .ok_or_else(|| LuaError::external("set_window_urgent requires 'id'"))?
                as u64;
            Ok(Action::SetWindowUrgent(id))
        }
        "unset_window_urgent" | "unsetwindowurgent" => {
            let id = extract_int_opt(tbl, "id")?
                .ok_or_else(|| LuaError::external("unset_window_urgent requires 'id'"))?
                as u64;
            Ok(Action::UnsetWindowUrgent(id))
        }

        _ => Err(LuaError::external(format!(
            "Unknown action '{}'. See niri documentation for valid actions.",
            action_str
        ))),
    }
}

fn parse_position_change_value(value: LuaValue) -> LuaResult<PositionChange> {
    match value {
        LuaValue::Nil => Err(LuaError::external("position change requires a value")),
        LuaValue::Integer(n) => Ok(PositionChange::SetFixed(n as f64)),
        LuaValue::Number(n) => Ok(PositionChange::SetFixed(n)),
        LuaValue::String(s) => parse_position_change_str(&s.to_str()?),
        other => Err(LuaError::external(format!(
            "position change must be a number or string, got {:?}",
            other.type_name()
        ))),
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

fn extract_string_array(tbl: &LuaTable, field: &str) -> LuaResult<Vec<String>> {
    if let Some(arr_tbl) = extract_table_opt(tbl, field)? {
        let mut result = Vec::new();
        for pair in arr_tbl.pairs::<i64, String>() {
            let (_, s) = pair?;
            result.push(s);
        }
        Ok(result)
    } else {
        Ok(Vec::new())
    }
}

fn extract_workspace_reference(tbl: &LuaTable) -> LuaResult<WorkspaceReferenceArg> {
    if let Some(idx) = extract_int_opt(tbl, "index")? {
        Ok(WorkspaceReferenceArg::Index(idx as u8))
    } else if let Some(name) = extract_string_opt(tbl, "name")? {
        Ok(WorkspaceReferenceArg::Name(name))
    } else if let Some(id) = extract_int_opt(tbl, "id")? {
        Ok(WorkspaceReferenceArg::Id(id as u64))
    } else if let Some(ref_val) = extract_int_opt(tbl, "reference")? {
        Ok(WorkspaceReferenceArg::Index(ref_val as u8))
    } else if let Some(ref_str) = extract_string_opt(tbl, "reference")? {
        if let Ok(idx) = ref_str.parse::<u8>() {
            Ok(WorkspaceReferenceArg::Index(idx))
        } else {
            Ok(WorkspaceReferenceArg::Name(ref_str))
        }
    } else {
        Err(LuaError::external(
            "Workspace action requires 'index', 'name', 'id', or 'reference' field",
        ))
    }
}

fn extract_workspace_reference_config(tbl: &LuaTable) -> LuaResult<WorkspaceReference> {
    extract_workspace_reference(tbl).map(WorkspaceReference::from)
}

fn extract_size_change(tbl: &LuaTable) -> LuaResult<SizeChange> {
    if let Some(fixed) = extract_int_opt(tbl, "fixed")? {
        return Ok(SizeChange::SetFixed(fixed as i32));
    }
    if let Some(proportion) = extract_float_opt(tbl, "proportion")? {
        return Ok(SizeChange::SetProportion(proportion));
    }
    if let Some(change_str) = extract_string_opt(tbl, "change")? {
        return parse_size_change_string(&change_str);
    }
    if let Some(value) = extract_int_opt(tbl, "value")? {
        return Ok(SizeChange::SetFixed(value as i32));
    }
    Err(LuaError::external(
        "Size change requires 'fixed' (int), 'proportion' (float), or 'change' (string like '+10%')",
    ))
}

fn parse_size_change_string(s: &str) -> LuaResult<SizeChange> {
    let s = s.trim();
    if s.ends_with('%') {
        let num_str = s.trim_end_matches('%');
        if let Some(stripped) = num_str.strip_prefix('+') {
            let val: f64 = stripped
                .parse()
                .map_err(|_| LuaError::external(format!("Invalid size change: {}", s)))?;
            Ok(SizeChange::AdjustProportion(val / 100.0))
        } else if let Some(stripped) = num_str.strip_prefix('-') {
            let val: f64 = stripped
                .parse()
                .map_err(|_| LuaError::external(format!("Invalid size change: {}", s)))?;
            Ok(SizeChange::AdjustProportion(-val / 100.0))
        } else {
            let val: f64 = num_str
                .parse()
                .map_err(|_| LuaError::external(format!("Invalid size change: {}", s)))?;
            Ok(SizeChange::SetProportion(val / 100.0))
        }
    } else if let Some(stripped) = s.strip_prefix('+') {
        let val: i32 = stripped
            .parse()
            .map_err(|_| LuaError::external(format!("Invalid size change: {}", s)))?;
        Ok(SizeChange::AdjustFixed(val))
    } else if let Some(stripped) = s.strip_prefix('-') {
        let val: i32 = stripped
            .parse()
            .map_err(|_| LuaError::external(format!("Invalid size change: {}", s)))?;
        Ok(SizeChange::AdjustFixed(-val))
    } else {
        let val: i32 = s
            .parse()
            .map_err(|_| LuaError::external(format!("Invalid size change: {}", s)))?;
        Ok(SizeChange::SetFixed(val))
    }
}

fn extract_layout_switch_target(tbl: &LuaTable) -> LuaResult<LayoutSwitchTarget> {
    if let Some(idx) = extract_int_opt(tbl, "layout")? {
        return Ok(LayoutSwitchTarget::Index(idx as u8));
    }
    if let Some(target_str) = extract_string_opt(tbl, "layout")? {
        return match target_str.to_lowercase().as_str() {
            "next" => Ok(LayoutSwitchTarget::Next),
            "prev" | "previous" => Ok(LayoutSwitchTarget::Prev),
            _ => {
                if let Ok(idx) = target_str.parse::<u8>() {
                    Ok(LayoutSwitchTarget::Index(idx))
                } else {
                    Err(LuaError::external(format!(
                        "Invalid layout target: {}. Use 'next', 'prev', or an index.",
                        target_str
                    )))
                }
            }
        };
    }
    if let Some(target_str) = extract_string_opt(tbl, "target")? {
        return match target_str.to_lowercase().as_str() {
            "next" => Ok(LayoutSwitchTarget::Next),
            "prev" | "previous" => Ok(LayoutSwitchTarget::Prev),
            _ => Err(LuaError::external(format!(
                "Invalid layout target: {}. Use 'next', 'prev', or an index.",
                target_str
            ))),
        };
    }
    Ok(LayoutSwitchTarget::Next)
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
}
