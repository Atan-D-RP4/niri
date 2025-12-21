//! Common test utilities for niri-lua integration tests.

use niri_lua::LuaRuntime;

/// Create a LuaRuntime with component registered (required for REPL and tests).
pub fn create_runtime() -> LuaRuntime {
    let runtime = LuaRuntime::new().expect("Failed to create Lua runtime");
    // Register component to get __niri_format_value (required for execute_string)
    runtime
        .register_component(|_, _| Ok(()))
        .expect("Failed to register component");
    runtime
}
