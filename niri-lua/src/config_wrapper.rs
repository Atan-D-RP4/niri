use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;
use mlua::UserData;
use niri_config::{Config, ScreenshotPath};

use crate::collections::{
    BindsCollection, EnvironmentCollection, LayerRulesCollection, OutputsCollection,
    SpawnAtStartupCollection, WindowRulesCollection, WorkspacesCollection,
};
use crate::config_dirty::ConfigDirtyFlags;
use crate::config_state::ConfigState;
use crate::property_registry::{PropertyRegistry, PropertyType};

macro_rules! collection_field {
    ($fields:expr, $name:literal, $collection:ident) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok($collection {
                state: this.state(),
            })
        });
    };
}

macro_rules! scalar_field {
    ($fields:expr, $name:literal, $config_field:ident, $dirty_flag:ident, $type:ty) => {
        $fields.add_field_method_get($name, |_, this| Ok(this.config.borrow().$config_field));

        $fields.add_field_method_set($name, |_, this, value: $type| {
            this.config.borrow_mut().$config_field = value;
            let mut dirty = this.dirty.borrow_mut();
            dirty.$dirty_flag = true;
            dirty.mark_dirty($name);
            Ok(())
        });
    };
}

macro_rules! wrapper_field {
    ($fields:expr, $name:literal, $config_field:ident, $dirty_flag:ident, $wrapper:ident, $inner:ty) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok(this.config.borrow().$config_field.0.clone())
        });

        $fields.add_field_method_set($name, |_, this, value: $inner| {
            this.config.borrow_mut().$config_field = $wrapper(value);
            let mut dirty = this.dirty.borrow_mut();
            dirty.$dirty_flag = true;
            dirty.mark_dirty($name);
            Ok(())
        });
    };
}

#[derive(Clone)]
pub struct ConfigWrapper {
    pub config: Rc<RefCell<Config>>,
    pub dirty: Rc<RefCell<ConfigDirtyFlags>>,
}

impl ConfigWrapper {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        Self::new_with_shared_state(config, Rc::new(RefCell::new(ConfigDirtyFlags::default())))
    }

    pub fn new_with_shared_state(
        config: Rc<RefCell<Config>>,
        dirty: Rc<RefCell<ConfigDirtyFlags>>,
    ) -> Self {
        Self { config, dirty }
    }

    pub fn new_default() -> Self {
        Self::new_with_shared_state(
            Rc::new(RefCell::new(Config::default())),
            Rc::new(RefCell::new(ConfigDirtyFlags::default())),
        )
    }

    pub fn take_dirty_flags(&self) -> ConfigDirtyFlags {
        self.dirty.borrow_mut().take()
    }

    pub fn has_dirty_flags(&self) -> bool {
        self.dirty.borrow().any()
    }

    pub fn get_config(&self) -> Rc<RefCell<Config>> {
        self.config.clone()
    }

    pub fn extract_config(&self) -> Config {
        let mut guard = self.config.borrow_mut();
        std::mem::take(&mut *guard)
    }

    pub fn swap_config(&self, new_config: Config) -> Config {
        let mut guard = self.config.borrow_mut();
        std::mem::replace(&mut *guard, new_config)
    }

    pub fn state(&self) -> ConfigState {
        ConfigState::new(self.config.clone(), self.dirty.clone())
    }
}

impl UserData for ConfigWrapper {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Collection fields: read-only access to collections
        collection_field!(fields, "workspaces", WorkspacesCollection);
        collection_field!(fields, "outputs", OutputsCollection);
        collection_field!(fields, "window_rules", WindowRulesCollection);
        collection_field!(fields, "binds", BindsCollection);
        collection_field!(fields, "environment", EnvironmentCollection);
        collection_field!(fields, "layer_rules", LayerRulesCollection);
        collection_field!(fields, "spawn_at_startup", SpawnAtStartupCollection);

        // Scalar fields
        scalar_field!(fields, "prefer_no_csd", prefer_no_csd, misc, bool);

        // Wrapper fields (newtypes)
        wrapper_field!(
            fields,
            "screenshot_path",
            screenshot_path,
            misc,
            ScreenshotPath,
            Option<String>
        );
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("apply", |_, _this, ()| Ok(()));

        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            let registry = crate::property_registry::PropertyRegistry::try_global()
                .ok_or_else(|| LuaError::runtime("PropertyRegistry not initialized"))?;

