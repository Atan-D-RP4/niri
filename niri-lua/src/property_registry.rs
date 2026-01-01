//! Property registry for unified config API.
//!
//! This module provides a central registry mapping config paths to property descriptors
//! with getter/setter functions. It replaces the 40+ generated proxy structs with a
//! unified dynamic property system.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use mlua::prelude::*;
use niri_config::Config;
use niri_lua_traits::PropertyRegistryMut;

use crate::config_state::DirtyFlag;

/// Global registry using OnceLock (safe, immutable after init).
static REGISTRY: OnceLock<PropertyRegistry> = OnceLock::new();

impl PropertyRegistryMut for PropertyRegistry {
    fn add_from_metadata(&mut self, metadata: &niri_lua_traits::PropertyMetadata) {
        let path = metadata.path;
        if self.properties.contains_key(path) {
            return;
        }

        let prop_type = convert_property_type(&metadata.ty);
        let dirty_flag = infer_dirty_flag(path);

        let descriptor = PropertyDescriptor {
            path,
            ty: prop_type,
            dirty_flag,
            getter: create_placeholder_getter(),
            setter: create_placeholder_setter(),
            signal: !metadata.no_signal,
        };
        self.add(descriptor);
    }
}

/// Describes the type of a config property for validation and documentation.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyType {
    /// Boolean value.
    Bool,
    /// Integer value (i8-i64, u8-u64).
    Integer,
    /// Floating-point number (f32, f64).
    Number,
    /// String value.
    String,
    /// Enum with known variants (case-sensitive string matching).
    Enum {
        /// Name of the enum type for error messages.
        name: &'static str,
        /// Valid variant names.
        variants: &'static [&'static str],
    },
    /// Array of elements with a specific type.
    Array(Box<PropertyType>),
    /// Nested struct that can be indexed into.
    Nested,
}

impl PropertyType {
    /// Returns a human-readable type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            PropertyType::Bool => "boolean",
            PropertyType::Integer => "integer",
            PropertyType::Number => "number",
            PropertyType::String => "string",
            PropertyType::Enum { name, .. } => name,
            PropertyType::Array(_) => "array",
            PropertyType::Nested => "table",
        }
    }
}

/// Type alias for property getter function.
///
/// Receives Lua context and config reference, returns Lua value.
pub type PropertyGetter = fn(&Lua, &Config) -> LuaResult<LuaValue>;

/// Type alias for property setter function.
///
/// Receives Lua context, mutable config reference, and value to set.
pub type PropertySetter = fn(&Lua, &mut Config, LuaValue) -> LuaResult<()>;

/// Describes a single config property with getter/setter functions.
#[derive(Clone)]
pub struct PropertyDescriptor {
    /// Full dot-separated path (e.g., "cursor.xcursor_size").
    pub path: &'static str,
    /// Property type for validation and documentation.
    pub ty: PropertyType,
    /// Which dirty flag to set when this property changes.
    pub dirty_flag: DirtyFlag,
    /// Function to get the property value from config.
    pub getter: PropertyGetter,
    /// Function to set the property value in config.
    pub setter: PropertySetter,
    /// Whether to emit a signal when this property changes.
    pub signal: bool,
}

impl PropertyDescriptor {
    /// Create a new property descriptor with signal emission enabled.
    pub const fn new(
        path: &'static str,
        ty: PropertyType,
        dirty_flag: DirtyFlag,
        getter: PropertyGetter,
        setter: PropertySetter,
    ) -> Self {
        Self {
            path,
            ty,
            dirty_flag,
            getter,
            setter,
            signal: true,
        }
    }

    /// Create a descriptor for a nested struct (not directly assignable).
    pub fn nested(path: &'static str, dirty_flag: DirtyFlag) -> Self {
        Self {
            path,
            ty: PropertyType::Nested,
            dirty_flag,
            getter: |_, _| Ok(LuaValue::Nil),
            setter: |_, _, _| {
                Err(LuaError::external(
                    "cannot assign directly to nested config section",
                ))
            },
            signal: false,
        }
    }

