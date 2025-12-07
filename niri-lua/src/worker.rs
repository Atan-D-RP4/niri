//! Worker threads for offloading heavy Lua computation.
//!
//! This module provides the `niri.worker` API that allows Lua scripts to run
//! CPU-intensive computation in background threads without blocking the compositor.
//!
//! # Architecture
//!
//! Workers use **isolated Lua states** per thread to avoid the overhead of mlua's `send`
//! feature (which adds ~10-20% overhead to ALL Lua operations). Data is transferred
//! between the main thread and workers using JSON serialization.
//!
//! # Limitations
//!
//! Workers have access to a minimal Lua environment:
//! - Standard library (math, string, table, etc.)
//! - No `niri` API (no events, actions, config, state)
//! - Only pure computation
//!
//! # Example
//!
//! ```lua
//! local worker = niri.worker.new([[
//!     local result = 0
//!     for i = 1, 1000000 do
//!         result = result + i
//!     end
//!     return result
//! ]])
//!
//! worker:run(function(result, err)
//!     if err then
//!         print("Worker error:", err)
//!     else
//!         print("Worker result:", result)
//!     end
//! end)
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use async_channel::Sender;
use mlua::prelude::*;
use mlua::Compiler;

/// Default timeout for worker execution (30 seconds).
const WORKER_TIMEOUT: Duration = Duration::from_secs(30);

/// Counter for generating unique worker IDs.
static NEXT_WORKER_ID: AtomicU64 = AtomicU64::new(1);

/// Result from a worker thread execution.
#[derive(Debug)]
pub struct WorkerResult {
    /// The worker ID this result belongs to.
    pub worker_id: u64,
    /// The JSON-serialized result, or an error message.
    pub result: Result<serde_json::Value, String>,
}

/// Callback storage for pending worker results.
///
/// This is stored in the main Lua runtime and maps worker IDs to their
/// completion callbacks.
pub type WorkerCallbacks = Rc<RefCell<HashMap<u64, LuaRegistryKey>>>;

/// Create a new worker callbacks storage.
pub fn create_worker_callbacks() -> WorkerCallbacks {
    Rc::new(RefCell::new(HashMap::new()))
}

/// Register the worker API in a Lua context.
///
/// This creates the `niri.worker` table with:
/// - `new(script)` - Create a new worker with the given Lua script
///
/// The returned worker userdata has:
/// - `run([args], callback)` - Execute the worker with optional args and callback
/// - `cancel()` - Cancel a pending worker
///
/// # Arguments
///
/// * `lua` - The Lua context
/// * `result_tx` - Channel sender for delivering worker results to the main thread
/// * `callbacks` - Shared callback storage
///
/// # Example
///
/// ```lua
/// local worker = niri.worker.new([[
///     local args = ...
///     return args.x + args.y
/// ]])
///
/// worker:run({ x = 10, y = 20 }, function(result, err)
///     print("Sum:", result)  -- 30
/// end)
/// ```
pub fn register_worker_api(
    lua: &Lua,
    result_tx: Sender<WorkerResult>,
    callbacks: WorkerCallbacks,
) -> LuaResult<()> {
    // Get or create the niri table
    let niri: LuaTable = match lua.globals().get("niri")? {
        LuaValue::Table(t) => t,
        _ => {
            let t = lua.create_table()?;
            lua.globals().set("niri", t.clone())?;
            t
        }
    };

    // Create the worker table
    let worker_table = lua.create_table()?;

    // niri.worker.new(script) -> Worker
    let tx_clone = result_tx.clone();
    let callbacks_clone = callbacks.clone();
    let new_fn = lua.create_function(move |lua, script: String| {
        let worker_id = NEXT_WORKER_ID.fetch_add(1, Ordering::SeqCst);

        // Create Worker userdata
        let worker = Worker {
            id: worker_id,
            script,
            result_tx: tx_clone.clone(),
            callbacks: callbacks_clone.clone(),
            cancelled: Rc::new(RefCell::new(false)),
        };

        lua.create_userdata(worker)
    })?;
    worker_table.set("new", new_fn)?;

    niri.set("worker", worker_table)?;

    Ok(())
}

