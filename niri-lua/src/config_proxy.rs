use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataMethods};

use crate::config_state::ConfigState;
use crate::event_system::EventSystem;
use crate::property_registry::{PropertyRegistry, PropertyType};

/// Unified config proxy that dispatches property access through the global registry.
#[derive(Clone, Debug, Default)]
pub struct ConfigProxy {
    /// Current config path (empty string for root).
    pub current_path: String,
}

impl ConfigProxy {
    fn child_path(&self, key: &str) -> String {
        if self.current_path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", self.current_path, key)
        }
    }

    fn nested_proxy(lua: &Lua, path: String) -> LuaResult<LuaValue> {
        Ok(LuaValue::UserData(
            lua.create_userdata(ConfigProxy { current_path: path })?,
        ))
    }

    fn get_state(lua: &Lua) -> LuaResult<Rc<RefCell<ConfigState>>> {
        lua.app_data_ref::<Rc<RefCell<ConfigState>>>()
            .map(|state| state.clone())
            .ok_or_else(|| LuaError::external("config state not initialized"))
    }

    fn get_value(
        lua: &Lua,
        _path: &str,
        desc: &crate::property_registry::PropertyDescriptor,
    ) -> LuaResult<LuaValue> {
        let state = Self::get_state(lua)?;
        let state_ref = state.borrow();
        let config = state_ref.borrow_config();
        (desc.getter)(lua, &config)
    }
}

impl UserData for ConfigProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: String| {
            let path = this.child_path(&key);
            let registry = PropertyRegistry::global();

            if let Some(desc) = registry.get(&path) {
                if matches!(desc.ty, PropertyType::Nested) {
                    return Self::nested_proxy(lua, path);
                }

                return Self::get_value(lua, &path, desc);
            }

            if registry.is_nested(&path) {
                return Self::nested_proxy(lua, path);
            }