    /// Disable signal emission for this property.
    #[must_use]
    pub const fn no_signal(mut self) -> Self {
        self.signal = false;
        self
    }
}

impl std::fmt::Debug for PropertyDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyDescriptor")
            .field("path", &self.path)
            .field("ty", &self.ty)
            .field("dirty_flag", &self.dirty_flag)
            .field("signal", &self.signal)
            .finish()
    }
}

/// Central registry for all config properties.
///
/// The registry is immutable after initialization and uses `OnceLock` for
/// thread-safe static access. Properties are stored in a `BTreeMap` for
/// ordered iteration.
#[derive(Debug, Default)]
pub struct PropertyRegistry {
    properties: BTreeMap<String, PropertyDescriptor>,
}

impl PropertyRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            properties: BTreeMap::new(),
        }
    }

    /// Add a property descriptor to the registry.
    ///
    /// # Panics
    ///
    /// Panics if a property with the same path already exists (programming error).
    pub fn add(&mut self, descriptor: PropertyDescriptor) {
        let path = descriptor.path.to_string();
        if self.properties.contains_key(&path) {
            panic!(
                "PropertyRegistry: duplicate property path '{}' - this is a bug",
                path
            );
        }
        self.properties.insert(path, descriptor);
    }

    /// Get a property descriptor by path.
    pub fn get(&self, path: &str) -> Option<&PropertyDescriptor> {
        self.properties.get(path)
    }

    /// Check if a property exists at the given path.
    pub fn contains(&self, path: &str) -> bool {
        self.properties.contains_key(path)
    }

    /// Returns iterator over direct child property keys for a given prefix.
    ///
    /// For example, if prefix is "cursor", returns keys like "xcursor_size",
    /// "xcursor_theme", etc. (without the "cursor." prefix).
    pub fn children<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        let prefix_dot = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}.", prefix)
        };
        let prefix_len = prefix_dot.len();
        let is_root = prefix.is_empty();

        self.properties
            .keys()
            .filter(move |k| {
                if is_root {
                    // Root level: return first segment of each path
                    !k.contains('.')
                        || self
                            .properties
                            .contains_key(k.split('.').next().unwrap_or(""))
                } else {
                    k.starts_with(&prefix_dot) && !k[prefix_len..].contains('.')
                }
            })
            .map(move |k| {
                if is_root {
                    k.split('.').next().unwrap_or(k)
                } else {
                    &k[prefix_len..]
                }
            })
    }

    /// Returns iterator over all unique direct child keys for a given prefix.
    ///
    /// Unlike `children()`, this deduplicates nested paths to return only
    /// immediate children (both leaf properties and nested struct prefixes).
    pub fn child_keys(&self, prefix: &str) -> Vec<String> {
        let prefix_dot = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}.", prefix)
        };
        let prefix_len = prefix_dot.len();

        let mut keys: Vec<String> = self
            .properties
            .keys()
            .filter_map(|k| {
                if prefix.is_empty() {
                    // Root level: extract first segment
                    Some(k.split('.').next().unwrap_or(k).to_string())
                } else if k.starts_with(&prefix_dot) {
                    // Has our prefix: extract next segment
                    let rest = &k[prefix_len..];
                    Some(rest.split('.').next().unwrap_or(rest).to_string())
                } else {
                    None
                }
            })
            .collect();

        keys.sort();
        keys.dedup();
        keys
    }

    /// Check if a path represents a nested struct (has child properties).
    pub fn is_nested(&self, path: &str) -> bool {
        let prefix = format!("{}.", path);
        self.properties.keys().any(|k| k.starts_with(&prefix))
    }

    /// Get the number of registered properties.
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Returns an iterator over all property paths and descriptors.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PropertyDescriptor)> {
        self.properties.iter()
    }

    pub fn update_accessor(
        &mut self,
        path: &'static str,
        getter: PropertyGetter,
        setter: PropertySetter,
    ) {
        self.update_accessor_with_type(path, PropertyType::Nested, getter, setter);
    }

    pub fn update_accessor_with_type(
        &mut self,
        path: &'static str,
        ty: PropertyType,
        getter: PropertyGetter,
        setter: PropertySetter,
    ) {
        if let Some(desc) = self.properties.get_mut(path) {
            desc.getter = getter;
            desc.setter = setter;
        } else {
            let dirty_flag = infer_dirty_flag(path);
            self.properties.insert(
                path.to_string(),
                PropertyDescriptor::new(path, ty, dirty_flag, getter, setter),
            );
        }
    }

    /// Initialize the global registry.
    ///
    /// This should be called once at startup. The provided function should
    /// register all config properties.
    ///
    /// # Returns
    ///
    /// Reference to the initialized global registry.
    pub fn init_with<F>(init_fn: F) -> &'static PropertyRegistry
    where
        F: FnOnce(&mut PropertyRegistry),
    {
        REGISTRY.get_or_init(|| {
            let mut registry = PropertyRegistry::new();
            init_fn(&mut registry);
            registry
        })
    }

    /// Get the global registry.
    ///
    /// # Panics
    ///
    /// Panics if the registry has not been initialized.
    pub fn global() -> &'static PropertyRegistry {
        REGISTRY
            .get()
            .expect("PropertyRegistry not initialized - call init_with() first")
    }

    /// Try to get the global registry, returning None if not initialized.
    pub fn try_global() -> Option<&'static PropertyRegistry> {
        REGISTRY.get()
    }

    /// Initialize the global registry with properties from all config structs.
    ///
    /// This registers properties from all structs that implement `ConfigProperties`.
    /// Call this once at Lua runtime startup.
    pub fn init_from_config() -> &'static PropertyRegistry {
        use niri_config::appearance::Shadow;
        use niri_config::input::{
            Keyboard, Mouse, Tablet, Touch, Touchpad, Trackball, Trackpoint, Xkb,
        };
        use niri_config::{Animations, Cursor, Input, Layout};
        use niri_lua_traits::ConfigProperties;

        Self::init_with(|registry| {
            for metadata in Layout::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Input::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Keyboard::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Xkb::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Touchpad::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Touch::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Mouse::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Trackpoint::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Trackball::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Tablet::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Animations::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Cursor::property_metadata() {
                registry.add_from_metadata(&metadata);
            }
            for metadata in Shadow::property_metadata() {
                registry.add_from_metadata(&metadata);
            }

            crate::config_accessors::register_config_accessors(registry);
        })
    }
}