/// Deliver a worker result to the Lua callback.
///
/// This should be called from the main event loop when a worker result
/// is received from the result channel.
///
/// # Arguments
///
/// * `lua` - The Lua context
/// * `callbacks` - The callback storage
/// * `result` - The worker result to deliver
///
/// # Returns
///
/// Returns Ok if the callback was found and executed, or an error if
/// the callback was not found or execution failed.
pub fn deliver_worker_result(
    lua: &Lua,
    callbacks: &WorkerCallbacks,
    result: WorkerResult,
) -> LuaResult<()> {
    // Get and remove the callback for this worker
    let callback_key = callbacks.borrow_mut().remove(&result.worker_id);

    let Some(key) = callback_key else {
        // Worker was cancelled or callback already consumed
        return Ok(());
    };

    // Retrieve the callback function
    let callback: LuaFunction = lua.registry_value(&key)?;

    // Clean up registry
    lua.remove_registry_value(key)?;

    // Convert result to Lua values
    match result.result {
        Ok(json_value) => {
            // Convert JSON to Lua value
            let lua_value = json_to_lua(lua, &json_value)?;
            callback.call::<()>((lua_value, LuaValue::Nil))?;
        }
        Err(err_msg) => {
            callback.call::<()>((LuaValue::Nil, err_msg))?;
        }
    }

    Ok(())
}

/// Worker userdata for Lua.
struct Worker {
    id: u64,
    script: String,
    result_tx: Sender<WorkerResult>,
    callbacks: WorkerCallbacks,
    cancelled: Rc<RefCell<bool>>,
}

impl LuaUserData for Worker {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // worker:run([args], callback)
        methods.add_method("run", |lua, this, args: LuaMultiValue| {
            // Check if cancelled
            if *this.cancelled.borrow() {
                return Err(LuaError::external("Worker has been cancelled"));
            }

            // Parse arguments: either (callback) or (args, callback)
            let (json_args, callback) = parse_run_args(lua, args)?;

            // Store callback in registry
            let callback_key = lua.create_registry_value(callback)?;
            this.callbacks.borrow_mut().insert(this.id, callback_key);

            // Spawn worker thread
            let worker_id = this.id;
            let script = this.script.clone();
            let result_tx = this.result_tx.clone();

            thread::spawn(move || {
                let result = execute_worker_script(&script, json_args);
                let worker_result = WorkerResult { worker_id, result };

                // Send result back to main thread
                if let Err(e) = result_tx.send_blocking(worker_result) {
                    log::error!("Failed to send worker result: {}", e);
                }
            });

            Ok(())
        });

        // worker:cancel()
        methods.add_method("cancel", |_lua, this, ()| {
            *this.cancelled.borrow_mut() = true;

            // Remove callback if present (it won't be called)
            this.callbacks.borrow_mut().remove(&this.id);

            Ok(())
        });

        // worker.id (read-only)
        methods.add_meta_method(LuaMetaMethod::Index, |_lua, this, key: String| {
            match key.as_str() {
                "id" => Ok(LuaValue::Integer(this.id as i64)),
                _ => Ok(LuaValue::Nil),
            }
        });
    }
}

/// Parse the arguments to worker:run().
///
/// Accepts either:
/// - `(callback)` - no args, just a callback
/// - `(args, callback)` - args table and callback
fn parse_run_args(lua: &Lua, args: LuaMultiValue) -> LuaResult<(serde_json::Value, LuaFunction)> {
    let args_vec: Vec<LuaValue> = args.into_iter().collect();

    match args_vec.len() {
        1 => {
            // Just callback, no args
            let callback = match args_vec.into_iter().next().unwrap() {
                LuaValue::Function(f) => f,
                _ => return Err(LuaError::external("Expected callback function")),
            };
            Ok((serde_json::Value::Null, callback))
        }
        2 => {
            // Args and callback
            let mut iter = args_vec.into_iter();
            let args_value = iter.next().unwrap();
            let callback_value = iter.next().unwrap();

            let callback = match callback_value {
                LuaValue::Function(f) => f,
                _ => {
                    return Err(LuaError::external(
                        "Expected callback function as second argument",
                    ))
                }
            };

            let json_args = lua_to_json(lua, &args_value)?;
            Ok((json_args, callback))
        }
        n => Err(LuaError::external(format!(
            "Expected 1-2 arguments, got {}",
            n
        ))),
    }
}

