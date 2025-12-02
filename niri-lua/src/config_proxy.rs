//! Configuration proxy system for reactive Lua configuration.
//!
//! This module implements the Phase R1 API refactor:
//! - `PendingConfigChanges` - Stores staged configuration changes
//! - Proxy tables with metatables for intercepting reads/writes
//! - Collection wrappers for CRUD operations
//! - `niri.config:apply()` and `:auto_apply()` methods

use std::collections::HashMap;
use std::sync::Arc;

use mlua::prelude::*;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Represents a pending configuration change.
/// Changes are stored as JSON-like structures that can be merged into the active config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingConfigChanges {
    /// Scalar section changes (input, layout, cursor, etc.)
    /// Key is the section path (e.g., "layout.gaps", "input.keyboard.repeat_rate")
    pub scalar_changes: HashMap<String, serde_json::Value>,

    /// Collection additions (binds, outputs, workspaces, window_rules, etc.)
    /// Key is the collection name, value is a list of items to add
    pub collection_additions: HashMap<String, Vec<serde_json::Value>>,

    /// Collection removals (match criteria for items to remove)
    /// Key is the collection name, value is a list of match criteria
    pub collection_removals: HashMap<String, Vec<serde_json::Value>>,

    /// Collection replacements (complete replacement of a collection)
    /// Key is the collection name, value is the new collection
    pub collection_replacements: HashMap<String, Vec<serde_json::Value>>,

    /// Whether auto-apply mode is enabled
    pub auto_apply: bool,
}

impl PendingConfigChanges {
    /// Create a new empty pending changes container
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any pending changes
    pub fn has_changes(&self) -> bool {
        !self.scalar_changes.is_empty()
            || !self.collection_additions.is_empty()
            || !self.collection_removals.is_empty()
            || !self.collection_replacements.is_empty()
    }

    /// Clear all pending changes
    pub fn clear(&mut self) {
        self.scalar_changes.clear();
        self.collection_additions.clear();
        self.collection_removals.clear();
        self.collection_replacements.clear();
    }

    /// Set a scalar value at the given path
    pub fn set_scalar(&mut self, path: &str, value: serde_json::Value) {
        self.scalar_changes.insert(path.to_string(), value);
    }

    /// Add items to a collection
    pub fn add_to_collection(&mut self, collection: &str, items: Vec<serde_json::Value>) {
        self.collection_additions
            .entry(collection.to_string())
            .or_default()
            .extend(items);
    }

    /// Remove items from a collection by match criteria
    pub fn remove_from_collection(&mut self, collection: &str, criteria: serde_json::Value) {
        self.collection_removals
            .entry(collection.to_string())
            .or_default()
            .push(criteria);
    }

    /// Replace an entire collection
    pub fn replace_collection(&mut self, collection: &str, items: Vec<serde_json::Value>) {
        self.collection_replacements
            .insert(collection.to_string(), items);
    }
}

/// Thread-safe handle to pending configuration changes
pub type SharedPendingChanges = Arc<Mutex<PendingConfigChanges>>;

/// Create a new shared pending changes handle
pub fn create_shared_pending_changes() -> SharedPendingChanges {
    Arc::new(Mutex::new(PendingConfigChanges::new()))
}

/// Configuration section types for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSection {
    /// Scalar sections with direct property access
    Scalar,
    /// Collection sections with CRUD operations
    Collection,
}

/// Get the section type for a given config key
pub fn get_section_type(key: &str) -> ConfigSection {
    match key {
        // Collection sections
        "binds" | "outputs" | "workspaces" | "window_rules" | "layer_rules" | "environment"
        | "switch_events" | "spawn_at_startup" => ConfigSection::Collection,
        // Everything else is scalar
        _ => ConfigSection::Scalar,
    }
}

/// Lua userdata for a config section proxy.
/// This enables the `niri.config.layout.gaps = 16` syntax.
#[derive(Clone)]
pub struct ConfigSectionProxy {
    /// The section path (e.g., "layout", "input.keyboard")
    pub path: String,
    /// Reference to pending changes
    pub pending: SharedPendingChanges,
    /// Cached current values (from the active config)
    pub current_values: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl LuaUserData for ConfigSectionProxy {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Enable table-like access via __index and __newindex
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            // First check pending changes
            let full_path = if this.path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", this.path, key)
            };

