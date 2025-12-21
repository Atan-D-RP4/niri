//! Common test utilities for niri-lua integration tests.

use std::sync::Arc;

use niri_lua::LuaRuntime;

/// Create a LuaRuntime with component registered (required for REPL and tests).
pub fn create_runtime() -> LuaRuntime {
    let mut runtime = LuaRuntime::new().expect("Failed to create Lua runtime");
    // Register component to get __niri_format_value (required for execute_string)
    runtime
        .register_component(|_, _| Ok(()))
        .expect("Failed to register component");
    // Initialize process manager (required for spawn tests)
    runtime.init_process_manager();
    // Register action proxy with a no-op callback (required for spawn tests)
    runtime
        .register_action_proxy(Arc::new(|_action| Ok(())))
        .expect("Failed to register action proxy");
    runtime
}
