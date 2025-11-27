//! IPC-based Lua REPL execution for Niri.
//!
//! This module provides an interface for executing Lua code via IPC requests,
//! enabling interactive scripting without modifying the main Niri codebase.
//! The REPL can be accessed via IPC by sending `Request::ExecuteLua` with Lua code,
//! and receiving a `Response::LuaResult` with the output.
//!
//! # Example
//!
//! ```text
//! // Send via IPC:
//! {"ExecuteLua": {"code": "print(niri.version_string())"}}
//!
//! // Receive:
//! {"LuaResult": {"output": "Niri 0.1.0 (abc1234)", "success": true}}
//! ```

use std::sync::{Arc, Mutex};

/// Handler for executing Lua code from IPC requests.
///
/// This struct wraps a reference to the Lua runtime and provides a method
/// to execute code strings in that runtime.
pub struct IpcLuaExecutor {
    runtime: Arc<Mutex<Option<crate::LuaRuntime>>>,
}

impl IpcLuaExecutor {
    /// Create a new IPC Lua executor.
    ///
    /// # Arguments
    ///
    /// * `runtime` - Arc<Mutex<Option<LuaRuntime>>> containing the Lua runtime
    pub fn new(runtime: Arc<Mutex<Option<crate::LuaRuntime>>>) -> Self {
        Self { runtime }
    }

    /// Execute Lua code and return the output.
    ///
    /// # Arguments
    ///
    /// * `code` - The Lua code to execute
    ///
    /// # Returns
    ///
    /// A tuple of (output_string, success_bool) where:
    /// - output_string contains any printed output or error messages
    /// - success_bool indicates whether the code executed without errors
    pub fn execute(&self, code: &str) -> (String, bool) {
        match self.runtime.lock() {
            Ok(guard) => match guard.as_ref() {
                Some(runtime) => runtime.execute_string(code),
                None => ("Lua runtime not initialized".to_string(), false),
            },
            Err(e) => (format!("Failed to acquire Lua runtime lock: {}", e), false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_executor_basic() {
        let lua_runtime = crate::LuaRuntime::new().unwrap();
        let executor = IpcLuaExecutor::new(Arc::new(Mutex::new(Some(lua_runtime))));

        let (output, success) = executor.execute("return 1 + 1");
        assert!(success, "Execution should succeed");
        assert!(output.contains("2"), "Output should contain result");
    }

    #[test]
    fn test_lua_executor_print() {
        let lua_runtime = crate::LuaRuntime::new().unwrap();
        let executor = IpcLuaExecutor::new(Arc::new(Mutex::new(Some(lua_runtime))));

        let (output, success) = executor.execute("print('Hello'); print('World')");
        assert!(success, "Execution should succeed");
        assert!(
            output.contains("Hello"),
            "Output should contain first print"
        );
        assert!(
            output.contains("World"),
            "Output should contain second print"
        );
    }

    #[test]
    fn test_lua_executor_error() {
        let lua_runtime = crate::LuaRuntime::new().unwrap();
        let executor = IpcLuaExecutor::new(Arc::new(Mutex::new(Some(lua_runtime))));

        let (output, success) = executor.execute("error('test error')");
        assert!(!success, "Execution should fail");
        assert!(
            output.contains("Error"),
            "Output should contain error message"
        );
    }

    #[test]
    fn test_lua_executor_not_initialized() {
        let executor = IpcLuaExecutor::new(Arc::new(Mutex::new(None)));
        let (output, success) = executor.execute("print('test')");
        assert!(!success, "Execution should fail without runtime");
        assert!(
            output.contains("not initialized"),
            "Output should indicate runtime not initialized"
        );
    }
}