            let pending = this.pending.lock();
            if let Some(value) = pending.scalar_changes.get(&full_path) {
                return lua_value_from_json(lua, value);
            }
            drop(pending);

            // Check current values
            let current = this.current_values.lock();
            if let Some(value) = current.get(&key) {
                return lua_value_from_json(lua, value);
            }
            drop(current);

            // For nested sections, return a new proxy
            if get_section_type(&key) == ConfigSection::Scalar {
                let nested_proxy = ConfigSectionProxy {
                    path: full_path,
                    pending: this.pending.clone(),
                    current_values: Arc::new(Mutex::new(HashMap::new())),
                };
                return Ok(LuaValue::UserData(lua.create_userdata(nested_proxy)?));
            }

            Ok(LuaValue::Nil)
        });

        methods.add_meta_method(LuaMetaMethod::NewIndex, |lua, this, (key, value): (String, LuaValue)| {
            let full_path = if this.path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", this.path, key)
            };

            // Convert Lua value to JSON
            let json_value = lua_value_to_json(lua, &value)?;

            // Store in pending changes
            let mut pending = this.pending.lock();
            pending.set_scalar(&full_path, json_value);

            // Check for auto-apply
            if pending.auto_apply {
                drop(pending);
                // In a real implementation, this would trigger apply through the event loop
                // For now, we just mark it as needing apply
            }

            Ok(())
        });
    }
}

/// Lua userdata for a collection proxy.
/// This enables CRUD operations like `niri.config.binds:add({...})`.
#[derive(Clone)]
pub struct ConfigCollectionProxy {
    /// The collection name (e.g., "binds", "outputs")
    pub name: String,
    /// Reference to pending changes
    pub pending: SharedPendingChanges,
    /// Cached current items (from the active config)
    pub current_items: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl LuaUserData for ConfigCollectionProxy {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // list() - Return all items in the collection
        methods.add_method("list", |lua, this, ()| {
            let current = this.current_items.lock();
            let pending = this.pending.lock();

            // Combine current items with pending additions
            let mut items: Vec<serde_json::Value> = current.clone();

            // Add pending additions
            if let Some(additions) = pending.collection_additions.get(&this.name) {
                items.extend(additions.clone());
            }

            // If there's a replacement, use that instead
            if let Some(replacement) = pending.collection_replacements.get(&this.name) {
                items = replacement.clone();
            }

            // Convert to Lua table
            let result = lua.create_table()?;
            for (i, item) in items.iter().enumerate() {
                result.set(i + 1, lua_value_from_json(lua, item)?)?;
            }

            Ok(result)
        });

        // get(match_criteria) - Get items matching criteria
        methods.add_method("get", |lua, this, criteria: LuaTable| {
            let current = this.current_items.lock();
            let pending = this.pending.lock();

            // Convert match criteria to JSON
            let criteria_json = lua_table_to_json(lua, &criteria)?;

            // Combine current items with pending additions
            let mut items: Vec<serde_json::Value> = current.clone();
            if let Some(additions) = pending.collection_additions.get(&this.name) {
                items.extend(additions.clone());
            }
            if let Some(replacement) = pending.collection_replacements.get(&this.name) {
                items = replacement.clone();
            }

            // Filter items that match criteria
            let matched: Vec<_> = items
                .into_iter()
                .filter(|item| json_matches(&criteria_json, item))
                .collect();

            // Return as Lua table
            let result = lua.create_table()?;
            for (i, item) in matched.iter().enumerate() {
                result.set(i + 1, lua_value_from_json(lua, item)?)?;
            }

            Ok(result)
        });

        // add(items) - Add items to the collection
        methods.add_method("add", |lua, this, items: LuaValue| {
            let items_json = match items {
                LuaValue::Table(t) => {
                    // Check if it's an array or a single item
                    let first_val: LuaValue = t.get(1)?;
                    if matches!(first_val, LuaValue::Table(_)) {
                        // Array of items
                        lua_table_to_json_array(lua, &t)?
                    } else if first_val == LuaValue::Nil {
                        // Single item as table
                        vec![lua_table_to_json(lua, &t)?]
                    } else {
                        // Check if it has string keys (object-like)
                        let has_string_keys = t.pairs::<String, LuaValue>().next().is_some();
                        if has_string_keys {
                            vec![lua_table_to_json(lua, &t)?]
                        } else {
                            lua_table_to_json_array(lua, &t)?
                        }
                    }
                }
                _ => {
                    return Err(LuaError::external("add() expects a table"));
                }
            };

            let mut pending = this.pending.lock();
            pending.add_to_collection(&this.name, items_json);

            // Auto-apply check
            if pending.auto_apply {
                drop(pending);
                // Would trigger apply
            }

            Ok(())
        });

        // set(items) - Replace the entire collection
        methods.add_method("set", |lua, this, items: LuaTable| {
            let items_json = lua_table_to_json_array(lua, &items)?;

            let mut pending = this.pending.lock();
            pending.replace_collection(&this.name, items_json);

            if pending.auto_apply {
                drop(pending);
                // Would trigger apply
            }

            Ok(())
        });

        // remove(match_criteria) - Remove items matching criteria
        methods.add_method("remove", |lua, this, criteria: LuaTable| {
            let criteria_json = lua_table_to_json(lua, &criteria)?;

            let mut pending = this.pending.lock();
            pending.remove_from_collection(&this.name, criteria_json);

            if pending.auto_apply {
                drop(pending);
                // Would trigger apply
            }

            Ok(())
        });
    }
}