            Err(LuaError::external(format!(
                "unknown config property: {}",
                path
            )))
        });

        methods.add_meta_method(
            MetaMethod::NewIndex,
            |lua, this, (key, value): (String, LuaValue)| {
                let path = this.child_path(&key);
                let registry = PropertyRegistry::global();

                if let Some(desc) = registry.get(&path) {
                    if matches!(desc.ty, PropertyType::Nested) {
                        if let LuaValue::Table(table) = value {
                            let state = Self::get_state(lua)?;
                            let state_ref = state.borrow();
                            crate::config_wrapper::apply_table_to_nested_path(
                                lua, &state_ref, registry, &path, &table,
                            )?;
                            return Ok(());
                        } else {
                            return Err(LuaError::external(format!(
                                "cannot assign {} to nested config section '{}', expected table",
                                value.type_name(),
                                path
                            )));
                        }
                    }

                    let state = Self::get_state(lua)?;
                    let set_result = {
                        let state_ref = state.borrow();
                        state_ref.with_config(|config| (desc.setter)(lua, config, value.clone()))
                    };

                    set_result?;

                    {
                        let state_ref = state.borrow();
                        state_ref.mark_dirty(desc.dirty_flag);
                        state_ref.with_dirty_flags(|flags| flags.mark_dirty(&path));
                    }

                    if desc.signal {
                        if let Some(events) = lua.app_data_ref::<Rc<RefCell<EventSystem>>>() {
                            let current_value = {
                                let state_ref = state.borrow();
                                let config = state_ref.borrow_config();
                                (desc.getter)(lua, &config)?
                            };

                            let payload = lua.create_table()?;
                            payload.set("path", path.as_str())?;
                            payload.set("value", current_value)?;

                            events.borrow().emit(
                                lua,
                                &format!("config::{}", path),
                                LuaValue::Table(payload),
                            )?;
                        }
                    }

                    return Ok(());
                }

                if registry.is_nested(&path) {
                    match &value {
                        LuaValue::Table(table) => {
                            let state = Self::get_state(lua)?;
                            let state_ref = state.borrow();
                            crate::config_wrapper::apply_table_to_nested_path(
                                lua, &state_ref, registry, &path, table,
                            )?;
                            return Ok(());
                        }
                        LuaValue::UserData(ud) => {
                            if let Ok(proxy) = ud.borrow::<ConfigProxy>() {
                                if proxy.current_path == path {
                                    return Ok(());
                                }
                            }
                            return Err(LuaError::external(format!(
                                "cannot assign {} to nested config section '{}', expected table",
                                value.type_name(),
                                path
                            )));
                        }
                        _ => {
                            return Err(LuaError::external(format!(
                                "cannot assign {} to nested config section '{}', expected table",
                                value.type_name(),
                                path
                            )));
                        }
                    }
                }

                Err(LuaError::external(format!(
                    "unknown config property: {}",
                    path
                )))
            },
        );

        methods.add_meta_method(MetaMethod::Iter, |lua, this, ()| {
            let registry = PropertyRegistry::global();
            let keys = registry.child_keys(&this.current_path);
            let current_path = this.current_path.clone();
            let idx = Rc::new(RefCell::new(0usize));

            lua.create_function(move |lua, ()| {
                let mut i = idx.borrow_mut();
                if *i >= keys.len() {
                    return Ok((LuaValue::Nil, LuaValue::Nil));
                }

                let key = &keys[*i];
                *i += 1;

                let path = if current_path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", current_path, key)
                };

                let registry = PropertyRegistry::global();
                let value = if let Some(desc) = registry.get(&path) {
                    if matches!(desc.ty, PropertyType::Nested) {
                        Self::nested_proxy(lua, path)?
                    } else {
                        Self::get_value(lua, &path, desc)?
                    }
                } else if registry.is_nested(&path) {
                    Self::nested_proxy(lua, path)?
                } else {
                    LuaValue::Nil
                };

                Ok((LuaValue::String(lua.create_string(key)?), value))
            })
        });

        methods.add_method("snapshot", |lua, this, ()| {
            let registry = PropertyRegistry::global();
            let table = lua.create_table()?;

            for key in registry.child_keys(&this.current_path) {
                let path = this.child_path(&key);
                if let Some(desc) = registry.get(&path) {
                    if matches!(desc.ty, PropertyType::Nested) {
                        continue;
                    }

                    let value = Self::get_value(lua, &path, desc)?;
                    table.set(key, value)?;
                }
            }

            Ok(table)
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            if this.current_path.is_empty() {
                Ok("ConfigProxy(root)".to_string())
            } else {
                Ok(format!("ConfigProxy({})", this.current_path))
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use mlua::{FromLua, Lua};
    use niri_config::Config;

    use super::*;
    use crate::config_dirty::ConfigDirtyFlags;
    use crate::config_state::DirtyFlag;
    use crate::event_handlers::EventHandlers;
    use crate::property_registry::{
        extract_bool, extract_integer, extract_number, PropertyDescriptor,
    };

    fn ensure_registry() {
        if PropertyRegistry::try_global().is_some() {
            return;
        }
        PropertyRegistry::init_from_config();
    }

    fn make_state() -> Rc<RefCell<ConfigState>> {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        Rc::new(RefCell::new(ConfigState::new(config, dirty)))
    }

    fn root_proxy() -> ConfigProxy {
        ConfigProxy {
            current_path: String::new(),
        }
    }

    #[test]
    fn index_returns_value_and_nested_proxy() {
        ensure_registry();
        let lua = Lua::new();
        lua.set_app_data(make_state());

        let globals = lua.globals();
        globals.set("config", root_proxy()).unwrap();

        let gaps: f64 = lua.load("return config.layout.gaps").eval().unwrap();
        assert_eq!(gaps, Config::default().layout.gaps);

        let layout_tostring: String = lua
            .load("local l = config.layout; return tostring(l)")
            .eval()
            .unwrap();
        assert_eq!(layout_tostring, "ConfigProxy(layout)");
    }

    #[test]
    fn newindex_sets_value_and_marks_dirty() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state.clone());

        lua.globals().set("config", root_proxy()).unwrap();
        lua.load("config.layout.gaps = 20.0").exec().unwrap();

        let state_ref = state.borrow();
        assert!(state_ref.borrow_dirty_flags().layout);
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("layout.gaps"));
        assert_eq!(state_ref.borrow_config().layout.gaps, 20.0);
    }

    #[test]
    fn emits_config_events_on_set() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state);

        let handlers = Rc::new(RefCell::new(EventHandlers::new()));
        let handler_calls = lua.create_table().unwrap();
        handler_calls.set("count", 0).unwrap();

        {
            let handler_calls = handler_calls.clone();
            let callback = lua
                .create_function(move |_, payload: LuaValue| {
                    let current: i64 = handler_calls.get("count")?;
                    handler_calls.set("count", current + 1)?;

                    let table = payload.as_table().unwrap();
                    let path: String = table.get("path")?;
                    assert_eq!(path, "layout.gaps");
                    Ok(())
                })
                .unwrap();

            handlers
                .borrow_mut()
                .register_handler("config::layout.gaps", callback, false);
        }

        let event_system = EventSystem::new(handlers.clone());
        lua.set_app_data(Rc::new(RefCell::new(event_system)));

        lua.globals().set("config", root_proxy()).unwrap();
        lua.load("config.layout.gaps = 2.5").exec().unwrap();

        let calls: i64 = handler_calls.get("count").unwrap();
        assert_eq!(calls, 1);
    }

    #[test]
    fn pairs_and_snapshot_work_for_direct_children() {
        ensure_registry();
        let lua = Lua::new();
        lua.set_app_data(make_state());

        lua.globals().set("config", root_proxy()).unwrap();

        // In Luau, __iter is used with for-in directly (not pairs)
        let keys: Vec<String> = lua
            .load("local t={} for k,_ in config.layout do table.insert(t, k) end return t")
            .eval()
            .unwrap();
        // The registry may have more properties when init_from_config runs first,
        // so we just check that "gaps" is present (robust to global registry state)
        assert!(
            keys.contains(&"gaps".to_string()),
            "Expected 'gaps' in keys: {:?}",
            keys
        );

        let snapshot: mlua::Table = lua.load("return config.layout:snapshot()").eval().unwrap();
        let gaps: f64 = snapshot.get("gaps").unwrap();
        assert_eq!(gaps, Config::default().layout.gaps);
    }

    #[test]
    fn test_config_proxy_cursor_get() {
        ensure_registry();
        let lua = Lua::new();
        lua.set_app_data(make_state());

        lua.globals().set("config", root_proxy()).unwrap();

        let size: i64 = lua
            .load("return config.cursor.xcursor_size")
            .eval()
            .unwrap();
        assert_eq!(size, Config::default().cursor.xcursor_size as i64);
    }

    #[test]
    fn test_config_proxy_cursor_set() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state.clone());

        lua.globals().set("config", root_proxy()).unwrap();
        lua.load("config.cursor.xcursor_size = 42").exec().unwrap();

        let state_ref = state.borrow();
        assert!(state_ref.borrow_dirty_flags().cursor);
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("cursor.xcursor_size"));
        assert_eq!(state_ref.borrow_config().cursor.xcursor_size, 42);
    }

    #[test]
    fn test_config_proxy_nested_access_returns_nested_proxy() {
        ensure_registry();
        let lua = Lua::new();
        lua.set_app_data(make_state());

        lua.globals().set("config", root_proxy()).unwrap();

        let tostring_cursor: String = lua
            .load("local c = config.cursor; return tostring(c)")
            .eval()
            .unwrap();
        assert_eq!(tostring_cursor, "ConfigProxy(cursor)");

        let size: i64 = lua
            .load("local c = config.cursor; return c.xcursor_size")
            .eval()
            .unwrap();
        assert_eq!(size, Config::default().cursor.xcursor_size as i64);
    }

    #[test]
    fn test_nested_table_assignment() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state.clone());

        lua.globals().set("config", root_proxy()).unwrap();

        lua.load(
            r#"
            config.cursor = {
                xcursor_size = 48,
                hide_when_typing = true
            }
            "#,
        )
        .exec()
        .unwrap();

        let state_ref = state.borrow();
        assert!(state_ref.borrow_dirty_flags().cursor);
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("cursor.xcursor_size"));
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("cursor.hide_when_typing"));
        assert_eq!(state_ref.borrow_config().cursor.xcursor_size, 48);
        assert!(state_ref.borrow_config().cursor.hide_when_typing);
    }

    #[test]
    fn test_nested_table_assignment_rejects_non_table() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state);

        lua.globals().set("config", root_proxy()).unwrap();

        let result = lua.load("config.cursor = 42").exec();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("expected table") || err.contains("cannot assign"));
    }

    #[test]
    fn test_nested_table_assignment_unknown_property_error() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state);

        lua.globals().set("config", root_proxy()).unwrap();

        let result = lua
            .load("config.cursor = { nonexistent_field = true }")
            .exec();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown") || err.contains("nonexistent_field"));
    }

    #[test]
    fn test_nested_table_assignment_layout_shadow() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state.clone());

        lua.globals().set("config", root_proxy()).unwrap();

        lua.load(
            r#"
            config.layout.shadow = {
                on = true,
                softness = 25,
                spread = 10,
                draw_behind_window = true
            }
            "#,
        )
        .exec()
        .unwrap();

        let state_ref = state.borrow();
        assert!(state_ref.borrow_dirty_flags().layout);
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("layout.shadow.on"));
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("layout.shadow.softness"));

        let shadow = &state_ref.borrow_config().layout.shadow;
        assert!(shadow.on);
        assert!((shadow.softness - 25.0).abs() < 0.01);
        assert!((shadow.spread - 10.0).abs() < 0.01);
        assert!(shadow.draw_behind_window);
    }

    #[test]
    fn test_nested_table_assignment_input_touchpad() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state.clone());

        lua.globals().set("config", root_proxy()).unwrap();

        lua.load(
            r#"
            config.input.touchpad = {
                tap = true,
                natural_scroll = true,
                accel_speed = 0.5,
                dwt = true
            }
            "#,
        )
        .exec()
        .unwrap();

        let state_ref = state.borrow();
        assert!(state_ref.borrow_dirty_flags().input);
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("input.touchpad.tap"));
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("input.touchpad.natural_scroll"));

        let touchpad = &state_ref.borrow_config().input.touchpad;
        assert!(touchpad.tap);
        assert!(touchpad.natural_scroll);
        assert!((touchpad.accel_speed.0 - 0.5).abs() < 0.01);
        assert!(touchpad.dwt);
    }

    #[test]
    fn test_nested_table_assignment_input_touch() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state.clone());

        lua.globals().set("config", root_proxy()).unwrap();

        lua.load(
            r#"
            config.input.touch = {
                off = true,
                natural_scroll = true
            }
            "#,
        )
        .exec()
        .unwrap();

        let state_ref = state.borrow();
        assert!(state_ref.borrow_dirty_flags().input);
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("input.touch.off"));
        assert!(state_ref
            .borrow_dirty_flags()
            .dirty_paths
            .contains("input.touch.natural_scroll"));

        let touch = &state_ref.borrow_config().input.touch;
        assert!(touch.off);
        assert!(touch.natural_scroll);
    }

    #[test]
    fn test_deeply_nested_table_assignment() {
        ensure_registry();
        let lua = Lua::new();
        let state = make_state();
        lua.set_app_data(state.clone());

        lua.globals().set("config", root_proxy()).unwrap();

        lua.load(
            r#"
            config.input = {
                touchpad = {
                    tap = true,
                    natural_scroll = true
                },
                touch = {
                    off = false,
                    natural_scroll = true
                }
            }
            "#,
        )
        .exec()
        .unwrap();

        let state_ref = state.borrow();
        assert!(state_ref.borrow_dirty_flags().input);

        let config = state_ref.borrow_config();
        assert!(config.input.touchpad.tap);
        assert!(config.input.touchpad.natural_scroll);
        assert!(!config.input.touch.off);
        assert!(config.input.touch.natural_scroll);
    }
}