/// Infer the appropriate DirtyFlag from a config path prefix.
///
/// This allows automatic dirty flag assignment based on the config path,
/// reducing boilerplate in property registration.
pub fn infer_dirty_flag(path: &str) -> DirtyFlag {
    let prefix = path.split('.').next().unwrap_or(path);
    match prefix {
        "cursor" => DirtyFlag::Cursor,
        "layout" => DirtyFlag::Layout,
        "animations" => DirtyFlag::Animations,
        "input" => DirtyFlag::Input,
        "gestures" => DirtyFlag::Gestures,
        "overview" => DirtyFlag::Overview,
        "recent_windows" => DirtyFlag::RecentWindows,
        "clipboard" => DirtyFlag::Clipboard,
        "hotkey_overlay" => DirtyFlag::HotkeyOverlay,
        "config_notification" => DirtyFlag::ConfigNotification,
        "debug" => DirtyFlag::Debug,
        "xwayland_satellite" => DirtyFlag::XwaylandSatellite,
        "window_rules" => DirtyFlag::WindowRules,
        "layer_rules" => DirtyFlag::LayerRules,
        "binds" => DirtyFlag::Binds,
        "workspaces" => DirtyFlag::Workspaces,
        "environment" => DirtyFlag::Environment,
        "spawn_at_startup" => DirtyFlag::SpawnAtStartup,
        "outputs" => DirtyFlag::Outputs,
        "keyboard" => DirtyFlag::Keyboard,
        _ => DirtyFlag::Misc,
    }
}

