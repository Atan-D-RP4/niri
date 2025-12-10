// Lua API Schema definitions for type generation.
//
// This module defines the schema types used to describe the niri Lua API.
// The schema is used by build.rs to generate EmmyLua type definitions.
//
// ## Usage
//
// Define the API structure in `api_registry.rs` using these types, then
// build.rs will serialize it to `types/api.lua` for LSP support.

/// Complete schema for the niri Lua API.
#[derive(Debug, Clone)]
pub struct LuaApiSchema {
    /// Top-level modules (niri, niri.action, niri.state, etc.)
    pub modules: &'static [ModuleSchema],
    /// UserData types (Timer, Window, Animation, etc.)
    pub types: &'static [TypeSchema],
    /// Type aliases for common patterns
    pub aliases: &'static [AliasSchema],
}

/// Schema for a Lua module/namespace.
#[derive(Debug, Clone)]
pub struct ModuleSchema {
    /// Module path (e.g., "niri", "niri.action", "niri.state")
    pub path: &'static str,
    /// Module description
    pub description: &'static str,
    /// Functions defined in this module
    pub functions: &'static [FunctionSchema],
    /// Sub-module fields (e.g., niri.action, niri.state)
    pub fields: &'static [FieldSchema],
}

/// Schema for a function or method.
#[derive(Debug, Clone)]
pub struct FunctionSchema {
    /// Function name
    pub name: &'static str,
    /// Function description
    pub description: &'static str,
    /// Whether this is a method (uses `:` syntax) or function (uses `.` syntax)
    pub is_method: bool,
    /// Function parameters
    pub params: &'static [ParamSchema],
    /// Return types
    pub returns: &'static [ReturnSchema],
}

/// Schema for a function parameter.
#[derive(Debug, Clone)]
pub struct ParamSchema {
    /// Parameter name
    pub name: &'static str,
    /// Parameter type (EmmyLua type syntax)
    pub ty: &'static str,
    /// Parameter description
    pub description: &'static str,
    /// Whether the parameter is optional
    pub optional: bool,
}

/// Schema for a function return value.
#[derive(Debug, Clone)]
pub struct ReturnSchema {
    /// Return type (EmmyLua type syntax)
    pub ty: &'static str,
    /// Return value description
    pub description: &'static str,
}

/// Schema for a field (module field or type field).
#[derive(Debug, Clone)]
pub struct FieldSchema {
    /// Field name
    pub name: &'static str,
    /// Field type (EmmyLua type syntax)
    pub ty: &'static str,
    /// Field description
    pub description: &'static str,
}

/// Schema for a UserData type (class).
#[derive(Debug, Clone)]
pub struct TypeSchema {
    /// Type name (e.g., "Timer", "Window", "Animation")
    pub name: &'static str,
    /// Type description
    pub description: &'static str,
    /// Type fields
    pub fields: &'static [FieldSchema],
    /// Type methods
    pub methods: &'static [FunctionSchema],
}

/// Schema for a type alias.
#[derive(Debug, Clone)]
pub struct AliasSchema {
    /// Alias name
    pub name: &'static str,
    /// Aliased type (EmmyLua type syntax)
    pub ty: &'static str,
    /// Alias description
    pub description: &'static str,
}
