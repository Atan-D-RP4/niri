//! Derive macros for generating Lua configuration bindings.
//!
//! This crate provides three derive macros:
//! - `LuaConfigProxy`: Generates proxy structs with UserData implementation for config structs
//! - `LuaEnum`: Generates string conversion implementations for enums
//! - `DirtyFlags`: Generates dirty flag tracking with enum and helper methods

use proc_macro::TokenStream;

mod attributes;
mod collection_proxy;
mod config_proxy;
mod dirty_flags;
mod from_lua_table;
mod lua_enum;

/// Derive macro for generating Lua proxy types from config structs.
///
/// # Example
/// ```ignore
/// #[derive(LuaConfigProxy)]
/// #[lua_proxy(parent_path = "layout", dirty = "layout")]
/// pub struct FocusRing {
///     #[lua_proxy(field)]
///     pub width: f64,
///     
///     #[lua_proxy(nested)]
///     pub gradient: Option<Gradient>,
///     
///     #[lua_proxy(skip)]
///     internal: SomeType,
/// }
/// ```
#[proc_macro_derive(LuaConfigProxy, attributes(lua_proxy))]
pub fn derive_lua_config_proxy(input: TokenStream) -> TokenStream {
    config_proxy::derive(input)
}

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
