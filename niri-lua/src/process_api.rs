//! Process API for Lua - provides niri.process namespace.
//!
//! This module provides async process spawning with callback support:
//! - `niri.process.spawn(cmd, opts?, on_exit?)` - spawn with array command
//! - `niri.process.spawn_sh(cmd, opts?, on_exit?)` - spawn with shell command
//!
//! Unlike `niri.action:spawn()`, these functions integrate with the event loop
//! and support async callbacks for process completion.

use mlua::prelude::*;

use crate::process::{
    next_tracking_id, spawn_command, spawn_command_async, spawn_shell_command,
    spawn_shell_command_async, ProcessCallbacks, SharedProcessManager, SpawnOpts,
};

/// Register the process API with the Lua context.
///
/// This creates the `niri.process` table with spawn functions.
///
/// # Arguments
///
/// * `lua` - The Lua context
/// * `manager` - Shared process manager for callback registration
pub fn register_process_api(lua: &Lua, manager: SharedProcessManager) -> LuaResult<()> {
    // Get or create the niri table
    let niri: LuaTable = match lua.globals().get("niri")? {
        LuaValue::Table(t) => t,
        _ => {
            let t = lua.create_table()?;
            lua.globals().set("niri", t.clone())?;
            t
        }
    };

    // Create the process table
    let process_table = lua.create_table()?;

    // niri.process.spawn(cmd, opts?, on_exit?) -> ProcessHandle | nil
    let manager_clone = manager.clone();
    let spawn_fn = lua.create_function(move |lua, args: LuaMultiValue| {
        let mut args_iter = args.into_iter();

        // First argument: command (array of strings)
        let cmd_value = args_iter
            .next()
            .ok_or_else(|| LuaError::external("spawn requires a command argument"))?;

        let command: Vec<String> = match cmd_value {
            LuaValue::Table(t) => {
                let mut cmd = Vec::new();
                for i in 1..=t.len()? {
                    let s: String = t.get(i)?;
                    cmd.push(s);
                }
                if cmd.is_empty() {
                    return Err(LuaError::external("Command array cannot be empty"));
                }
                cmd
            }
            _ => return Err(LuaError::external("spawn requires a command array")),
        };

        // Second argument: opts (table, optional)
        let opts_value = args_iter.next();

        // We need to handle stdout/stderr which can be boolean or function.
        // To avoid creating registry keys unnecessarily (and leaking), we
        // first parse functions as LuaFunction and only create registry keys
        // when we decide to spawn asynchronously (i.e., when on_exit or
        // streaming callbacks are present).
        let (spawn_opts, stdout_fn, stderr_fn): (
            SpawnOpts,
            Option<LuaFunction>,
            Option<LuaFunction>,
        ) = match opts_value {
            Some(LuaValue::Table(t)) => {
                let mut sopts = SpawnOpts::from_lua_table(&t)?;

                // stdout: boolean | function | nil
                let stdout_val = t.get::<LuaValue>("stdout")?;
                let stdout_fn = match stdout_val {
                    LuaValue::Function(f) => {
                        // streaming implies we should capture for final result
                        sopts.capture_stdout = true;
                        Some(f)
                    }
                    LuaValue::Boolean(b) => {
                        sopts.capture_stdout = b;
                        None
                    }
                    LuaValue::Nil => None,
                    _ => return Err(LuaError::external("stdout must be boolean or function")),
                };

                // stderr: boolean | function | nil
                let stderr_val = t.get::<LuaValue>("stderr")?;
                let stderr_fn = match stderr_val {
                    LuaValue::Function(f) => {
                        sopts.capture_stderr = true;
                        Some(f)
                    }
                    LuaValue::Boolean(b) => {
                        sopts.capture_stderr = b;
                        None
                    }
                    LuaValue::Nil => None,
                    _ => return Err(LuaError::external("stderr must be boolean or function")),
                };

                (sopts, stdout_fn, stderr_fn)
            }
            Some(LuaValue::Nil) | None => (SpawnOpts::default(), None, None),
            _ => return Err(LuaError::external("opts must be a table or nil")),
        };

        // Third argument: on_exit callback (function, optional)
        let on_exit_value = args_iter.next();
        let on_exit_fn: Option<LuaFunction> = match on_exit_value {
            Some(LuaValue::Function(f)) => Some(f),
            Some(LuaValue::Nil) | None => None,
            _ => return Err(LuaError::external("on_exit must be a function or nil")),
        };

        // If detach mode, use simple spawn (no tracking)
        if spawn_opts.detach {
            let _ = spawn_command(command, spawn_opts)
                .map_err(|e| LuaError::external(format!("Failed to spawn: {}", e)))?;
            return Ok(LuaValue::Nil);
        }

        // Decide whether we need async spawn (tracking) i.e. when on_exit or
        // when streaming callbacks are provided.
        let need_async = on_exit_fn.is_some() || stdout_fn.is_some() || stderr_fn.is_some();

        // If no async work is needed, do a simple sync spawn and return handle
        if !need_async {
            let handle = spawn_command(command, spawn_opts)
                .map_err(|e| LuaError::external(format!("Failed to spawn: {}", e)))?;
            let ud = lua.create_userdata(handle)?;
            return Ok(LuaValue::UserData(ud));
        }

        // Async spawn path - create registry keys for any callbacks and register
        let tracking_id = next_tracking_id();

        // Store boolean flags before moving the functions
        let stream_stdout = stdout_fn.is_some();
        let stream_stderr = stderr_fn.is_some();

        // Convert optional LuaFunction callbacks into registry keys
        let on_exit_key: Option<LuaRegistryKey> = match on_exit_fn {
            Some(f) => Some(lua.create_registry_value(f)?),
            None => None,
        };

        let stdout_cb_key: Option<LuaRegistryKey> = match stdout_fn {
            Some(f) => Some(lua.create_registry_value(f)?),
            None => None,
        };

        let stderr_cb_key: Option<LuaRegistryKey> = match stderr_fn {
            Some(f) => Some(lua.create_registry_value(f)?),
            None => None,
        };

        // Get event sender from manager
        let event_tx = manager_clone.borrow().event_sender();

        // Register the process with callbacks
        let pid_placeholder = 0u32; // Will be updated after spawn
        manager_clone.borrow_mut().register(
            tracking_id,
            pid_placeholder,
            on_exit_key,
            stdout_cb_key,
            stderr_cb_key,
        );

        // Spawn with callbacks
        let callbacks = ProcessCallbacks {
            tracking_id,
            event_tx,
            stream_stdout,
            stream_stderr,
            ping: manager_clone.borrow().ping(),
        };

        match spawn_command_async(command, spawn_opts, callbacks) {
            Ok(handle) => {
                let ud = lua.create_userdata(handle)?;
                Ok(LuaValue::UserData(ud))
            }
            Err(e) => {
                // Clean up registry keys on spawn failure
                if let Some(keys) = manager_clone.borrow_mut().unregister(tracking_id) {
                    for key in keys {
                        let _ = lua.remove_registry_value(key);
                    }
                }
                Err(LuaError::external(format!("Failed to spawn: {}", e)))
            }
        }
    })?;
    process_table.set("spawn", spawn_fn)?;

    // niri.process.spawn_sh(cmd, opts?, on_exit?) -> ProcessHandle | nil
    let manager_clone2 = manager.clone();
    let spawn_sh_fn = lua.create_function(move |lua, args: LuaMultiValue| {
        let mut args_iter = args.into_iter();

        // First argument: command (string)
        let cmd_value = args_iter
            .next()
            .ok_or_else(|| LuaError::external("spawn_sh requires a command argument"))?;

        let command: String = match cmd_value {
            LuaValue::String(s) => s.to_str()?.to_string(),
            _ => return Err(LuaError::external("spawn_sh requires a command string")),
        };

        // Second argument: opts (table, optional)
        let opts_value = args_iter.next();

        // Parse options and possible streaming callbacks (stdout/stderr) as
        // either boolean or function. We defer creating registry keys until we
        // know we need async registration.
        let (spawn_opts, stdout_fn, stderr_fn): (
            SpawnOpts,
            Option<LuaFunction>,
            Option<LuaFunction>,
        ) = match opts_value {
            Some(LuaValue::Table(t)) => {
                let mut sopts = SpawnOpts::from_lua_table(&t)?;

                // stdout: boolean | function | nil
                let stdout_val = t.get::<LuaValue>("stdout")?;
                let stdout_fn = match stdout_val {
                    LuaValue::Function(f) => {
                        sopts.capture_stdout = true;
                        Some(f)
                    }
                    LuaValue::Boolean(b) => {
                        sopts.capture_stdout = b;
                        None
                    }
                    LuaValue::Nil => None,
                    _ => return Err(LuaError::external("stdout must be boolean or function")),
                };

                // stderr: boolean | function | nil
                let stderr_val = t.get::<LuaValue>("stderr")?;
                let stderr_fn = match stderr_val {
                    LuaValue::Function(f) => {
                        sopts.capture_stderr = true;
                        Some(f)
                    }
                    LuaValue::Boolean(b) => {
                        sopts.capture_stderr = b;
                        None
                    }
                    LuaValue::Nil => None,
                    _ => return Err(LuaError::external("stderr must be boolean or function")),
                };

                (sopts, stdout_fn, stderr_fn)
            }
            Some(LuaValue::Nil) | None => (SpawnOpts::default(), None, None),
            _ => return Err(LuaError::external("opts must be a table or nil")),
        };

        // Third argument: on_exit callback (function, optional)
        let on_exit_value = args_iter.next();
        let on_exit_fn: Option<LuaFunction> = match on_exit_value {
            Some(LuaValue::Function(f)) => Some(f),
            Some(LuaValue::Nil) | None => None,
            _ => return Err(LuaError::external("on_exit must be a function or nil")),
        };

        // If detach mode, use simple spawn (no tracking)
        if spawn_opts.detach {
            let _ = spawn_shell_command(command, spawn_opts)
                .map_err(|e| LuaError::external(format!("Failed to spawn: {}", e)))?;
            return Ok(LuaValue::Nil);
        }

        // Decide if we need async spawn (on_exit or streaming callbacks present)
        let need_async = on_exit_fn.is_some() || stdout_fn.is_some() || stderr_fn.is_some();

        if !need_async {
            let handle = spawn_shell_command(command, spawn_opts)
                .map_err(|e| LuaError::external(format!("Failed to spawn: {}", e)))?;
            let ud = lua.create_userdata(handle)?;
            return Ok(LuaValue::UserData(ud));
        }

        // Async spawn path
        let tracking_id = next_tracking_id();

        // Store boolean flags before moving the functions
        let stream_stdout = stdout_fn.is_some();
        let stream_stderr = stderr_fn.is_some();

        let on_exit_key: Option<LuaRegistryKey> = match on_exit_fn {
            Some(f) => Some(lua.create_registry_value(f)?),
            None => None,
        };

        let stdout_cb_key: Option<LuaRegistryKey> = match stdout_fn {
            Some(f) => Some(lua.create_registry_value(f)?),
            None => None,
        };

        let stderr_cb_key: Option<LuaRegistryKey> = match stderr_fn {
            Some(f) => Some(lua.create_registry_value(f)?),
            None => None,
        };

        // Get event sender from manager
        let event_tx = manager_clone2.borrow().event_sender();

        // Register the process with callbacks
        manager_clone2.borrow_mut().register(
            tracking_id,
            0, // PID placeholder
            on_exit_key,
            stdout_cb_key,
            stderr_cb_key,
        );

        // Spawn with callbacks
        let _callbacks = ProcessCallbacks {
            tracking_id,
            event_tx,
            stream_stdout,
            stream_stderr,
            ping: manager_clone2.borrow().ping(),
        };

        match spawn_shell_command_async(command, spawn_opts, _callbacks) {
            Ok(handle) => {
                let ud = lua.create_userdata(handle)?;
                Ok(LuaValue::UserData(ud))
            }
            Err(e) => {
                if let Some(keys) = manager_clone2.borrow_mut().unregister(tracking_id) {
                    for key in keys {
                        let _ = lua.remove_registry_value(key);
                    }
                }
                Err(LuaError::external(format!("Failed to spawn: {}", e)))
            }
        }
    })?;
    process_table.set("spawn_sh", spawn_sh_fn)?;

    niri.set("process", process_table)?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use mlua::prelude::*;

    use super::register_process_api;
    use crate::process::create_process_manager;

    #[test]
    fn test_spawn_basic() {
        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager).unwrap();

        // Spawn echo command
        let result: LuaValue = lua
            .load(r#"return niri.process.spawn({"echo", "hello"})"#)
            .eval()
            .unwrap();

        // Should return a userdata (ProcessHandle)
        assert!(matches!(result, LuaValue::UserData(_)));
    }

    #[test]
    fn test_spawn_with_wait() {
        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager).unwrap();

        // Spawn and wait
        let result: LuaTable = lua
            .load(
                r#"
                local proc = niri.process.spawn({"echo", "hello"}, {})
                return proc:wait()
            "#,
            )
            .eval()
            .unwrap();

        let code: i32 = result.get("code").unwrap();
        let stdout: String = result.get("stdout").unwrap();

        assert_eq!(code, 0);
        assert_eq!(stdout, "hello");
    }

    #[test]
    fn test_spawn_sh_basic() {
        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager).unwrap();

        // Spawn shell command
        let result: LuaValue = lua
            .load(r#"return niri.process.spawn_sh("echo hello")"#)
            .eval()
            .unwrap();

        assert!(matches!(result, LuaValue::UserData(_)));
    }

    #[test]
    fn test_spawn_detach() {
        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager).unwrap();

        // Spawn with detach - should return nil
        let result: LuaValue = lua
            .load(r#"return niri.process.spawn({"true"}, {detach = true})"#)
            .eval()
            .unwrap();

        assert!(matches!(result, LuaValue::Nil));
    }

    #[test]
    fn test_spawn_with_on_exit_registers() {
        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        // Spawn with on_exit callback
        lua.load(
            r#"
            niri.process.spawn({"echo", "hello"}, {}, function(result)
                -- callback will be called when process exits
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Should have registered a process
        assert!(!manager.borrow().is_empty());
    }

    #[test]
    fn test_spawn_sh_with_on_exit_registers() {
        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        // Spawn shell command with on_exit callback
        lua.load(
            r#"
            niri.process.spawn_sh("echo hello", {}, function(result)
                -- callback
            end)
        "#,
        )
        .exec()
        .unwrap();

        assert!(!manager.borrow().is_empty());
    }

    #[test]
    fn test_on_exit_callback_fires() {
        use std::time::Duration;

        use crate::process::fire_due_process_events;

        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        // Set up a global variable to track callback invocation
        lua.load("_G.callback_result = nil").exec().unwrap();

        // Spawn a quick command with on_exit callback
        lua.load(
            r#"
            niri.process.spawn({"echo", "test_output"}, {}, function(result)
                _G.callback_result = result
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Wait for the process to complete (echo is fast)
        std::thread::sleep(Duration::from_millis(100));

        // Fire pending events - this should invoke the callback
        let (processed, errors) = fire_due_process_events(&lua, &manager);

        // Should have processed the exit event
        assert!(processed > 0, "Expected at least one event to be processed");
        assert!(errors.is_empty(), "Expected no errors: {:?}", errors);

        // Verify the callback was invoked with the result
        let result: LuaTable = lua.load("return _G.callback_result").eval().unwrap();
        let code: i32 = result.get("code").unwrap();
        let stdout: String = result.get("stdout").unwrap();

        assert_eq!(code, 0);
        assert_eq!(stdout, "test_output");

        // Manager should have cleaned up the process
        assert!(manager.borrow().is_empty());
    }

    #[test]
    fn test_on_exit_callback_with_exit_code() {
        use std::time::Duration;

        use crate::process::fire_due_process_events;

        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        lua.load("_G.exit_code = nil").exec().unwrap();

        // Spawn a command that exits with code 42
        lua.load(
            r#"
            niri.process.spawn_sh("exit 42", {}, function(result)
                _G.exit_code = result.code
            end)
        "#,
        )
        .exec()
        .unwrap();

        std::thread::sleep(Duration::from_millis(100));
        fire_due_process_events(&lua, &manager);

        let exit_code: i32 = lua.load("return _G.exit_code").eval().unwrap();
        assert_eq!(exit_code, 42);
    }

    #[test]
    fn test_on_exit_callback_with_stderr() {
        use std::time::Duration;

        use crate::process::fire_due_process_events;

        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        lua.load("_G.stderr_output = nil").exec().unwrap();

        // Spawn a command that writes to stderr
        lua.load(
            r#"
            niri.process.spawn_sh("echo error_msg >&2", {}, function(result)
                _G.stderr_output = result.stderr
            end)
        "#,
        )
        .exec()
        .unwrap();

        std::thread::sleep(Duration::from_millis(100));
        fire_due_process_events(&lua, &manager);

        let stderr: String = lua.load("return _G.stderr_output").eval().unwrap();
        assert_eq!(stderr, "error_msg");
    }

    #[test]
    fn test_stdout_streaming_callback() {
        use std::time::Duration;

        use crate::process::fire_due_process_events;

        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        lua.load("_G.stream_lines = nil").exec().unwrap();

        // Spawn a command that prints two lines. Provide a stdout streaming
        // callback and an on_exit callback so the process is tracked.
        // Callback signature: function(err, data) as per spec
        lua.load(
            r#"
            niri.process.spawn_sh("printf 'one\ntwo\n'", {
                stdout = function(err, data)
                    _G.stream_lines = _G.stream_lines or {}
                    table.insert(_G.stream_lines, data)
                end
            }, function(result)
                _G.exit_result = result
            end)
        "#,
        )
        .exec()
        .unwrap();

        // Wait briefly for process to complete
        std::thread::sleep(Duration::from_millis(100));

        let (processed, errors) = fire_due_process_events(&lua, &manager);
        assert!(processed > 0);
        assert!(errors.is_empty());

        // Verify streaming callback was called per-line
        let lines_table: LuaTable = lua.load("return _G.stream_lines").eval().unwrap();
        assert_eq!(lines_table.len().unwrap(), 2);
        let l1: String = lines_table.get(1).unwrap();
        let l2: String = lines_table.get(2).unwrap();
        assert_eq!(l1, "one");
        assert_eq!(l2, "two");

        // Verify final captured stdout also contains both lines (text mode strips trailing newline)
        let result: LuaTable = lua.load("return _G.exit_result").eval().unwrap();
        let stdout: String = result.get("stdout").unwrap();
        assert_eq!(stdout, "one\ntwo");
    }

    #[test]
    fn test_stderr_streaming_callback() {
        use std::time::Duration;

        use crate::process::fire_due_process_events;

        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        lua.load("_G.stderr_line = nil").exec().unwrap();

        // Callback signature: function(err, data) as per spec
        lua.load(
            r#"
            niri.process.spawn_sh("echo error_msg >&2", {
                stderr = function(err, data)
                    _G.stderr_line = data
                end
            }, function(result)
                _G.exit_done = true
            end)
        "#,
        )
        .exec()
        .unwrap();

        std::thread::sleep(Duration::from_millis(100));

        let (_processed, errors) = fire_due_process_events(&lua, &manager);
        assert!(errors.is_empty());

        let stderr: String = lua.load("return _G.stderr_line").eval().unwrap();
        assert_eq!(stderr, "error_msg");
    }

    #[test]
    fn test_streaming_with_capture() {
        use std::time::Duration;

        use crate::process::fire_due_process_events;

        let lua = Lua::new();
        let manager = create_process_manager();
        register_process_api(&lua, manager.clone()).unwrap();

        lua.load("_G.lines = nil; _G.final_stdout = nil")
            .exec()
            .unwrap();

        // Callback signature: function(err, data) as per spec
        lua.load(
            r#"
            niri.process.spawn_sh("printf 'a\nb\n'", {
                stdout = function(err, data)
                    _G.lines = _G.lines or {}
                    table.insert(_G.lines, data)
                end
            }, function(result)
                _G.final_stdout = result.stdout
            end)
        "#,
        )
        .exec()
        .unwrap();

        std::thread::sleep(Duration::from_millis(100));

        let (_processed, errors) = fire_due_process_events(&lua, &manager);
        assert!(errors.is_empty());

        let lines_table: LuaTable = lua.load("return _G.lines").eval().unwrap();
        assert_eq!(lines_table.len().unwrap(), 2);
        let final_stdout: String = lua.load("return _G.final_stdout").eval().unwrap();
        assert_eq!(final_stdout, "a\nb");
    }
}
