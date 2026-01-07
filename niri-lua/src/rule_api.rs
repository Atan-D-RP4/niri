//! Simplified Rule API for niri Lua configuration.

use mlua::{Lua, Result, Table, Value};

use crate::collections::{
    extract_bind, extract_layer_rule, extract_window_rule, extract_workspace,
};
use crate::config_state::{ConfigState, DirtyFlag};

pub fn register_rule_namespace(lua: &Lua, niri_table: &Table, state: ConfigState) -> Result<()> {
    let rule_table = lua.create_table()?;

    let state_clone = state.clone();
    let window_fn = lua.create_function(move |lua, table: Table| {
        let rule = extract_window_rule(lua, &table)?;
        state_clone.borrow_config_mut().window_rules.push(rule);
        state_clone.mark_dirty(DirtyFlag::WindowRules);
        Ok(())
    })?;
    rule_table.set("window", window_fn)?;

    let state_clone = state.clone();
    let layer_fn = lua.create_function(move |lua, table: Table| {
        let rule = extract_layer_rule(lua, &table)?;
        state_clone.borrow_config_mut().layer_rules.push(rule);
        state_clone.mark_dirty(DirtyFlag::LayerRules);
        Ok(())
    })?;
    rule_table.set("layer", layer_fn)?;

    niri_table.set("rule", rule_table)?;
    Ok(())
}

pub fn register_workspace_namespace(
    lua: &Lua,
    niri_table: &Table,
    state: ConfigState,
) -> Result<()> {
    let workspace_table: Table = niri_table.get("workspace").unwrap_or_else(|_| {
        lua.create_table()
            .expect("Failed to create workspace table")
    });

    let add_fn = lua.create_function(move |lua, table: Table| {
        let workspace = extract_workspace(lua, &table)?;
        state.borrow_config_mut().workspaces.push(workspace);
        state.mark_dirty(DirtyFlag::Workspaces);
        Ok(())
    })?;
    workspace_table.set("add", add_fn)?;

    niri_table.set("workspace", workspace_table)?;
    Ok(())
}

pub fn register_bind_namespace(lua: &Lua, niri_table: &Table, state: ConfigState) -> Result<()> {
    let bind_table = lua.create_table()?;

    let state_clone = state.clone();
    let add_fn = lua.create_function(move |lua, args: mlua::MultiValue| {
        let bind = parse_bind_args(lua, args)?;
        state_clone.borrow_config_mut().binds.0.push(bind);
        state_clone.mark_dirty(DirtyFlag::Binds);
        Ok(())
    })?;
    bind_table.set("add", add_fn)?;

    let state_clone = state.clone();
    let remove_fn = lua.create_function(move |_lua, key: String| {
        state_clone.borrow_config_mut().binds.0.retain(|b| {
            let bind_key = format!("{:?}", b.key);
            bind_key != key
        });
        state_clone.mark_dirty(DirtyFlag::Binds);
        Ok(())
    })?;
    bind_table.set("remove", remove_fn)?;

    let clear_fn = lua.create_function(move |_lua, ()| {
        state.borrow_config_mut().binds.0.clear();
        state.mark_dirty(DirtyFlag::Binds);
        Ok(())
    })?;
    bind_table.set("clear", clear_fn)?;

    niri_table.set("bind", bind_table)?;
    Ok(())
}

/// Parses bind args from positional `("Mod+Q", "close_window")` or table `({ key=..., action=...
/// })` form.
fn parse_bind_args(lua: &Lua, args: mlua::MultiValue) -> Result<niri_config::Bind> {
    let args_vec: Vec<Value> = args.into_iter().collect();

    if args_vec.is_empty() {
        return Err(mlua::Error::runtime(
            "bind.add requires at least one argument",
        ));
    }

    match &args_vec[0] {
        Value::Table(table) => extract_bind(lua, table),
        Value::String(key_str) => {
            let key = key_str.to_str()?.to_string();

            let action = args_vec
                .get(1)
                .and_then(|v| v.as_string().map(|s| s.to_string_lossy()))
                .ok_or_else(|| {
                    mlua::Error::runtime("bind.add requires action as second argument")
                })?;

            let table = lua.create_table()?;
            table.set("key", key)?;
            table.set("action", action)?;

            if let Some(Value::Table(args_table)) = args_vec.get(2) {
                table.set("args", args_table.clone())?;
            }

            if let Some(Value::Table(opts)) = args_vec.get(3) {
                for pair in opts.pairs::<String, Value>() {
                    let (k, v) = pair?;
                    table.set(k, v)?;
                }
            }

            extract_bind(lua, &table)
        }
        _ => Err(mlua::Error::runtime(
            "bind.add expects either a table or (key, action, [args], [options])",
        )),
    }
}

