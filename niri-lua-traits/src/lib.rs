//! Shared traits and helpers for Lua table extraction.
//!
//! This crate provides the `FromLuaTable` trait and helper functions used by
//! both `niri-lua` extractors and `niri-config` derive macros.

use std::str::FromStr;

use mlua::prelude::*;

/// Types that can be constructed from a Lua table.
///
/// This trait is the foundation for extracting Rust configuration types from
/// Lua tables. Implementations should return `Ok(Some(Self))` if any relevant
/// fields were present, `Ok(None)` if the table had no relevant fields (all
/// defaults), or `Err` if extraction failed.
pub trait FromLuaTable: Sized {
    /// Extract this type from a Lua table.
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>>;

    /// Extract this type, returning default if no fields present.
    fn from_lua_table_or_default(table: &LuaTable) -> LuaResult<Self>
    where
        Self: Default,
    {
        match Self::from_lua_table(table)? {
            Some(value) => Ok(value),
            None => Ok(Self::default()),
        }
    }

    /// Extract a required instance (error if no fields present).
    fn from_lua_table_required(table: &LuaTable) -> LuaResult<Self> {
        match Self::from_lua_table(table)? {
            Some(value) => Ok(value),
            None => Err(LuaError::external("missing required fields")),
        }
    }
}

/// Trait for extracting a nested table field as a type.
pub trait ExtractField<T> {
    fn extract_field(&self, field: &str) -> LuaResult<Option<T>>;
}

impl<T> ExtractField<T> for LuaTable
where
    T: FromLuaTable,
{
    fn extract_field(&self, field: &str) -> LuaResult<Option<T>> {
        match self.get::<LuaValue>(field)? {
            LuaValue::Nil => Ok(None),
            LuaValue::Boolean(false) => Ok(None),
            LuaValue::Table(table) => T::from_lua_table(&table),
            other => Err(LuaError::external(format!(
                "expected table for field '{field}', found {other:?}"
            ))),
        }
    }
}

/// Helper to extract an optional string field from a Lua table.
pub fn extract_string_opt(table: &LuaTable, field: &str) -> LuaResult<Option<String>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::String(s)) => Ok(Some(s.to_string_lossy().to_string())),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional boolean field from a Lua table.
pub fn extract_bool_opt(table: &LuaTable, field: &str) -> LuaResult<Option<bool>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Boolean(b)) => Ok(Some(b)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional integer field from a Lua table.
pub fn extract_int_opt(table: &LuaTable, field: &str) -> LuaResult<Option<i64>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Integer(i)) => Ok(Some(i)),
        Ok(LuaValue::Number(n)) => Ok(Some(n as i64)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional float field from a Lua table.
pub fn extract_float_opt(table: &LuaTable, field: &str) -> LuaResult<Option<f64>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Number(n)) => Ok(Some(n)),
        Ok(LuaValue::Integer(i)) => Ok(Some(i as f64)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional table field from a Lua table.
pub fn extract_table_opt(table: &LuaTable, field: &str) -> LuaResult<Option<LuaTable>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Table(t)) => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional vector of strings from a Lua table sequence.
pub fn extract_vec_opt(table: &LuaTable, field: &str) -> LuaResult<Option<Vec<String>>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Table(t)) => {
            let mut result = Vec::new();
            for (_, v) in t.pairs::<i64, String>().flatten() {
                result.push(v);
            }
            if result.is_empty() {
                Ok(None)
            } else {
                Ok(Some(result))
            }
        }
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Trait for types that can be parsed from a string (like Color).
///
/// This allows generic color extraction without depending on niri-config types.
pub trait FromLuaString: Sized {
    fn from_lua_string(s: &str) -> Option<Self>;
}

/// Helper to extract a type that implements FromLuaString from a string field.
pub fn extract_from_string_opt<T: FromLuaString>(
    table: &LuaTable,
    field: &str,
) -> LuaResult<Option<T>> {
    if let Some(s) = extract_string_opt(table, field)? {
        Ok(T::from_lua_string(&s))
    } else {
        Ok(None)
    }
}

/// Blanket implementation for any type that implements FromStr.
impl<T> FromLuaString for T
where
    T: FromStr,
{
    fn from_lua_string(s: &str) -> Option<Self> {
        T::from_str(s).ok()
    }
}