            if let Some(desc) = registry.get(&key) {
                if matches!(desc.ty, crate::property_registry::PropertyType::Nested) {
                    return Ok(LuaValue::UserData(lua.create_userdata(
                        crate::config_proxy::ConfigProxy {
                            current_path: key.clone(),
                        },
                    )?));
                }
                let state = this.state();
                let config = state.borrow_config();
                return (desc.getter)(lua, &*config);
            }

            if registry.is_nested(&key) {
                return Ok(LuaValue::UserData(lua.create_userdata(
                    crate::config_proxy::ConfigProxy { current_path: key },
                )?));
            }

            Ok(LuaValue::Nil)
        });

        methods.add_meta_method(
            LuaMetaMethod::NewIndex,
            |lua, this, (key, value): (String, LuaValue)| {
                let registry = crate::property_registry::PropertyRegistry::try_global()
                    .ok_or_else(|| LuaError::runtime("PropertyRegistry not initialized"))?;

                // Check if key is a leaf property with a setter
                if let Some(desc) = registry.get(&key) {
                    if !matches!(desc.ty, PropertyType::Nested) {
                        let state = this.state();
                        let mut config = state.borrow_config_mut();
                        (desc.setter)(lua, &mut *config, value)?;
                        state.mark_dirty(desc.dirty_flag);
                        state.with_dirty_flags(|flags| flags.mark_dirty(&key));
                        return Ok(());
                    }
                }

                // Handle table assignment to nested paths (e.g., niri.config.input = { ... })
                if registry.is_nested(&key)
                    || matches!(
                        registry.get(&key).map(|d| &d.ty),
                        Some(PropertyType::Nested)
                    )
                {
                    match &value {
                        LuaValue::Table(table) => {
                            let state = this.state();
                            apply_table_to_nested_path(lua, &state, registry, &key, table)?;
                            return Ok(());
                        }
                        LuaValue::UserData(ud) => {
                            if let Ok(proxy) = ud.borrow::<crate::config_proxy::ConfigProxy>() {
                                if proxy.current_path == key {
                                    return Ok(());
                                }
                            }
                            return Err(LuaError::runtime(format!(
                                "cannot assign {} to nested config section '{}', expected table",
                                value.type_name(),
                                key
                            )));
                        }
                        _ => {
                            return Err(LuaError::runtime(format!(
                                "cannot assign {} to nested config section '{}', expected table",
                                value.type_name(),
                                key
                            )));
                        }
                    }
                }

                Err(LuaError::runtime(format!(
                    "attempt to set an unknown field '{}'",
                    key
                )))
            },
        );
    }
}

pub fn register_config_wrapper(lua: &Lua, wrapper: ConfigWrapper) -> LuaResult<()> {
    let globals = lua.globals();
    let niri: LuaTable = globals.get("niri")?;
    niri.set("config", wrapper)?;
    Ok(())
}

/// Recursively applies a Lua table to a nested config path.
///
/// For each key in the table:
/// - If the child path is a leaf property, call its setter
/// - If the child path is nested and value is a table, recurse
/// - Otherwise, raise an error
pub fn apply_table_to_nested_path(
    lua: &Lua,
    state: &ConfigState,
    registry: &PropertyRegistry,
    base_path: &str,
    table: &LuaTable,
) -> LuaResult<()> {
    for pair in table.clone().pairs::<String, LuaValue>() {
        let (key, value) = pair?;
        let child_path = if base_path.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", base_path, key)
        };

        if let Some(desc) = registry.get(&child_path) {
            if matches!(desc.ty, PropertyType::Nested) {
                if let LuaValue::Table(nested_table) = value {
                    apply_table_to_nested_path(lua, state, registry, &child_path, &nested_table)?;
                } else {
                    return Err(LuaError::runtime(format!(
                        "cannot assign {} to nested config section '{}', expected table",
                        value.type_name(),
                        child_path
                    )));
                }
            } else {
                state.with_config(|config| (desc.setter)(lua, config, value))?;
                state.mark_dirty(desc.dirty_flag);
                state.with_dirty_flags(|flags| flags.mark_dirty(&child_path));
            }
        } else if registry.is_nested(&child_path) {
            if let LuaValue::Table(nested_table) = value {
                apply_table_to_nested_path(lua, state, registry, &child_path, &nested_table)?;
            } else {
                return Err(LuaError::runtime(format!(
                    "cannot assign {} to nested config section '{}', expected table",
                    value.type_name(),
                    child_path
                )));
            }
        } else {
            return Err(LuaError::runtime(format!(
                "unknown config property: {}",
                child_path
            )));
        }
    }
    Ok(())
}