/// Main config proxy that provides the `niri.config` table.
#[derive(Clone)]
pub struct ConfigProxy {
    /// Reference to pending changes
    pub pending: SharedPendingChanges,
    /// Section proxies for scalar sections
    pub section_proxies: Arc<Mutex<HashMap<String, ConfigSectionProxy>>>,
    /// Collection proxies for collection sections
    pub collection_proxies: Arc<Mutex<HashMap<String, ConfigCollectionProxy>>>,
}

impl ConfigProxy {
    /// Create a new config proxy
    pub fn new(pending: SharedPendingChanges) -> Self {
        Self {
            pending,
            section_proxies: Arc::new(Mutex::new(HashMap::new())),
            collection_proxies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Initialize section proxies from the current config
    pub fn init_from_config(&self, config: &niri_config::Config) {
        // Populate section proxies with actual config values
        let mut section_proxies = self.section_proxies.lock();

        // Layout section
        let layout_values = config_to_layout_json(&config.layout);
        section_proxies.insert(
            "layout".to_string(),
            ConfigSectionProxy {
                path: "layout".to_string(),
                pending: self.pending.clone(),
                current_values: Arc::new(Mutex::new(layout_values)),
            },
        );

        // Input section
        let input_values = config_to_input_json(&config.input);
        section_proxies.insert(
            "input".to_string(),
            ConfigSectionProxy {
                path: "input".to_string(),
                pending: self.pending.clone(),
                current_values: Arc::new(Mutex::new(input_values)),
            },
        );

        // Cursor section
        let cursor_values = config_to_cursor_json(&config.cursor);
        section_proxies.insert(
            "cursor".to_string(),
            ConfigSectionProxy {
                path: "cursor".to_string(),
                pending: self.pending.clone(),
                current_values: Arc::new(Mutex::new(cursor_values)),
            },
        );

        // Animations section
        let animations_values = config_to_animations_json(&config.animations);
        section_proxies.insert(
            "animations".to_string(),
            ConfigSectionProxy {
                path: "animations".to_string(),
                pending: self.pending.clone(),
                current_values: Arc::new(Mutex::new(animations_values)),
            },
        );

        drop(section_proxies);

        // Populate collection proxies with actual config values
        let mut collection_proxies = self.collection_proxies.lock();

        // Binds collection
        let binds_items = config_to_binds_json(&config.binds);
        collection_proxies.insert(
            "binds".to_string(),
            ConfigCollectionProxy {
                name: "binds".to_string(),
                pending: self.pending.clone(),
                current_items: Arc::new(Mutex::new(binds_items)),
            },
        );

        // Window rules collection
        let window_rules_items = config_to_window_rules_json(&config.window_rules);
        collection_proxies.insert(
            "window_rules".to_string(),
            ConfigCollectionProxy {
                name: "window_rules".to_string(),
                pending: self.pending.clone(),
                current_items: Arc::new(Mutex::new(window_rules_items)),
            },
        );

        // Outputs collection
        let outputs_items = config_to_outputs_json(&config.outputs);
        collection_proxies.insert(
            "outputs".to_string(),
            ConfigCollectionProxy {
                name: "outputs".to_string(),
                pending: self.pending.clone(),
                current_items: Arc::new(Mutex::new(outputs_items)),
            },
        );

        // Workspaces collection
        let workspaces_items = config_to_workspaces_json(&config.workspaces);
        collection_proxies.insert(
            "workspaces".to_string(),
            ConfigCollectionProxy {
                name: "workspaces".to_string(),
                pending: self.pending.clone(),
                current_items: Arc::new(Mutex::new(workspaces_items)),
            },
        );

        // Spawn at startup collection
        let spawn_items: Vec<serde_json::Value> = config
            .spawn_at_startup
            .iter()
            .map(|s| serde_json::json!({ "command": s.command }))
            .collect();
        collection_proxies.insert(
            "spawn_at_startup".to_string(),
            ConfigCollectionProxy {
                name: "spawn_at_startup".to_string(),
                pending: self.pending.clone(),
                current_items: Arc::new(Mutex::new(spawn_items)),
            },
        );
    }
}

impl LuaUserData for ConfigProxy {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // __index for accessing config sections
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            let section_type = get_section_type(&key);

            match section_type {
                ConfigSection::Scalar => {
                    // Return a section proxy for scalar sections
                    let proxy = ConfigSectionProxy {
                        path: key.clone(),
                        pending: this.pending.clone(),
                        current_values: Arc::new(Mutex::new(HashMap::new())),
                    };
                    Ok(LuaValue::UserData(lua.create_userdata(proxy)?))
                }
                ConfigSection::Collection => {
                    // Return a collection proxy for collection sections
                    let proxy = ConfigCollectionProxy {
                        name: key.clone(),
                        pending: this.pending.clone(),
                        current_items: Arc::new(Mutex::new(Vec::new())),
                    };
                    Ok(LuaValue::UserData(lua.create_userdata(proxy)?))
                }
            }
        });