/// Execute a worker script in an isolated Lua state.
///
/// This runs in a worker thread with its own Lua instance.
fn execute_worker_script(
    script: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Create isolated Lua state
    let lua = Lua::new();

    // Load safe standard libraries only
    lua.load_std_libs(LuaStdLib::ALL_SAFE)
        .map_err(|e| format!("Failed to load stdlib: {}", e))?;

    // Set up timeout interrupt (same as main runtime)
    let deadline = std::time::Instant::now() + WORKER_TIMEOUT;
    lua.set_interrupt(move |_lua| {
        if std::time::Instant::now() > deadline {
            return Err(LuaError::external("Worker execution timeout"));
        }
        Ok(LuaVmState::Continue)
    });

    // Compile script with optimization
    let compiler = Compiler::new().set_optimization_level(2).set_debug_level(1);

    let bytecode = compiler
        .compile(script)
        .map_err(|e| format!("Compilation error: {}", e))?;

    // Load the chunk
    let chunk = lua
        .load(bytecode)
        .into_function()
        .map_err(|e| format!("Load error: {}", e))?;

    // Convert JSON args to Lua value
    let lua_args = json_to_lua(&lua, &args).map_err(|e| format!("Args conversion error: {}", e))?;

    // Execute with args as varargs
    let result: LuaValue = chunk
        .call(lua_args)
        .map_err(|e| format!("Execution error: {}", e))?;

    // Convert result back to JSON
    lua_to_json(&lua, &result).map_err(|e| format!("Result conversion error: {}", e))
}

