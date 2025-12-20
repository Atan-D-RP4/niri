//! Common test utilities for niri-lua integration tests.

use niri_lua::{create_process_manager, register_process_api, LuaRuntime, SharedProcessManager};

/// Create a LuaRuntime with component registered (required for REPL and tests).
pub fn create_runtime() -> LuaRuntime {
    let runtime = LuaRuntime::new().expect("Failed to create Lua runtime");
    // Register component to get __niri_format_value (required for execute_string)
    runtime
        .register_component(|_, _| Ok(()))
        .expect("Failed to register component");
    runtime
}

/// Create a LuaRuntime with the process API registered and return the shared manager.
pub fn create_runtime_with_process_api() -> (LuaRuntime, SharedProcessManager) {
    let mut runtime = create_runtime();
    let manager = create_process_manager();
    register_process_api(runtime.inner(), manager.clone()).expect("Failed to register process API");
    // Set the process manager on the runtime so fire_process_events() works
    runtime.set_process_manager(manager.clone());
    (runtime, manager)
}