pub fn register_all(lua: &Lua, niri_table: &Table, state: ConfigState) -> Result<()> {
    register_rule_namespace(lua, niri_table, state.clone())?;
    register_workspace_namespace(lua, niri_table, state.clone())?;
    register_bind_namespace(lua, niri_table, state)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use niri_config::Config;

    use super::*;
    use crate::config_dirty::ConfigDirtyFlags;

    fn create_test_state() -> ConfigState {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        ConfigState::new(config, dirty)
    }

    fn setup_lua_with_niri() -> (Lua, Table, ConfigState) {
        let lua = Lua::new();
        let niri_table = lua.create_table().unwrap();
        let state = create_test_state();
        register_all(&lua, &niri_table, state.clone()).unwrap();
        (lua, niri_table, state)
    }

    #[test]
    fn test_rule_window_adds_window_rule() {
        let (lua, niri_table, state) = setup_lua_with_niri();
        lua.globals().set("niri", niri_table).unwrap();

        let initial_count = state.borrow_config().window_rules.len();

        lua.load(
            r#"
            niri.rule.window({
                match = { app_id = "firefox" },
                open_floating = true
            })
        "#,
        )
        .exec()
        .unwrap();

        let new_count = state.borrow_config().window_rules.len();
        assert_eq!(new_count, initial_count + 1);
        assert!(state.is_any_dirty());
    }

    #[test]
    fn test_rule_layer_adds_layer_rule() {
        let (lua, niri_table, state) = setup_lua_with_niri();
        lua.globals().set("niri", niri_table).unwrap();

        let initial_count = state.borrow_config().layer_rules.len();

        lua.load(
            r#"
            niri.rule.layer({
                match = { namespace = "waybar" },
                opacity = 0.9
            })
        "#,
        )
        .exec()
        .unwrap();

        let new_count = state.borrow_config().layer_rules.len();
        assert_eq!(new_count, initial_count + 1);
        assert!(state.is_any_dirty());
    }

    #[test]
    fn test_workspace_add() {
        let (lua, niri_table, state) = setup_lua_with_niri();
        lua.globals().set("niri", niri_table).unwrap();

        let initial_count = state.borrow_config().workspaces.len();

        lua.load(r#"niri.workspace.add({ name = "dev" })"#)
            .exec()
            .unwrap();

        let new_count = state.borrow_config().workspaces.len();
        assert_eq!(new_count, initial_count + 1);
        assert!(state.is_any_dirty());
    }

    #[test]
    fn test_bind_add_table_form() {
        let (lua, niri_table, state) = setup_lua_with_niri();
        lua.globals().set("niri", niri_table).unwrap();

        let initial_count = state.borrow_config().binds.0.len();

        lua.load(
            r#"
            niri.bind.add({
                key = "Mod+Q",
                action = "close_window"
            })
        "#,
        )
        .exec()
        .unwrap();

        let new_count = state.borrow_config().binds.0.len();
        assert_eq!(new_count, initial_count + 1);
        assert!(state.is_any_dirty());
    }

    #[test]
    fn test_bind_add_positional_form() {
        let (lua, niri_table, state) = setup_lua_with_niri();
        lua.globals().set("niri", niri_table).unwrap();

        let initial_count = state.borrow_config().binds.0.len();

        lua.load(r#"niri.bind.add("Mod+Return", "spawn", {"alacritty"})"#)
            .exec()
            .unwrap();

        let new_count = state.borrow_config().binds.0.len();
        assert_eq!(new_count, initial_count + 1);
    }

    #[test]
    fn test_bind_clear() {
        let (lua, niri_table, state) = setup_lua_with_niri();
        lua.globals().set("niri", niri_table).unwrap();

        lua.load(
            r#"
            niri.bind.add("Mod+Q", "close_window")
            niri.bind.add("Mod+Return", "spawn", {"alacritty"})
        "#,
        )
        .exec()
        .unwrap();

        assert!(state.borrow_config().binds.0.len() >= 2);

        lua.load("niri.bind.clear()").exec().unwrap();

        assert_eq!(state.borrow_config().binds.0.len(), 0);
    }
}