/// Convert a Lua value to JSON.
///
/// Supported types: nil, boolean, number, string, table (array or object)
/// Unsupported: functions, userdata, threads, metatables
#[allow(clippy::only_used_in_recursion)]
fn lua_to_json(lua: &Lua, value: &LuaValue) -> LuaResult<serde_json::Value> {
    match value {
        LuaValue::Nil => Ok(serde_json::Value::Null),
        LuaValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        LuaValue::Integer(i) => Ok(serde_json::json!(*i)),
        LuaValue::Number(n) => {
            // Handle NaN and infinity
            if n.is_nan() || n.is_infinite() {
                Ok(serde_json::Value::Null)
            } else {
                Ok(serde_json::json!(*n))
            }
        }
        LuaValue::String(s) => {
            let s = s
                .to_str()
                .map_err(|e| LuaError::external(format!("Invalid UTF-8: {}", e)))?;
            Ok(serde_json::Value::String(s.to_string()))
        }
        LuaValue::Table(t) => {
            // Check if it's an array (sequential integer keys starting at 1)
            let len = t.raw_len();
            if len > 0 {
                // Try to treat as array
                let mut arr = Vec::with_capacity(len);
                for i in 1..=len {
                    let v: LuaValue = t.raw_get(i)?;
                    arr.push(lua_to_json(lua, &v)?);
                }
                // Verify no other keys (simple check: pairs count matches len)
                let pairs_count: usize = t.pairs::<LuaValue, LuaValue>().count();
                if pairs_count == len {
                    return Ok(serde_json::Value::Array(arr));
                }
            }

            // Treat as object
            let mut map = serde_json::Map::new();
            for pair in t.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair?;
                let key = match k {
                    LuaValue::String(s) => s
                        .to_str()
                        .map_err(|e| LuaError::external(format!("Invalid key UTF-8: {}", e)))?
                        .to_string(),
                    LuaValue::Integer(i) => i.to_string(),
                    _ => continue, // Skip non-string, non-integer keys
                };
                map.insert(key, lua_to_json(lua, &v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        LuaValue::Function(_) => Err(LuaError::external("Cannot serialize function to JSON")),
        LuaValue::Thread(_) => Err(LuaError::external("Cannot serialize thread to JSON")),
        LuaValue::UserData(_) => Err(LuaError::external("Cannot serialize userdata to JSON")),
        LuaValue::LightUserData(_) => {
            Err(LuaError::external("Cannot serialize lightuserdata to JSON"))
        }
        LuaValue::Error(e) => Err(LuaError::external(format!(
            "Cannot serialize error to JSON: {}",
            e
        ))),
        _ => Err(LuaError::external(
            "Cannot serialize unknown Lua type to JSON",
        )),
    }
}

/// Convert a JSON value to Lua.
fn json_to_lua(lua: &Lua, value: &serde_json::Value) -> LuaResult<LuaValue> {
    match value {
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
        serde_json::Value::String(s) => {
            let lua_str = lua.create_string(s)?;
            Ok(LuaValue::String(lua_str))
        }
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (k, v) in obj {
                table.set(k.as_str(), json_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // JSON Conversion Tests
    // ========================================================================

    #[test]
    fn lua_to_json_nil() {
        let lua = Lua::new();
        let result = lua_to_json(&lua, &LuaValue::Nil).unwrap();
        assert_eq!(result, serde_json::Value::Null);
    }

    #[test]
    fn lua_to_json_boolean() {
        let lua = Lua::new();
        let result = lua_to_json(&lua, &LuaValue::Boolean(true)).unwrap();
        assert_eq!(result, serde_json::Value::Bool(true));
    }

    #[test]
    fn lua_to_json_integer() {
        let lua = Lua::new();
        let result = lua_to_json(&lua, &LuaValue::Integer(42)).unwrap();
        assert_eq!(result, serde_json::json!(42));
    }

    #[test]
    fn lua_to_json_number() {
        let lua = Lua::new();
        let result = lua_to_json(&lua, &LuaValue::Number(2.5)).unwrap();
        assert_eq!(result, serde_json::json!(2.5));
    }

    #[test]
    fn lua_to_json_string() {
        let lua = Lua::new();
        let s = lua.create_string("hello").unwrap();
        let result = lua_to_json(&lua, &LuaValue::String(s)).unwrap();
        assert_eq!(result, serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn lua_to_json_array() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set(1, "a").unwrap();
        table.set(2, "b").unwrap();
        table.set(3, "c").unwrap();

        let result = lua_to_json(&lua, &LuaValue::Table(table)).unwrap();
        assert_eq!(result, serde_json::json!(["a", "b", "c"]));
    }

    #[test]
    fn lua_to_json_object() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("x", 10).unwrap();
        table.set("y", 20).unwrap();

        let result = lua_to_json(&lua, &LuaValue::Table(table)).unwrap();
        assert_eq!(result, serde_json::json!({"x": 10, "y": 20}));
    }

    #[test]
    fn lua_to_json_nested() {
        let lua = Lua::new();
        let inner = lua.create_table().unwrap();
        inner.set("a", 1).unwrap();

        let outer = lua.create_table().unwrap();
        outer.set("nested", inner).unwrap();

        let result = lua_to_json(&lua, &LuaValue::Table(outer)).unwrap();
        assert_eq!(result, serde_json::json!({"nested": {"a": 1}}));
    }

    #[test]
    fn lua_to_json_function_fails() {
        let lua = Lua::new();
        let func = lua.create_function(|_, ()| Ok(())).unwrap();
        let result = lua_to_json(&lua, &LuaValue::Function(func));
        assert!(result.is_err());
    }

    // ========================================================================
    // JSON to Lua Conversion Tests
    // ========================================================================

    #[test]
    fn json_to_lua_null() {
        let lua = Lua::new();
        let result = json_to_lua(&lua, &serde_json::Value::Null).unwrap();
        assert!(matches!(result, LuaValue::Nil));
    }

    #[test]
    fn json_to_lua_bool() {
        let lua = Lua::new();
        let result = json_to_lua(&lua, &serde_json::Value::Bool(true)).unwrap();
        assert!(matches!(result, LuaValue::Boolean(true)));
    }

    #[test]
    fn json_to_lua_number() {
        let lua = Lua::new();
        let result = json_to_lua(&lua, &serde_json::json!(42)).unwrap();
        assert!(matches!(result, LuaValue::Integer(42)));
    }

    #[test]
    fn json_to_lua_float() {
        let lua = Lua::new();
        let result = json_to_lua(&lua, &serde_json::json!(2.5)).unwrap();
        if let LuaValue::Number(n) = result {
            assert!((n - 2.5).abs() < 0.001);
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn json_to_lua_string() {
        let lua = Lua::new();
        let result = json_to_lua(&lua, &serde_json::Value::String("test".to_string())).unwrap();
        if let LuaValue::String(s) = result {
            assert_eq!(s.to_str().unwrap(), "test");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn json_to_lua_array() {
        let lua = Lua::new();
        let result = json_to_lua(&lua, &serde_json::json!([1, 2, 3])).unwrap();
        if let LuaValue::Table(t) = result {
            assert_eq!(t.raw_len(), 3);
            let v1: i64 = t.get(1).unwrap();
            assert_eq!(v1, 1);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn json_to_lua_object() {
        let lua = Lua::new();
        let result = json_to_lua(&lua, &serde_json::json!({"key": "value"})).unwrap();
        if let LuaValue::Table(t) = result {
            let v: String = t.get("key").unwrap();
            assert_eq!(v, "value");
        } else {
            panic!("Expected Table");
        }
    }

    // ========================================================================
    // Worker Script Execution Tests
    // ========================================================================

    #[test]
    fn execute_worker_simple_return() {
        let result = execute_worker_script("return 42", serde_json::Value::Null);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!(42));
    }

    #[test]
    fn execute_worker_computation() {
        let result = execute_worker_script(
            "local sum = 0; for i = 1, 10 do sum = sum + i end; return sum",
            serde_json::Value::Null,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!(55));
    }

    #[test]
    fn execute_worker_with_args() {
        let result = execute_worker_script(
            "local args = ...; return args.x + args.y",
            serde_json::json!({"x": 10, "y": 20}),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!(30));
    }

    #[test]
    fn execute_worker_returns_table() {
        let result = execute_worker_script("return { a = 1, b = 2 }", serde_json::Value::Null);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!({"a": 1, "b": 2}));
    }

    #[test]
    fn execute_worker_returns_array() {
        let result = execute_worker_script("return { 1, 2, 3 }", serde_json::Value::Null);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn execute_worker_syntax_error() {
        let result = execute_worker_script("return {", serde_json::Value::Null);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Compilation error") || err.contains("error"));
    }

    #[test]
    fn execute_worker_runtime_error() {
        let result = execute_worker_script("error('intentional')", serde_json::Value::Null);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("intentional"));
    }

    #[test]
    fn execute_worker_no_niri_api() {
        // Workers should not have access to niri API
        let result = execute_worker_script("return niri == nil", serde_json::Value::Null);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::Value::Bool(true));
    }

    // ========================================================================
    // Worker ID Tests
    // ========================================================================

    #[test]
    fn worker_ids_are_unique() {
        let id1 = NEXT_WORKER_ID.fetch_add(1, Ordering::SeqCst);
        let id2 = NEXT_WORKER_ID.fetch_add(1, Ordering::SeqCst);
        assert_ne!(id1, id2);
    }

    // ========================================================================
    // WorkerCallbacks Tests
    // ========================================================================

    #[test]
    fn worker_callbacks_insert_remove() {
        let lua = Lua::new();
        let callbacks = create_worker_callbacks();

        let func = lua.create_function(|_, ()| Ok(())).unwrap();
        let key = lua.create_registry_value(func).unwrap();

        callbacks.borrow_mut().insert(1, key);
        assert!(callbacks.borrow().contains_key(&1));

        let removed = callbacks.borrow_mut().remove(&1);
        assert!(removed.is_some());
        assert!(!callbacks.borrow().contains_key(&1));
    }
}