/// Parse an enum variant from a Lua string value.
///
/// Returns a clear error message listing valid variants on failure.
pub fn parse_enum_variant<T>(value: &str, variants: &[&str], enum_name: &str) -> LuaResult<T>
where
    T: std::str::FromStr,
{
    value.parse().map_err(|_| {
        LuaError::external(format!(
            "invalid {} value \"{}\", expected one of: {}",
            enum_name,
            value,
            variants.join(", ")
        ))
    })
}

/// Validate array elements and convert to Vec.
///
/// Iterates over Lua table entries, validating and converting each element.
/// Returns a clear error message with the index on element validation failure.
pub fn validate_array_elements<T, F>(lua: &Lua, table: &LuaTable, convert: F) -> LuaResult<Vec<T>>
where
    F: Fn(&Lua, LuaValue) -> LuaResult<T>,
{
    let mut vec = Vec::new();
    for pair in table.clone().pairs::<i64, LuaValue>() {
        let (idx, value) = pair?;
        let elem = convert(lua, value)
            .map_err(|e| LuaError::external(format!("invalid element at index {}: {}", idx, e)))?;
        vec.push(elem);
    }
    Ok(vec)
}

/// Helper to create a type error with expected type name.
pub fn type_error(expected: &str, got: &LuaValue) -> LuaError {
    let got_type = match got {
        LuaValue::Nil => "nil",
        LuaValue::Boolean(_) => "boolean",
        LuaValue::Integer(_) => "integer",
        LuaValue::Number(_) => "number",
        LuaValue::String(_) => "string",
        LuaValue::Table(_) => "table",
        LuaValue::Function(_) => "function",
        LuaValue::Thread(_) => "thread",
        LuaValue::UserData(_) => "userdata",
        LuaValue::LightUserData(_) => "lightuserdata",
        LuaValue::Error(_) => "error",
        _ => "unknown",
    };
    LuaError::external(format!("expected {}, got {}", expected, got_type))
}

/// Helper to extract integer with range checking.
pub fn extract_integer<T>(value: &LuaValue, type_name: &str) -> LuaResult<T>
where
    T: TryFrom<i64> + std::fmt::Display,
    <T as TryFrom<i64>>::Error: std::fmt::Display,
{
    let i = value
        .as_integer()
        .ok_or_else(|| type_error("integer", value))?;
    T::try_from(i).map_err(|e| {
        LuaError::external(format!("value {} out of range for {}: {}", i, type_name, e))
    })
}

/// Helper to extract number (float).
pub fn extract_number(value: &LuaValue) -> LuaResult<f64> {
    value
        .as_number()
        .or_else(|| value.as_integer().map(|i| i as f64))
        .ok_or_else(|| type_error("number", value))
}

/// Helper to extract string.
pub fn extract_string(value: &LuaValue) -> LuaResult<String> {
    value
        .as_string()
        .and_then(|s| s.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| type_error("string", value))
}