        // __newindex for setting entire sections
        methods.add_meta_method(LuaMetaMethod::NewIndex, |lua, this, (key, value): (String, LuaValue)| {
            // When setting an entire section like `niri.config.layout = { ... }`
            // We need to flatten it into individual scalar changes
            if let LuaValue::Table(t) = value {
                flatten_table_to_changes(lua, &this.pending, &key, &t)?;
            } else {
                // Single value assignment
                let json_value = lua_value_to_json(lua, &value)?;
                let mut pending = this.pending.lock();
                pending.set_scalar(&key, json_value);
            }

            Ok(())
        });

        // apply() method - apply all pending changes
        methods.add_method("apply", |_lua, this, ()| {
            let pending = this.pending.lock();
            if pending.has_changes() {
                // In the real implementation, this would send a message through the event loop
                // to apply the pending changes to the active config
                log::debug!("ConfigProxy::apply() called with {} scalar changes, {} collection additions",
                    pending.scalar_changes.len(),
                    pending.collection_additions.values().map(|v| v.len()).sum::<usize>());
            }
            Ok(())
        });

        // auto_apply(bool) method - enable/disable auto-apply mode
        methods.add_method("auto_apply", |_lua, this, enabled: bool| {
            let mut pending = this.pending.lock();
            pending.auto_apply = enabled;
            Ok(())
        });
    }
}

// ============================================================================
// Helper Functions for JSON <-> Lua conversion
// ============================================================================

/// Convert a Lua value to a JSON value
fn lua_value_to_json(lua: &Lua, value: &LuaValue) -> LuaResult<serde_json::Value> {
    match value {
        LuaValue::Nil => Ok(serde_json::Value::Null),
        LuaValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        LuaValue::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        LuaValue::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                Ok(serde_json::Value::Number(num))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        LuaValue::String(s) => Ok(serde_json::Value::String(s.to_string_lossy().to_string())),
        LuaValue::Table(t) => lua_table_to_json(lua, t),
        _ => Err(LuaError::external(format!(
            "Cannot convert {:?} to JSON",
            value
        ))),
    }
}

/// Convert a Lua table to a JSON object
fn lua_table_to_json(lua: &Lua, table: &LuaTable) -> LuaResult<serde_json::Value> {
    // Check if it's an array (has numeric keys starting from 1)
    let first: LuaValue = table.get(1)?;
    if first != LuaValue::Nil {
        // Likely an array
        let arr = lua_table_to_json_array(lua, table)?;
        return Ok(serde_json::Value::Array(arr));
    }

    // Object
    let mut map = serde_json::Map::new();
    for pair in table.pairs::<String, LuaValue>() {
        let (k, v) = pair?;
        map.insert(k, lua_value_to_json(lua, &v)?);
    }
    Ok(serde_json::Value::Object(map))
}

