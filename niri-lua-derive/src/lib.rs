//! Derive macros for generating Lua configuration bindings.
//!
//! This crate provides derive macros:
//! - `LuaEnum`: Generates string conversion implementations for enums
//! - `DirtyFlags`: Generates dirty flag tracking with enum and helper methods
//! - `ConfigProperties`: Generates property registration code for config structs
//! - `FromLuaTable`: Extracts Rust structs from Lua tables

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod attributes;
mod config_properties;
mod dirty_flags;
mod from_lua_table;
mod lua_enum;

/// Derive macro for generating string conversion for enums.
///
/// # Example
/// ```ignore
/// #[derive(LuaEnum)]
/// #[lua_enum(rename_all = "kebab-case")]
/// pub enum CenterFocusedColumn {
///     Never,
///     Always,
///     OnOverflow,
/// }
/// ```
#[proc_macro_derive(LuaEnum, attributes(lua_enum))]
pub fn derive_lua_enum(input: TokenStream) -> TokenStream {
    lua_enum::derive(input)
}

/// Derive macro for generating dirty flag tracking.
///
/// # Example
/// ```ignore
/// #[derive(DirtyFlags)]
/// pub struct ConfigDirtyFlags {
///     pub input: bool,
///     pub layout: bool,
///     pub animations: bool,
/// }
/// ```
///
/// This generates:
/// - A `ConfigDirtyFlag` enum with variants `Input`, `Layout`, `Animations`
/// - Methods: `mark()`, `is_dirty()`, `any()`, `clear()`, `clear_flag()`, `dirty_flags()`
/// - A `Default` implementation that initializes all flags to `false`
#[proc_macro_derive(DirtyFlags)]
pub fn derive_dirty_flags(input: TokenStream) -> TokenStream {
    dirty_flags::derive(input)
}

#[proc_macro_derive(ConfigProperties, attributes(config))]
pub fn derive_config_properties(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match config_properties::derive(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for extracting Rust structs from Lua tables.
///
/// # Example
/// ```ignore
/// #[derive(Default, FromLuaTable)]
/// pub struct FocusRing {
///     #[from_lua(float)]
///     pub width: f64,
///     
///     #[from_lua(nested)]
///     pub active_gradient: Option<Gradient>,
///     
///     #[from_lua(skip)]
///     internal: SomeType,
/// }
/// ```
///
/// Supported field attributes:
/// - `#[from_lua(string)]` - Extract as string
/// - `#[from_lua(bool)]` - Extract as boolean
/// - `#[from_lua(int)]` - Extract as integer
/// - `#[from_lua(float)]` - Extract as float
/// - `#[from_lua(color)]` - Extract as color
/// - `#[from_lua(nested)]` - Extract as nested FromLuaTable
/// - `#[from_lua(vec)]` - Extract as Vec of FromLuaTable items
/// - `#[from_lua(skip)]` - Skip this field
/// - `#[from_lua(rename = "lua-name")]` - Use custom Lua key name
#[proc_macro_derive(FromLuaTable, attributes(from_lua))]
pub fn derive_from_lua_table(input: TokenStream) -> TokenStream {
    from_lua_table::derive(input)
}