/// Helper to extract boolean.
pub fn extract_bool(value: &LuaValue) -> LuaResult<bool> {
    value
        .as_boolean()
        .ok_or_else(|| type_error("boolean", value))
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn convert_property_type(ty: &niri_lua_traits::PropertyType) -> PropertyType {
    match ty {
        niri_lua_traits::PropertyType::Bool => PropertyType::Bool,
        niri_lua_traits::PropertyType::Integer => PropertyType::Integer,
        niri_lua_traits::PropertyType::Number => PropertyType::Number,
        niri_lua_traits::PropertyType::String => PropertyType::String,
        niri_lua_traits::PropertyType::Enum { name, variants } => {
            PropertyType::Enum { name, variants }
        }
        niri_lua_traits::PropertyType::Array(inner) => {
            PropertyType::Array(Box::new(convert_property_type(inner)))
        }
        niri_lua_traits::PropertyType::Nested => PropertyType::Nested,
    }
}

fn create_placeholder_getter() -> PropertyGetter {
    |_lua, _config| Ok(LuaValue::Nil)
}

fn create_placeholder_setter() -> PropertySetter {
    |_lua, _config, _value| Ok(())
}

#[macro_export]
macro_rules! register_accessor {
    ($registry:expr, $path:literal, $get:expr, $set:expr) => {{
        let getter: $crate::property_registry::PropertyGetter = |lua, config| {
            let value = $get(config);
            lua.to_value(&value)
        };
        let setter: $crate::property_registry::PropertySetter = |lua, config, value| {
            let converted = lua.from_value(value)?;
            $set(config, converted);
            Ok(())
        };
        $registry.update_accessor($path, getter, setter);
    }};
}

#[macro_export]
macro_rules! register_option_accessor {
    ($registry:expr, $path:literal, $get:expr, $set:expr) => {{
        let getter: $crate::property_registry::PropertyGetter = |lua, config| match $get(config) {
            Some(v) => lua.to_value(&v),
            None => Ok(mlua::Value::Nil),
        };
        let setter: $crate::property_registry::PropertySetter = |lua, config, value| {
            if value.is_nil() {
                $set(config, None);
            } else {
                let converted = lua.from_value(value)?;
                $set(config, Some(converted));
            }
            Ok(())
        };
        $registry.update_accessor($path, getter, setter);
    }};
}

#[macro_export]
macro_rules! register_enum_accessor {
    ($registry:expr, $path:literal, $get:expr, $set:expr, $enum_ty:ty) => {{
        let getter: $crate::property_registry::PropertyGetter = |lua, config| {
            let value: $enum_ty = $get(config);
            let s = format!("{:?}", value);
            let snake = $crate::property_registry::to_snake_case(&s);
            lua.create_string(&snake).map(mlua::Value::String)
        };
        let setter: $crate::property_registry::PropertySetter = |_lua, config, value| {
            let s = $crate::property_registry::extract_string(&value)?;
            let pascal = $crate::property_registry::to_pascal_case(&s);
            let parsed: $enum_ty = pascal
                .parse()
                .map_err(|_| mlua::Error::external(format!("invalid enum value: {}", s)))?;
            $set(config, parsed);
            Ok(())
        };
        $registry.update_accessor($path, getter, setter);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_getter(_lua: &Lua, _config: &Config) -> LuaResult<LuaValue> {
        Ok(LuaValue::Nil)
    }

    fn dummy_setter(_lua: &Lua, _config: &mut Config, _value: LuaValue) -> LuaResult<()> {
        Ok(())
    }

    #[test]
    fn test_property_registry_add_and_get() {
        let mut registry = PropertyRegistry::new();

        registry.add(PropertyDescriptor::new(
            "cursor.xcursor_size",
            PropertyType::Integer,
            DirtyFlag::Cursor,
            dummy_getter,
            dummy_setter,
        ));

        assert!(registry.contains("cursor.xcursor_size"));
        assert!(!registry.contains("cursor.nonexistent"));

        let desc = registry.get("cursor.xcursor_size").unwrap();
        assert_eq!(desc.path, "cursor.xcursor_size");
        assert_eq!(desc.ty, PropertyType::Integer);
        assert_eq!(desc.dirty_flag, DirtyFlag::Cursor);
        assert!(desc.signal);
    }

    #[test]
    fn test_property_registry_children() {
        let mut registry = PropertyRegistry::new();

        registry.add(PropertyDescriptor::new(
            "cursor.xcursor_size",
            PropertyType::Integer,
            DirtyFlag::Cursor,
            dummy_getter,
            dummy_setter,
        ));
        registry.add(PropertyDescriptor::new(
            "cursor.xcursor_theme",
            PropertyType::String,
            DirtyFlag::Cursor,
            dummy_getter,
            dummy_setter,
        ));
        registry.add(PropertyDescriptor::new(
            "layout.gaps",
            PropertyType::Number,
            DirtyFlag::Layout,
            dummy_getter,
            dummy_setter,
        ));

        let cursor_children: Vec<_> = registry.child_keys("cursor");
        assert_eq!(cursor_children, vec!["xcursor_size", "xcursor_theme"]);

        let layout_children: Vec<_> = registry.child_keys("layout");
        assert_eq!(layout_children, vec!["gaps"]);
    }

    #[test]
    fn test_property_registry_is_nested() {
        let mut registry = PropertyRegistry::new();

        registry.add(PropertyDescriptor::nested("cursor", DirtyFlag::Cursor));
        registry.add(PropertyDescriptor::new(
            "cursor.xcursor_size",
            PropertyType::Integer,
            DirtyFlag::Cursor,
            dummy_getter,
            dummy_setter,
        ));

        assert!(registry.is_nested("cursor"));
        assert!(!registry.is_nested("cursor.xcursor_size"));
    }

    #[test]
    fn test_infer_dirty_flag() {
        assert_eq!(infer_dirty_flag("cursor.xcursor_size"), DirtyFlag::Cursor);
        assert_eq!(infer_dirty_flag("layout.gaps"), DirtyFlag::Layout);
        assert_eq!(
            infer_dirty_flag("animations.window_open"),
            DirtyFlag::Animations
        );
        assert_eq!(infer_dirty_flag("input.keyboard.xkb"), DirtyFlag::Input);
        assert_eq!(infer_dirty_flag("unknown.property"), DirtyFlag::Misc);
    }

    #[test]
    fn test_property_type_names() {
        assert_eq!(PropertyType::Bool.type_name(), "boolean");
        assert_eq!(PropertyType::Integer.type_name(), "integer");
        assert_eq!(PropertyType::Number.type_name(), "number");
        assert_eq!(PropertyType::String.type_name(), "string");
        assert_eq!(
            PropertyType::Enum {
                name: "CenterFocusedColumn",
                variants: &[]
            }
            .type_name(),
            "CenterFocusedColumn"
        );
        assert_eq!(
            PropertyType::Array(Box::new(PropertyType::Integer)).type_name(),
            "array"
        );
        assert_eq!(PropertyType::Nested.type_name(), "table");
    }

    #[test]
    fn test_no_signal() {
        let desc = PropertyDescriptor::new(
            "test.path",
            PropertyType::Bool,
            DirtyFlag::Misc,
            dummy_getter,
            dummy_setter,
        )
        .no_signal();

        assert!(!desc.signal);
    }

    #[test]
    #[should_panic(expected = "duplicate property path")]
    fn test_duplicate_path_panics() {
        let mut registry = PropertyRegistry::new();

        registry.add(PropertyDescriptor::new(
            "test.path",
            PropertyType::Bool,
            DirtyFlag::Misc,
            dummy_getter,
            dummy_setter,
        ));

        // This should panic
        registry.add(PropertyDescriptor::new(
            "test.path",
            PropertyType::Integer,
            DirtyFlag::Misc,
            dummy_getter,
            dummy_setter,
        ));
    }

    #[test]
    fn test_validate_array_elements() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set(1, 10i64).unwrap();
        table.set(2, 20i64).unwrap();
        table.set(3, 30i64).unwrap();

        let result: Vec<i64> = validate_array_elements(&lua, &table, |_, v| {
            v.as_integer()
                .ok_or_else(|| LuaError::external("expected integer"))
        })
        .unwrap();

        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn test_validate_array_elements_error() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set(1, 10i64).unwrap();
        table.set(2, "not an integer").unwrap();

        let result: LuaResult<Vec<i64>> = validate_array_elements(&lua, &table, |_, v| {
            v.as_integer()
                .ok_or_else(|| LuaError::external("expected integer"))
        });

        let err = result.unwrap_err();
        assert!(err.to_string().contains("index 2"));
    }

    #[test]
    fn test_init_from_config_registers_properties() {
        use niri_config::Config;
        use niri_lua_traits::ConfigProperties;

        let metadata = Config::property_metadata();

        assert!(!metadata.is_empty(), "Config should have properties");

        let expected_prefixes = [
            "config.cursor",
            "config.layout",
            "config.input",
            "config.animations",
        ];
        for prefix in expected_prefixes {
            let has_prefix = metadata.iter().any(|m| m.path.starts_with(prefix));
            assert!(
                has_prefix,
                "Should have properties starting with {}",
                prefix
            );
        }
    }
}