/// Convert a Lua array table to a JSON array
fn lua_table_to_json_array(lua: &Lua, table: &LuaTable) -> LuaResult<Vec<serde_json::Value>> {
    let mut arr = Vec::new();
    let mut i = 1;
    loop {
        let val: LuaValue = table.get(i)?;
        if val == LuaValue::Nil {
            break;
        }
        arr.push(lua_value_to_json(lua, &val)?);
        i += 1;
    }
    Ok(arr)
}

/// Convert a JSON value to a Lua value
fn lua_value_from_json(lua: &Lua, json: &serde_json::Value) -> LuaResult<LuaValue> {
    match json {
        serde_json::Value::Null => Ok(LuaValue::Nil),
        serde_json::Value::Bool(b) => Ok(LuaValue::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LuaValue::Number(f))
            } else {
                Ok(LuaValue::Nil)
            }
        }
        serde_json::Value::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, val) in arr.iter().enumerate() {
                table.set(i + 1, lua_value_from_json(lua, val)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (k, v) in obj.iter() {
                table.set(k.as_str(), lua_value_from_json(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

/// Check if a string looks like a regex pattern
/// (contains regex metacharacters that aren't commonly found in plain strings)
fn looks_like_regex(s: &str) -> bool {
    // These indicate the string is likely a regex pattern
    s.starts_with('^')
        || s.ends_with('$')
        || s.contains(".*")
        || s.contains(".+")
        || s.contains("\\d")
        || s.contains("\\w")
        || s.contains("\\s")
        || s.contains("[^")
        || s.contains("(?")
}

/// Check if a JSON value matches criteria
fn json_matches(criteria: &serde_json::Value, item: &serde_json::Value) -> bool {
    match (criteria, item) {
        (serde_json::Value::Object(crit), serde_json::Value::Object(obj)) => {
            // All criteria fields must match
            for (key, expected) in crit.iter() {
                match obj.get(key) {
                    Some(actual) => {
                        // For strings, support regex matching when pattern looks like regex
                        if let (serde_json::Value::String(exp), serde_json::Value::String(act)) =
                            (expected, actual)
                        {
                            if looks_like_regex(exp) {
                                // Try regex matching
                                if let Ok(re) = regex::Regex::new(exp) {
                                    if !re.is_match(act) {
                                        return false;
                                    }
                                } else if exp != act {
                                    // Invalid regex, fall back to exact match
                                    return false;
                                }
                            } else if exp != act {
                                // Plain string comparison
                                return false;
                            }
                        } else if expected != actual {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        }
        _ => criteria == item,
    }
}

/// Flatten a nested Lua table into scalar changes
fn flatten_table_to_changes(
    lua: &Lua,
    pending: &SharedPendingChanges,
    prefix: &str,
    table: &LuaTable,
) -> LuaResult<()> {
    for pair in table.pairs::<String, LuaValue>() {
        let (k, v) = pair?;
        let path = format!("{}.{}", prefix, k);

        match v {
            LuaValue::Table(ref t) => {
                // Check if it's an array or nested object
                let first: LuaValue = t.get(1)?;
                if first != LuaValue::Nil {
                    // Array - store as JSON array
                    let json_value = lua_value_to_json(lua, &v)?;
                    let mut p = pending.lock();
                    p.set_scalar(&path, json_value);
                } else {
                    // Nested object - recurse
                    flatten_table_to_changes(lua, pending, &path, t)?;
                }
            }
            _ => {
                let json_value = lua_value_to_json(lua, &v)?;
                let mut p = pending.lock();
                p.set_scalar(&path, json_value);
            }
        }
    }
    Ok(())
}

// ============================================================================
// Config to JSON conversion helpers
// ============================================================================

/// Convert Layout config to JSON HashMap
fn config_to_layout_json(layout: &niri_config::Layout) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert("gaps".to_string(), serde_json::json!(layout.gaps));
    map.insert(
        "center_focused_column".to_string(),
        serde_json::json!(format!("{:?}", layout.center_focused_column)),
    );
    map.insert(
        "always_center_single_column".to_string(),
        serde_json::json!(layout.always_center_single_column),
    );
    map.insert(
        "empty_workspace_above_first".to_string(),
        serde_json::json!(layout.empty_workspace_above_first),
    );
    map.insert(
        "default_column_display".to_string(),
        serde_json::json!(format!("{:?}", layout.default_column_display)),
    );

    // Struts
    map.insert(
        "struts".to_string(),
        serde_json::json!({
            "left": layout.struts.left.0,
            "right": layout.struts.right.0,
            "top": layout.struts.top.0,
            "bottom": layout.struts.bottom.0,
        }),
    );

    // Focus ring
    map.insert(
        "focus_ring".to_string(),
        serde_json::json!({
            "off": layout.focus_ring.off,
            "width": layout.focus_ring.width,
        }),
    );

    // Border
    map.insert(
        "border".to_string(),
        serde_json::json!({
            "off": layout.border.off,
            "width": layout.border.width,
        }),
    );

    // Shadow
    map.insert(
        "shadow".to_string(),
        serde_json::json!({
            "on": layout.shadow.on,
            "softness": layout.shadow.softness,
            "spread": layout.shadow.spread,
        }),
    );

    map
}

/// Convert Input config to JSON HashMap
fn config_to_input_json(input: &niri_config::Input) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();

    // Keyboard settings
    map.insert(
        "keyboard".to_string(),
        serde_json::json!({
            "repeat_delay": input.keyboard.repeat_delay,
            "repeat_rate": input.keyboard.repeat_rate,
            "track_layout": format!("{:?}", input.keyboard.track_layout),
            "numlock": input.keyboard.numlock,
        }),
    );

    // Touchpad settings
    map.insert(
        "touchpad".to_string(),
        serde_json::json!({
            "off": input.touchpad.off,
            "tap": input.touchpad.tap,
            "dwt": input.touchpad.dwt,
            "dwtp": input.touchpad.dwtp,
            "natural_scroll": input.touchpad.natural_scroll,
            "accel_speed": input.touchpad.accel_speed.0,
        }),
    );

    // Mouse settings
    map.insert(
        "mouse".to_string(),
        serde_json::json!({
            "off": input.mouse.off,
            "natural_scroll": input.mouse.natural_scroll,
            "accel_speed": input.mouse.accel_speed.0,
        }),
    );

    map.insert(
        "disable_power_key_handling".to_string(),
        serde_json::json!(input.disable_power_key_handling),
    );
    map.insert(
        "workspace_auto_back_and_forth".to_string(),
        serde_json::json!(input.workspace_auto_back_and_forth),
    );

    map
}

/// Convert Cursor config to JSON HashMap
fn config_to_cursor_json(cursor: &niri_config::Cursor) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert(
        "xcursor_theme".to_string(),
        serde_json::json!(cursor.xcursor_theme),
    );
    map.insert(
        "xcursor_size".to_string(),
        serde_json::json!(cursor.xcursor_size),
    );
    map.insert(
        "hide_when_typing".to_string(),
        serde_json::json!(cursor.hide_when_typing),
    );
    if let Some(ms) = cursor.hide_after_inactive_ms {
        map.insert(
            "hide_after_inactive_ms".to_string(),
            serde_json::json!(ms),
        );
    }
    map
}

/// Convert Animations config to JSON HashMap
fn config_to_animations_json(
    animations: &niri_config::Animations,
) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert("off".to_string(), serde_json::json!(animations.off));
    map.insert(
        "slowdown".to_string(),
        serde_json::json!(animations.slowdown),
    );
    map
}

/// Convert Binds to JSON array
fn config_to_binds_json(binds: &niri_config::Binds) -> Vec<serde_json::Value> {
    binds
        .0
        .iter()
        .map(|bind| {
            serde_json::json!({
                "key": format!("{:?}", bind.key),
                "action": format!("{:?}", bind.action),
                "repeat": bind.repeat,
                "allow_when_locked": bind.allow_when_locked,
                "allow_inhibiting": bind.allow_inhibiting,
            })
        })
        .collect()
}

/// Convert WindowRules to JSON array
fn config_to_window_rules_json(rules: &[niri_config::WindowRule]) -> Vec<serde_json::Value> {
    rules
        .iter()
        .map(|rule| {
            let mut obj = serde_json::Map::new();

            // Matches
            if !rule.matches.is_empty() {
                let matches: Vec<_> = rule
                    .matches
                    .iter()
                    .map(|m| {
                        let mut match_obj = serde_json::Map::new();
                        if let Some(ref app_id) = m.app_id {
                            match_obj
                                .insert("app_id".to_string(), serde_json::json!(app_id.0.as_str()));
                        }
                        if let Some(ref title) = m.title {
                            match_obj
                                .insert("title".to_string(), serde_json::json!(title.0.as_str()));
                        }
                        serde_json::Value::Object(match_obj)
                    })
                    .collect();
                obj.insert("matches".to_string(), serde_json::Value::Array(matches));
            }

            // Common rule properties
            if let Some(ref output) = rule.open_on_output {
                obj.insert("open_on_output".to_string(), serde_json::json!(output));
            }
            if let Some(maximized) = rule.open_maximized {
                obj.insert("open_maximized".to_string(), serde_json::json!(maximized));
            }
            if let Some(fullscreen) = rule.open_fullscreen {
                obj.insert("open_fullscreen".to_string(), serde_json::json!(fullscreen));
            }
            if let Some(floating) = rule.open_floating {
                obj.insert("open_floating".to_string(), serde_json::json!(floating));
            }

            serde_json::Value::Object(obj)
        })
        .collect()
}

/// Convert Outputs to JSON array
fn config_to_outputs_json(outputs: &niri_config::Outputs) -> Vec<serde_json::Value> {
    outputs
        .0
        .iter()
        .map(|output| {
            let mut obj = serde_json::Map::new();
            obj.insert("name".to_string(), serde_json::json!(output.name));
            obj.insert("off".to_string(), serde_json::json!(output.off));

            if let Some(ref scale) = output.scale {
                obj.insert("scale".to_string(), serde_json::json!(scale.0));
            }
            obj.insert(
                "transform".to_string(),
                serde_json::json!(format!("{:?}", output.transform)),
            );
            if let Some(ref pos) = output.position {
                obj.insert(
                    "position".to_string(),
                    serde_json::json!({"x": pos.x, "y": pos.y}),
                );
            }
            obj.insert(
                "focus_at_startup".to_string(),
                serde_json::json!(output.focus_at_startup),
            );

            serde_json::Value::Object(obj)
        })
        .collect()
}

/// Convert Workspaces to JSON array
fn config_to_workspaces_json(workspaces: &[niri_config::Workspace]) -> Vec<serde_json::Value> {
    workspaces
        .iter()
        .map(|ws| {
            let mut obj = serde_json::Map::new();
            obj.insert("name".to_string(), serde_json::json!(&ws.name.0));
            if let Some(ref output) = ws.open_on_output {
                obj.insert("open_on_output".to_string(), serde_json::json!(output));
            }
            serde_json::Value::Object(obj)
        })
        .collect()
}

/// Register the config proxy API to Lua
pub fn register_config_proxy_to_lua(
    lua: &Lua,
    pending: SharedPendingChanges,
    config: &niri_config::Config,
) -> LuaResult<()> {
    let globals = lua.globals();

    // Get or create the niri table
    let niri_table: LuaTable = globals
        .get("niri")
        .unwrap_or_else(|_| lua.create_table().unwrap());

    // Create the config proxy
    let proxy = ConfigProxy::new(pending);
    proxy.init_from_config(config);

    // Set niri.config as a userdata with metatable
    niri_table.set("config", proxy)?;
    globals.set("niri", niri_table)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_config_changes_basic() {
        let mut pending = PendingConfigChanges::new();
        assert!(!pending.has_changes());

        pending.set_scalar("layout.gaps", serde_json::json!(16));
        assert!(pending.has_changes());
        assert_eq!(
            pending.scalar_changes.get("layout.gaps"),
            Some(&serde_json::json!(16))
        );

        pending.clear();
        assert!(!pending.has_changes());
    }

    #[test]
    fn pending_config_changes_collections() {
        let mut pending = PendingConfigChanges::new();

        pending.add_to_collection(
            "binds",
            vec![serde_json::json!({
                "key": "Mod+T",
                "action": "spawn",
                "args": ["kitty"]
            })],
        );

        assert!(pending.has_changes());
        assert_eq!(pending.collection_additions.get("binds").unwrap().len(), 1);

        pending.remove_from_collection("binds", serde_json::json!({"key": "Mod+T"}));
        assert_eq!(pending.collection_removals.get("binds").unwrap().len(), 1);
    }

    #[test]
    fn json_matches_simple() {
        let criteria = serde_json::json!({"key": "Mod+T"});
        let item = serde_json::json!({"key": "Mod+T", "action": "spawn"});

        assert!(json_matches(&criteria, &item));

        let non_matching = serde_json::json!({"key": "Mod+Q"});
        assert!(!json_matches(&criteria, &non_matching));
    }

    #[test]
    fn json_matches_regex() {
        let criteria = serde_json::json!({"app_id": "^firefox"});
        let item = serde_json::json!({"app_id": "firefox-developer", "title": "Test"});

        assert!(json_matches(&criteria, &item));

        let non_matching = serde_json::json!({"app_id": "chrome"});
        assert!(!json_matches(&criteria, &non_matching));
    }

    #[test]
    fn config_section_type() {
        assert_eq!(get_section_type("layout"), ConfigSection::Scalar);
        assert_eq!(get_section_type("input"), ConfigSection::Scalar);
        assert_eq!(get_section_type("binds"), ConfigSection::Collection);
        assert_eq!(get_section_type("outputs"), ConfigSection::Collection);
        assert_eq!(get_section_type("window_rules"), ConfigSection::Collection);
    }

    #[test]
    fn lua_json_roundtrip() {
        let lua = Lua::new();

        // Test basic types
        let json_int = serde_json::json!(42);
        let lua_int = lua_value_from_json(&lua, &json_int).unwrap();
        let back = lua_value_to_json(&lua, &lua_int).unwrap();
        assert_eq!(json_int, back);

        // Test string
        let json_str = serde_json::json!("hello");
        let lua_str = lua_value_from_json(&lua, &json_str).unwrap();
        let back = lua_value_to_json(&lua, &lua_str).unwrap();
        assert_eq!(json_str, back);

        // Test object
        let json_obj = serde_json::json!({"foo": "bar", "baz": 123});
        let lua_obj = lua_value_from_json(&lua, &json_obj).unwrap();
        let back = lua_value_to_json(&lua, &lua_obj).unwrap();
        assert_eq!(json_obj, back);
    }

    #[test]
    fn config_proxy_apply() {
        let lua = Lua::new();
        let pending = create_shared_pending_changes();

        let proxy = ConfigProxy::new(pending.clone());
        let ud = lua.create_userdata(proxy).unwrap();

        // Set it as niri.config
        let niri = lua.create_table().unwrap();
        niri.set("config", ud).unwrap();
        lua.globals().set("niri", niri).unwrap();

        // Test apply method
        lua.load("niri.config:apply()").exec().unwrap();
    }

    #[test]
    fn config_proxy_auto_apply() {
        let lua = Lua::new();
        let pending = create_shared_pending_changes();

        let proxy = ConfigProxy::new(pending.clone());
        let ud = lua.create_userdata(proxy).unwrap();

        let niri = lua.create_table().unwrap();
        niri.set("config", ud).unwrap();
        lua.globals().set("niri", niri).unwrap();

        // Test auto_apply method
        lua.load("niri.config:auto_apply(true)").exec().unwrap();

        let p = pending.lock();
        assert!(p.auto_apply);
    }

    #[test]
    fn collection_proxy_add() {
        let lua = Lua::new();
        let pending = create_shared_pending_changes();

        let proxy = ConfigCollectionProxy {
            name: "binds".to_string(),
            pending: pending.clone(),
            current_items: Arc::new(Mutex::new(Vec::new())),
        };

        let ud = lua.create_userdata(proxy).unwrap();
        lua.globals().set("binds", ud).unwrap();

        // Test add method with single item
        lua.load(r#"binds:add({ key = "Mod+T", action = "spawn" })"#)
            .exec()
            .unwrap();

        let p = pending.lock();
        assert_eq!(p.collection_additions.get("binds").unwrap().len(), 1);
    }

    #[test]
    fn collection_proxy_list() {
        let lua = Lua::new();
        let pending = create_shared_pending_changes();

        // Pre-populate with some items
        let current_items = vec![
            serde_json::json!({"key": "Mod+Q", "action": "close-window"}),
            serde_json::json!({"key": "Mod+T", "action": "spawn"}),
        ];

        let proxy = ConfigCollectionProxy {
            name: "binds".to_string(),
            pending: pending.clone(),
            current_items: Arc::new(Mutex::new(current_items)),
        };

        let ud = lua.create_userdata(proxy).unwrap();
        lua.globals().set("binds", ud).unwrap();

        // Test list method
        let result: i64 = lua.load("return #binds:list()").eval().unwrap();
        assert_eq!(result, 2);
    }
}
