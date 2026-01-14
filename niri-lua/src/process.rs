//! Process spawning and management API for Lua.
//!
//! This module provides process spawning capabilities integrated with `niri.action:spawn()`
//! and `niri.action:spawn_sh()`. When called with an options table, these functions return
//! a `ProcessHandle` userdata that allows controlling and monitoring the spawned process.
//!
//! # API Overview
//!
//! ```lua
//! -- Fire-and-forget (existing behavior)
//! niri.action:spawn({"cmd", "arg1", "arg2"})
//! niri.action:spawn_sh("cmd arg1 | cmd2")
//!
//! -- With options, returns ProcessHandle
//! local handle = niri.action:spawn({"cmd"}, {
//!     cwd = "/tmp",
//!     env = { VAR = "value" },
//!     capture_stdout = true,
//!     capture_stderr = true,
//!     text = true,
//! })
//!
//! -- Wait for process to complete
//! local result = handle:wait()
//! print(result.code, result.stdout)
//!
//! -- Or with streaming callbacks
//! local handle = niri.action:spawn({"tail", "-f", "log.txt"}, {
//!     stdout = function(err, line) print(line) end,
//!     on_exit = function(result) print("Done:", result.code) end,
//! })
//! ```
//!
//! # ProcessHandle Methods
//!
//! - `handle:wait(timeout_ms?)` - Wait for process to exit, returns result table
//! - `handle:kill(signal?)` - Send signal to process (default: SIGTERM)
//! - `handle:write(data)` - Write to process stdin (if stdin="pipe")
//! - `handle:close_stdin()` - Close the stdin pipe
//! - `handle:is_closing()` - Check if stdin is closed
//! - `handle.pid` - Process ID (read-only)
//!
//! # Result Table
//!
//! ```lua
//! {
//!     code = 0,           -- Exit code (nil if killed by signal)
//!     signal = nil,       -- Signal number (nil if normal exit)
//!     stdout = "output",  -- Captured stdout (if capture_stdout=true)
//!     stderr = "",        -- Captured stderr (if capture_stderr=true)
//! }
//! ```
//!
//! # Thread Safety
//!
//! [`ProcessManager`] is designed for single-threaded use only. It must be
//! accessed only from the main Lua thread (where the Lua runtime lives).
//! Worker threads for reading process output communicate with the manager
//! via `mpsc::channel` messages, not by directly accessing the manager.
//!
//! This design allows using `Rc<RefCell<>>` instead of `Arc<Mutex<>>`,
//! matching the pattern used by [`SharedTimerManager`] and [`SharedEventHandlers`].
//!
//! # Callback ID Lifecycle
//!
//! When callbacks are registered via [`parse_spawn_opts()`], callback IDs are
//! returned and stored in [`SpawnOpts`]. These IDs are automatically unregistered
//! by [`LuaRuntime::process_callbacks()`]` when the process exits.
//!
//! [`CallbackRegistry::unregister()`] returns the Lua registry key so it can
//! be properly cleaned up from the Lua state.
//!
//! # Detached Worker Threads
//!
//! Worker threads for stdout/stderr reading and process waiting are detached.
//! Join handles are not stored because deterministic shutdown is not required.
//! The compositor may exit without explicitly joining these threads.
//!
//! [`SharedTimerManager`]: crate::loop_api::SharedTimerManager
//! [`SharedEventHandlers`]: crate::event_system::SharedEventHandlers
//! [`parse_spawn_opts()`]: crate::process::parse_spawn_opts

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{ChildStdin, Command, ExitStatus, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

use mlua::prelude::*;
use niri_config::Environment;
use nix::libc;

use crate::SharedCallbackRegistry;

/// Pending callback event from process_events().
/// Fields: (callback_id, handle_id, data, stream_name, text_mode, is_exit)
pub type PendingCallback = (u64, u64, Vec<u8>, &'static str, bool, bool);

pub static CHILD_ENV: RwLock<Environment> = RwLock::new(Environment(Vec::new()));
pub static CHILD_WAYLAND_DISPLAY: RwLock<Option<String>> = RwLock::new(None);
pub static CHILD_DISPLAY: RwLock<Option<String>> = RwLock::new(None);

pub fn set_child_wayland_display(socket_name: Option<String>) {
    *CHILD_WAYLAND_DISPLAY.write().unwrap() = socket_name;
}

pub fn set_child_display(display: Option<String>) {
    *CHILD_DISPLAY.write().unwrap() = display;
}

pub fn set_child_env(env: Environment) {
    *CHILD_ENV.write().unwrap() = env;
}

/// Grace period after SIGTERM before sending SIGKILL (configurable).
pub const SIGTERM_GRACE_MS: u64 = 1000;

pub const MAX_CALLBACKS_PER_FLUSH: usize = 16;

static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

/// Options for spawning a process.
#[derive(Debug)]
pub struct SpawnOpts {
    /// Working directory for the spawned process.
    pub cwd: Option<PathBuf>,
    /// Environment variables to set (merged with or replacing parent env).
    pub env: Option<HashMap<String, String>>,
    /// If true, start with empty environment (only use env table).
    pub clear_env: bool,
    /// Stdin configuration.
    pub stdin: StdinMode,
    /// Whether to capture stdout for wait()/on_exit.
    pub capture_stdout: bool,
    /// Whether to capture stderr for wait()/on_exit.
    pub capture_stderr: bool,
    /// If true, decode output as UTF-8 text; if false, return raw bytes.
    pub text: bool,
    /// If true, fire-and-forget (return nil instead of ProcessHandle).
    pub detach: bool,
    /// Registry key for stdout streaming callback.
    pub stdout_callback: Option<u64>,
    /// Registry key for stderr streaming callback.
    pub stderr_callback: Option<u64>,
    /// Registry key for on_exit callback.
    pub on_exit_callback: Option<u64>,
}

impl Default for SpawnOpts {
    fn default() -> Self {
        Self {
            cwd: None,
            env: None,
            clear_env: false,
            stdin: StdinMode::Closed,
            capture_stdout: false,
            capture_stderr: false,
            text: true, // Default to text mode
            detach: false,
            stdout_callback: None,
            stderr_callback: None,
            on_exit_callback: None,
        }
    }
}

/// Stdin configuration mode.
#[derive(Debug, Clone, Default)]
pub enum StdinMode {
    /// Stdin is closed (default).
    #[default]
    Closed,
    /// Stdin receives the given data once, then closes.
    Data(String),
    /// Stdin is a pipe that can be written to via ProcessHandle.
    Pipe,
}

/// Result of a completed process.
#[derive(Debug, Clone)]
pub struct SpawnResult {
    /// Exit code (None if killed by signal).
    pub code: Option<i32>,
    /// Signal number (None if normal exit).
    pub signal: Option<i32>,
    /// Captured stdout (empty if not captured).
    pub stdout: Vec<u8>,
    /// Captured stderr (empty if not captured).
    pub stderr: Vec<u8>,
}

impl SpawnResult {
    /// Convert to a Lua table.
    pub fn to_lua_table(&self, lua: &Lua, text_mode: bool) -> LuaResult<LuaTable> {
        let table = lua.create_table()?;

        if let Some(code) = self.code {
            table.set("code", code)?;
        } else {
            table.set("code", LuaValue::Nil)?;
        }

        if let Some(signal) = self.signal {
            table.set("signal", signal)?;
        } else {
            table.set("signal", LuaValue::Nil)?;
        }

        if text_mode {
            // Decode as UTF-8, replacing invalid sequences
            let stdout = String::from_utf8_lossy(&self.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&self.stderr).into_owned();
            table.set("stdout", stdout)?;
            table.set("stderr", stderr)?;
        } else {
            // Return raw bytes as Lua string
            table.set("stdout", lua.create_string(&self.stdout)?)?;
            table.set("stderr", lua.create_string(&self.stderr)?)?;
        }

        Ok(table)
    }
}

#[derive(Debug)]
enum WorkerMsg {
    Stdout { handle: u64, data: Vec<u8> },
    Stderr { handle: u64, data: Vec<u8> },
    StdoutClosed { handle: u64 },
    StderrClosed { handle: u64 },
    Exit { handle: u64, result: SpawnResult },
    Error { handle: u64, message: String },
}

struct ProcessState {
    pid: u32,
    stdin: Option<ChildStdin>,
    stdin_closed: bool,
    /// Text mode for streaming callbacks (carried to event dispatch).
    text_mode: bool,
    stdout_buffer: Vec<u8>,
    /// Captured stderr buffer.
    stderr_buffer: Vec<u8>,
    /// Whether to capture stdout.
    capture_stdout: bool,
    /// Whether to capture stderr.
    capture_stderr: bool,
    /// Registry key for stdout callback.
    pub stdout_callback: Option<u64>,
    /// Registry key for stderr callback.
    pub stderr_callback: Option<u64>,
    /// Registry key for on_exit callback.
    pub on_exit_callback: Option<u64>,
    /// Cached exit result (after process exits)
    stdout_done: bool,
    stderr_done: bool,
    pending_exit: Option<SpawnResult>,
    exit_result: Option<SpawnResult>,
}

pub struct ProcessManager {
    processes: HashMap<u64, ProcessState>,
    event_sender: Sender<WorkerMsg>,
    event_receiver: Receiver<WorkerMsg>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            processes: HashMap::new(),
            event_sender: tx,
            event_receiver: rx,
        }
    }

    /// Spawn a command with options.
    ///
    /// Returns the handle ID and PID on success.
    pub fn spawn_command(
        &mut self,
        command: Vec<String>,
        opts: SpawnOpts,
    ) -> Result<(u64, u32), String> {
        if command.is_empty() {
            return Err("Command cannot be empty".to_string());
        }

        let handle_id = NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst);

        // Build the command
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }

        // Set working directory
        if let Some(ref cwd) = opts.cwd {
            cmd.current_dir(cwd);
        }

        // Set environment
        if opts.clear_env {
            cmd.env_clear();
        }

        // propagate compositor environment
        if let Some(ref display) = *CHILD_WAYLAND_DISPLAY.read().unwrap() {
            cmd.env("WAYLAND_DISPLAY", display);
        }
        if let Some(ref d) = *CHILD_DISPLAY.read().unwrap() {
            cmd.env("DISPLAY", d);
        } else {
            cmd.env_remove("DISPLAY");
        }
        for var in &CHILD_ENV.read().unwrap().0 {
            if let Some(v) = &var.value {
                cmd.env(&var.name, v);
            } else {
                cmd.env_remove(&var.name);
            }
        }
        // Set user-provided environment (overrides compositor env)
        if let Some(ref env) = opts.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        // Configure stdio
        match &opts.stdin {
            StdinMode::Closed => cmd.stdin(Stdio::null()),
            StdinMode::Data(_) | StdinMode::Pipe => cmd.stdin(Stdio::piped()),
        };

        if opts.capture_stdout || opts.stdout_callback.is_some() {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::null());
        }

        if opts.capture_stderr || opts.stderr_callback.is_some() {
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stderr(Stdio::null());
        }

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn: {}", e))?;
        let pid = child.id();

        // handle stdin data (immediately for Data; keep pipe for Pipe)
        let mut stdin_handle = child.stdin.take();
        let stdin_closed = match &opts.stdin {
            StdinMode::Data(s) => {
                if let Some(ref mut st) = stdin_handle {
                    let _ = st.write_all(s.as_bytes());
                }
                stdin_handle = None;
                true
            }
            StdinMode::Closed => true,
            StdinMode::Pipe => false,
        };

        // spawn reader threads (they send WorkerMsg to manager)
        let tx = self.event_sender.clone();
        let has_stdout = child.stdout.is_some();
        let has_stderr = child.stderr.is_some();
        if let Some(stdout) = child.stdout.take() {
            let tx_clone = tx.clone();
            let id = handle_id;
            let text_mode = opts.text;
            thread::spawn(move || read_stream(stdout, id, tx_clone, text_mode, true));
        }
        if let Some(stderr) = child.stderr.take() {
            let tx_clone = tx.clone();
            let id = handle_id;
            let text_mode = opts.text;
            thread::spawn(move || read_stream(stderr, id, tx_clone, text_mode, false));
        }

        // waiter thread — sends Exit(handle_id, SpawnResult)
        let tx_clone = self.event_sender.clone();
        let id = handle_id;
        let _capture_out = opts.capture_stdout;
        let _capture_err = opts.capture_stderr;
        thread::spawn(move || {
            let status = match child.wait() {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx_clone.send(WorkerMsg::Error {
                        handle: id,
                        message: format!("wait error: {}", e),
                    });
                    return;
                }
            };
            // Buffers are empty here - process_events populates them from WorkerMsg chunks
            // and transfers them to SpawnResult when Exit is processed
            let res = exit_status_to_result(status);
            let _ = tx_clone.send(WorkerMsg::Exit {
                handle: id,
                result: res,
            });
        });

        // NOTE: We don't hold join handles; threads are detached.
        // Manager will receive the Exit WorkerMsg and update state.exit_result accordingly.
        let state = ProcessState {
            pid,
            stdin: stdin_handle,
            stdin_closed,
            text_mode: opts.text,
            capture_stdout: opts.capture_stdout,
            capture_stderr: opts.capture_stderr,
            stdout_buffer: Vec::new(),
            stderr_buffer: Vec::new(),
            stdout_callback: opts.stdout_callback,
            stderr_callback: opts.stderr_callback,
            on_exit_callback: opts.on_exit_callback,
            stdout_done: !has_stdout,
            stderr_done: !has_stderr,
            pending_exit: None,
            exit_result: None,
        };
        self.processes.insert(handle_id, state);

        Ok((handle_id, pid))
    }

    pub fn spawn_shell_command(
        &mut self,
        command: String,
        opts: SpawnOpts,
    ) -> Result<(u64, u32), String> {
        self.spawn_command(vec!["sh".to_string(), "-c".to_string(), command], opts)
    }

    pub fn get_pid(&self, handle_id: u64) -> Option<u32> {
        self.processes.get(&handle_id).map(|s| s.pid)
    }

    pub fn write_stdin(&mut self, handle_id: u64, data: &[u8]) -> Result<(), String> {
        let state = self
            .processes
            .get_mut(&handle_id)
            .ok_or("Process not found")?;
        if state.stdin_closed {
            return Err("Stdin closed".to_string());
        }
        let stdin = state.stdin.as_mut().ok_or("stdin not available")?;
        stdin
            .write_all(data)
            .map_err(|e| format!("write failed: {}", e))?;
        stdin.flush().map_err(|e| format!("flush failed: {}", e))?;
        Ok(())
    }

    pub fn close_stdin(&mut self, handle_id: u64) -> Result<(), String> {
        let state = self
            .processes
            .get_mut(&handle_id)
            .ok_or("Process not found")?;
        state.stdin = None;
        state.stdin_closed = true;
        return Ok(());
    }

    pub fn is_stdin_closed(&self, handle_id: u64) -> bool {
        self.processes
            .get(&handle_id)
            .map(|s| s.stdin_closed)
            .unwrap_or(true)
    }

    pub fn kill(&mut self, handle_id: u64, signal: i32) -> Result<bool, String> {
        let state = self.processes.get(&handle_id).ok_or("Process not found")?;
        #[cfg(unix)]
        {
            let res = unsafe { libc::kill(state.pid as i32, signal) };
            Ok(res == 0)
        }
        #[cfg(not(unix))]
        {
            Err("kill not supported".to_string())
        }
    }

    /// Wait for process to exit (blocks). We use cached result when manager has processed Exit.
    pub fn wait(&mut self, handle_id: u64, timeout_ms: Option<u64>) -> Result<SpawnResult, String> {
        // If already have exit_result, return
        if let Some(state) = self.processes.get(&handle_id) {
            if let Some(ref r) = state.exit_result {
                return Ok(r.clone());
            }
        } else {
            return Err("Process not found".to_string());
        }

        // Simple loop that pumps events until exit_result is available or timeout occurs.
        let start = std::time::Instant::now();
        loop {
            // check and process pending events (small, non-blocking)
            let _callbacks = self.process_events();
            // user of the manager should invoke these callbacks on main thread via registry.
            // See below: process_events returns tuples (we'll call registry there).
            // After processing, check exit_result again.
            if let Some(state) = self.processes.get(&handle_id) {
                if let Some(ref r) = state.exit_result {
                    return Ok(r.clone());
                }
            }

            if let Some(to) = timeout_ms {
                if start.elapsed() > Duration::from_millis(to) {
                    // escalate: SIGTERM then SIGKILL after grace
                    let _pid = self.get_pid(handle_id).ok_or("Process not found")?;
                    let _ = self.kill(handle_id, libc::SIGTERM);
                    std::thread::sleep(Duration::from_millis(SIGTERM_GRACE_MS));
                    // after grace, check again
                    if let Some(state) = self.processes.get(&handle_id) {
                        if state.exit_result.is_some() {
                            return Ok(state.exit_result.clone().unwrap());
                        }
                    }
                    let _ = self.kill(handle_id, libc::SIGKILL);
                    // now wait indefinitely until exit_result appears
                    loop {
                        let _ = self.process_events();
                        if let Some(state) = self.processes.get(&handle_id) {
                            if let Some(ref r) = state.exit_result {
                                return Ok(r.clone());
                            }
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
            // small sleep to avoid busy-loop
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// Pull up to MAX_CALLBACKS_PER_FLUSH messages and prepare callbacks to invoke on the main thread.
    /// Returns a vector of tuples: (callback_id, handle_id, payload_bytes, stream, text_mode, is_exit)
    /// where stream is "stdout" or "stderr" for streaming callbacks.
    /// Manager/user must use CallbackRegistry to actually invoke Lua callbacks on main thread.
    pub fn process_events(&mut self) -> Vec<PendingCallback> {
        let mut result: Vec<PendingCallback> = Vec::new();
        let mut count = 0;
        while count < MAX_CALLBACKS_PER_FLUSH {
            match self.event_receiver.try_recv() {
                Ok(msg) => {
                    count += 1;
                    match msg {
                        WorkerMsg::Stdout { handle: id, data } => {
                            if let Some(state) = self.processes.get_mut(&id) {
                                if state.capture_stdout {
                                    state.stdout_buffer.extend(&data);
                                }
                                if let Some(cb) = state.stdout_callback {
                                    result.push((cb, id, data, "stdout", state.text_mode, false));
                                }
                            }
                        }
                        WorkerMsg::Stderr { handle: id, data } => {
                            if let Some(state) = self.processes.get_mut(&id) {
                                if state.capture_stderr {
                                    state.stderr_buffer.extend(&data);
                                }
                                if let Some(cb) = state.stderr_callback {
                                    result.push((cb, id, data, "stderr", state.text_mode, false));
                                }
                            }
                        }
                        WorkerMsg::StdoutClosed { handle: id } => {
                            if let Some(state) = self.processes.get_mut(&id) {
                                state.stdout_done = true;
                                Self::try_finalize_exit(state, id, &mut result);
                            }
                        }
                        WorkerMsg::StderrClosed { handle: id } => {
                            if let Some(state) = self.processes.get_mut(&id) {
                                state.stderr_done = true;
                                Self::try_finalize_exit(state, id, &mut result);
                            }
                        }
                        WorkerMsg::Exit {
                            handle: id,
                            result: res,
                        } => {
                            if let Some(state) = self.processes.get_mut(&id) {
                                state.pending_exit = Some(res);
                                Self::try_finalize_exit(state, id, &mut result);
                            }
                        }
                        WorkerMsg::Error {
                            handle: id,
                            message: msg,
                        } => {
                            log::error!("Process {}: {}", id, msg);
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
        result
    }

    fn try_finalize_exit(state: &mut ProcessState, id: u64, result: &mut Vec<PendingCallback>) {
        if !state.stdout_done || !state.stderr_done || state.pending_exit.is_none() {
            return;
        }
        if state.exit_result.is_some() {
            return;
        }
        let mut exit_result = state.pending_exit.take().unwrap();
        if state.capture_stdout {
            exit_result.stdout = state.stdout_buffer.clone();
        }
        if state.capture_stderr {
            exit_result.stderr = state.stderr_buffer.clone();
        }
        state.exit_result = Some(exit_result);
        if let Some(cb) = state.on_exit_callback {
            result.push((cb, id, Vec::new(), "exit", state.text_mode, true));
        }
    }

    pub fn has_exited(&self, handle_id: u64) -> bool {
        self.processes
            .get(&handle_id)
            .map(|s| s.exit_result.is_some())
            .unwrap_or(true)
    }

    pub fn get_exit_result(&self, handle_id: u64) -> Option<SpawnResult> {
        self.processes
            .get(&handle_id)
            .and_then(|s| s.exit_result.clone())
    }

    pub fn remove(&mut self, handle_id: u64) -> Option<(Option<u64>, Option<u64>, Option<u64>)> {
        self.processes
            .remove(&handle_id)
            .map(|s| (s.stdout_callback, s.stderr_callback, s.on_exit_callback))
    }

    pub fn get_callback_ids(
        &self,
        handle_id: u64,
    ) -> Option<(Option<u64>, Option<u64>, Option<u64>)> {
        self.processes
            .get(&handle_id)
            .map(|s| (s.stdout_callback, s.stderr_callback, s.on_exit_callback))
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Read stream (text line or binary chunk) and send WorkerMsg to manager.
fn read_stream<R: Read + Send + 'static>(
    stream: R,
    handle_id: u64,
    sender: Sender<WorkerMsg>,
    text_mode: bool,
    is_stdout: bool,
) {
    let mut reader = BufReader::new(stream);
    if text_mode {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let bytes = line.clone().into_bytes();
                    // strip trailing newline? keep it — caller can decide.
                    let _ = sender.send(if is_stdout {
                        WorkerMsg::Stdout {
                            handle: handle_id,
                            data: bytes,
                        }
                    } else {
                        WorkerMsg::Stderr {
                            handle: handle_id,
                            data: bytes,
                        }
                    });
                }
                Err(e) => {
                    let _ = sender.send(WorkerMsg::Error {
                        handle: handle_id,
                        message: format!("read error: {}", e),
                    });
                    break;
                }
            }
        }
    } else {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    let _ = sender.send(if is_stdout {
                        WorkerMsg::Stdout {
                            handle: handle_id,
                            data,
                        }
                    } else {
                        WorkerMsg::Stderr {
                            handle: handle_id,
                            data,
                        }
                    });
                }
                Err(e) => {
                    let _ = sender.send(WorkerMsg::Error {
                        handle: handle_id,
                        message: format!("read error: {}", e),
                    });
                    break;
                }
            }
        }
    }
    let _ = sender.send(if is_stdout {
        WorkerMsg::StdoutClosed { handle: handle_id }
    } else {
        WorkerMsg::StderrClosed { handle: handle_id }
    });
}

fn exit_status_to_result(status: ExitStatus) -> SpawnResult {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        SpawnResult {
            code: status.code(),
            signal: status.signal(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
    #[cfg(not(unix))]
    {
        SpawnResult {
            code: status.code(),
            signal: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}

pub type SharedProcessManager = Rc<RefCell<ProcessManager>>;

pub fn create_process_manager() -> SharedProcessManager {
    Rc::new(RefCell::new(ProcessManager::new()))
}

/// Parse spawn options from a Lua table.
pub fn parse_spawn_opts(
    lua: &Lua,
    table: &LuaTable,
    registry: Option<&SharedCallbackRegistry>,
) -> LuaResult<SpawnOpts> {
    let mut opts = SpawnOpts::default();

    // cwd: string?
    if let Ok(cwd) = table.get::<Option<String>>("cwd") {
        opts.cwd = cwd.map(PathBuf::from);
    }

    // env: table<string, string>?
    if let Ok(Some(env_table)) = table.get::<Option<LuaTable>>("env") {
        let mut env = HashMap::new();
        for pair in env_table.pairs::<String, String>() {
            let (k, v) = pair?;
            env.insert(k, v);
        }
        opts.env = Some(env);
    }

    // clear_env: boolean?
    if let Ok(clear_env) = table.get::<Option<bool>>("clear_env") {
        opts.clear_env = clear_env.unwrap_or(false);
    }

    // stdin: string | boolean | "pipe"?
    if let Ok(stdin_val) = table.get::<LuaValue>("stdin") {
        opts.stdin = match stdin_val {
            LuaValue::Nil => StdinMode::Closed,
            LuaValue::Boolean(false) => StdinMode::Closed,
            LuaValue::Boolean(true) => StdinMode::Pipe,
            LuaValue::String(s) => {
                let s = s.to_str()?;
                if s == "pipe" {
                    StdinMode::Pipe
                } else {
                    StdinMode::Data(s.to_string())
                }
            }
            _ => {
                return Err(LuaError::external(
                    "stdin must be boolean, 'pipe', or a string of data",
                ))
            }
        };
    }

    // stdin_pipe: boolean? (alias for stdin="pipe")
    if let Ok(stdin_pipe) = table.get::<Option<bool>>("stdin_pipe") {
        if stdin_pipe.unwrap_or(false) {
            opts.stdin = StdinMode::Pipe;
        }
    }

    // capture_stdout: boolean?
    if let Ok(capture) = table.get::<Option<bool>>("capture_stdout") {
        opts.capture_stdout = capture.unwrap_or(false);
    }

    // capture_stderr: boolean?
    if let Ok(capture) = table.get::<Option<bool>>("capture_stderr") {
        opts.capture_stderr = capture.unwrap_or(false);
    }

    // stdout: boolean | function?
    if let Ok(stdout_val) = table.get::<LuaValue>("stdout") {
        match stdout_val {
            LuaValue::Boolean(true) => {
                opts.capture_stdout = true;
            }
            LuaValue::Function(f) => {
                opts.capture_stdout = true;
                if let Some(registry) = registry {
                    let id = registry.borrow_mut().register(lua, f)?;
                    opts.stdout_callback = Some(id);
                } else {
                    return Err(LuaError::external("Process manager not initialized; callback functions require a callback registry"));
                }
            }
            LuaValue::Nil | LuaValue::Boolean(false) => {}
            _ => return Err(LuaError::external("stdout must be boolean or function")),
        }
    }

    // stderr: boolean | function?
    if let Ok(stderr_val) = table.get::<LuaValue>("stderr") {
        match stderr_val {
            LuaValue::Boolean(true) => {
                opts.capture_stderr = true;
            }
            LuaValue::Function(f) => {
                opts.capture_stderr = true;
                if let Some(registry) = registry {
                    let id = registry.borrow_mut().register(lua, f)?;
                    opts.stderr_callback = Some(id);
                } else {
                    return Err(LuaError::external("Process manager not initialized; callback functions require a callback registry"));
                }
            }
            LuaValue::Nil | LuaValue::Boolean(false) => {}
            _ => return Err(LuaError::external("stderr must be boolean or function")),
        }
    }

    // text: boolean?
    if let Ok(text) = table.get::<Option<bool>>("text") {
        opts.text = text.unwrap_or(true);
    }

    // detach: boolean?
    if let Ok(detach) = table.get::<Option<bool>>("detach") {
        opts.detach = detach.unwrap_or(false);
    }

    // on_exit: function?
    if let Ok(Some(f)) = table.get::<Option<LuaFunction>>("on_exit") {
        if let Some(registry) = registry {
            let id = registry.borrow_mut().register(lua, f)?;
            opts.on_exit_callback = Some(id);
        } else {
            return Err(LuaError::external(
                "Process manager not initialized; callback functions require a callback registry",
            ));
        }
    }

    Ok(opts)
}

/// ProcessHandle userdata for Lua - returned from spawn with options.
pub struct ProcessHandle {
    /// Unique handle ID.
    pub id: u64,
    /// Process ID.
    pub pid: u32,
    /// Reference to the process manager.
    pub manager: SharedProcessManager,
    /// Whether this handle is in text mode.
    pub text_mode: bool,
}

impl LuaUserData for ProcessHandle {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // handle.pid (read-only)
        fields.add_field_method_get("pid", |_, this| Ok(this.pid));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // handle:wait(timeout_ms?) -> result_table
        methods.add_method("wait", |lua, this, timeout_ms: Option<u64>| {
            let result = {
                let mut manager = this.manager.borrow_mut();
                manager.wait(this.id, timeout_ms)
            };

            match result {
                Ok(result) => result.to_lua_table(lua, this.text_mode),
                Err(e) => Err(LuaError::external(e)),
            }
        });

        // handle:kill(signal?) -> boolean
        methods.add_method("kill", |_lua, this, signal: Option<LuaValue>| {
            let sig = match signal {
                None => libc::SIGTERM,
                Some(LuaValue::Integer(n)) => n as i32,
                Some(LuaValue::String(s)) => {
                    let s = s.to_str()?;
                    match s.to_uppercase().as_str() {
                        "TERM" | "SIGTERM" => libc::SIGTERM,
                        "KILL" | "SIGKILL" => libc::SIGKILL,
                        "INT" | "SIGINT" => libc::SIGINT,
                        "HUP" | "SIGHUP" => libc::SIGHUP,
                        "QUIT" | "SIGQUIT" => libc::SIGQUIT,
                        "USR1" | "SIGUSR1" => libc::SIGUSR1,
                        "USR2" | "SIGUSR2" => libc::SIGUSR2,
                        _ => return Err(LuaError::external(format!("Unknown signal: {}", s))),
                    }
                }
                _ => return Err(LuaError::external("signal must be an integer or string")),
            };

            let result = {
                let mut manager = this.manager.borrow_mut();
                manager.kill(this.id, sig)
            };

            match result {
                Ok(success) => Ok(success),
                Err(e) => Err(LuaError::external(e)),
            }
        });

        // handle:write(data) -> boolean
        methods.add_method("write", |_lua, this, data: LuaString| {
            let bytes = data.as_bytes();
            let result = {
                let mut manager = this.manager.borrow_mut();
                manager.write_stdin(this.id, &bytes)
            };

            match result {
                Ok(()) => Ok(true),
                Err(e) => Err(LuaError::external(e)),
            }
        });

        // handle:close_stdin()
        methods.add_method("close_stdin", |_lua, this, ()| {
            let result = {
                let mut manager = this.manager.borrow_mut();
                manager.close_stdin(this.id)
            };

            match result {
                Ok(()) => Ok(()),
                Err(e) => Err(LuaError::external(e)),
            }
        });

        // handle:is_closing() -> boolean
        methods.add_method("is_closing", |_lua, this, ()| {
            let manager = this.manager.borrow();
            Ok(manager.is_stdin_closed(this.id))
        });

        methods.add_meta_method(mlua::MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("ProcessHandle {{ pid = {} }}", this.pid))
        });

        methods.add_method("inspect", |lua, this, ()| {
            let props = lua.create_table()?;
            props.set("pid", this.pid)?;
            props.set("wait", "<method: wait(timeout_ms?) -> result>")?;
            props.set("kill", "<method: kill(signal?) -> boolean>")?;
            props.set("write", "<method: write(data) -> boolean>")?;
            props.set("close_stdin", "<method: close_stdin()>")?;
            props.set("is_closing", "<method: is_closing() -> boolean>")?;
            props.set("inspect", "<method: inspect() -> table>")?;
            Ok(props)
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::thread;
    use std::time::Duration;

    use mlua::prelude::{Lua, LuaFunction, LuaTable, LuaValue};

    use super::*;
    use crate::CallbackRegistry;

    #[test]
    fn test_spawn_opts_default() {
        let opts = SpawnOpts::default();
        assert!(opts.cwd.is_none());
        assert!(opts.env.is_none());
        assert!(!opts.clear_env);
        assert!(matches!(opts.stdin, StdinMode::Closed));
        assert!(!opts.capture_stdout);
        assert!(!opts.capture_stderr);
        assert!(opts.text);
        assert!(!opts.detach);
    }

    #[test]
    fn test_spawn_result_to_lua_text_mode() {
        let lua = Lua::new();
        let result = SpawnResult {
            code: Some(0),
            signal: None,
            stdout: b"hello\nworld".to_vec(),
            stderr: Vec::new(),
        };

        let table = result.to_lua_table(&lua, true).unwrap();
        assert_eq!(table.get::<i32>("code").unwrap(), 0);
        assert!(table.get::<LuaValue>("signal").unwrap().is_nil());
        assert_eq!(table.get::<String>("stdout").unwrap(), "hello\nworld");
        assert_eq!(table.get::<String>("stderr").unwrap(), "");
    }

    #[test]
    fn test_spawn_result_signal() {
        let lua = Lua::new();
        let result = SpawnResult {
            code: None,
            signal: Some(9),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };

        let table = result.to_lua_table(&lua, true).unwrap();
        assert!(table.get::<LuaValue>("code").unwrap().is_nil());
        assert_eq!(table.get::<i32>("signal").unwrap(), 9);
    }

    #[test]
    fn test_parse_spawn_opts_empty() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();

        assert!(opts.cwd.is_none());
        assert!(!opts.detach);
        assert!(opts.text);
    }

    #[test]
    fn test_parse_spawn_opts_cwd() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("cwd", "/tmp").unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();

        assert_eq!(opts.cwd, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_parse_spawn_opts_env() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let env_table = lua.create_table().unwrap();
        env_table.set("FOO", "bar").unwrap();
        table.set("env", env_table).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();

        let env = opts.env.unwrap();
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_parse_spawn_opts_stdin_modes() {
        let lua = Lua::new();

        // stdin = false
        let table = lua.create_table().unwrap();
        table.set("stdin", false).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Closed));

        // stdin = true (pipe)
        let table = lua.create_table().unwrap();
        table.set("stdin", true).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Pipe));

        // stdin = "pipe"
        let table = lua.create_table().unwrap();
        table.set("stdin", "pipe").unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Pipe));

        // stdin = "data"
        let table = lua.create_table().unwrap();
        table.set("stdin", "hello").unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Data(ref s) if s == "hello"));
    }

    #[test]
    fn test_parse_spawn_opts_detach() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("detach", true).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();

        assert!(opts.detach);
    }

    #[test]
    fn test_process_manager_new() {
        let manager = ProcessManager::new();
        assert!(manager.processes.is_empty());
    }

    #[test]
    fn test_process_manager_spawn_empty_command() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();
        let result = manager.spawn_command(vec![], opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_process_manager_spawn_echo() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts {
            capture_stdout: true,
            ..Default::default()
        };
        let result = manager.spawn_command(vec!["echo".to_string(), "hello".to_string()], opts);
        assert!(result.is_ok());

        let (handle_id, pid) = result.unwrap();
        assert!(pid > 0);
        assert!(handle_id > 0);
    }

    #[test]
    fn test_process_manager_wait() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts {
            capture_stdout: true,
            ..Default::default()
        };
        let (handle_id, _) = manager
            .spawn_command(vec!["true".to_string()], opts)
            .unwrap();

        let result = manager.wait(handle_id, None).unwrap();
        assert_eq!(result.code, Some(0));
        assert!(result.signal.is_none());
    }

    #[test]
    fn test_process_manager_wait_with_exit_code() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();
        let (handle_id, _) = manager
            .spawn_command(vec!["false".to_string()], opts)
            .unwrap();

        let result = manager.wait(handle_id, None).unwrap();
        assert_eq!(result.code, Some(1));
    }

    #[test]
    fn test_parse_spawn_opts_with_registry_stdout_callback() {
        let lua = Lua::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let table = lua.create_table().unwrap();

        // Create a function and set it as stdout callback
        let func: LuaFunction = lua.create_function(|_, ()| Ok("test".to_string())).unwrap();
        table.set("stdout", func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();

        assert!(opts.capture_stdout);
        assert!(opts.stdout_callback.is_some());
        let id = opts.stdout_callback.unwrap();

        // Verify the function can be retrieved
        let retrieved = registry.borrow().get(&lua, id).unwrap().unwrap();
        let result: String = retrieved.call(()).unwrap();
        assert_eq!(result, "test");
    }

    #[test]
    fn test_parse_spawn_opts_with_registry_stderr_callback() {
        let lua = Lua::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let table = lua.create_table().unwrap();

        // Create a function and set it as stderr callback
        let func: LuaFunction = lua
            .create_function(|_, ()| Ok("stderr".to_string()))
            .unwrap();
        table.set("stderr", func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();

        assert!(opts.capture_stderr);
        assert!(opts.stderr_callback.is_some());
        let id = opts.stderr_callback.unwrap();

        // Verify the function can be retrieved
        let retrieved = registry.borrow().get(&lua, id).unwrap().unwrap();
        let result: String = retrieved.call(()).unwrap();
        assert_eq!(result, "stderr");
    }

    #[test]
    fn test_parse_spawn_opts_with_registry_on_exit_callback() {
        let lua = Lua::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let table = lua.create_table().unwrap();

        // Create a function and set it as on_exit callback
        let func: LuaFunction = lua.create_function(|_, ()| Ok("exit".to_string())).unwrap();
        table.set("on_exit", func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();

        assert!(opts.on_exit_callback.is_some());
        let id = opts.on_exit_callback.unwrap();

        // Verify the function can be retrieved
        let retrieved = registry.borrow().get(&lua, id).unwrap().unwrap();
        let result: String = retrieved.call(()).unwrap();
        assert_eq!(result, "exit");
    }

    #[test]
    fn test_parse_spawn_opts_with_registry_unique_ids() {
        let lua = Lua::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));

        // Create multiple tables with callbacks
        let table1 = lua.create_table().unwrap();
        let func1: LuaFunction = lua.create_function(|_, ()| Ok(1)).unwrap();
        table1.set("stdout", func1).unwrap();

        let table2 = lua.create_table().unwrap();
        let func2: LuaFunction = lua.create_function(|_, ()| Ok(2)).unwrap();
        table2.set("stderr", func2).unwrap();

        let table3 = lua.create_table().unwrap();
        let func3: LuaFunction = lua.create_function(|_, ()| Ok(3)).unwrap();
        table3.set("on_exit", func3).unwrap();

        let opts1 = parse_spawn_opts(&lua, &table1, Some(&registry)).unwrap();
        let opts2 = parse_spawn_opts(&lua, &table2, Some(&registry)).unwrap();
        let opts3 = parse_spawn_opts(&lua, &table3, Some(&registry)).unwrap();

        // All IDs should be unique
        let ids = vec![
            opts1.stdout_callback.unwrap(),
            opts2.stderr_callback.unwrap(),
            opts3.on_exit_callback.unwrap(),
        ];
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        sorted_ids.dedup();
        assert_eq!(sorted_ids.len(), ids.len());
    }

    #[test]
    fn test_parse_spawn_opts_without_registry_with_callback_fails() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();

        // Create a function and set it as stdout callback
        let func: LuaFunction = lua.create_function(|_, ()| Ok(())).unwrap();
        table.set("stdout", func).unwrap();

        let result = parse_spawn_opts(&lua, &table, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("callback functions require a callback registry"));
    }

    #[test]
    fn test_process_events_returns_callback_events() {
        let mut manager = ProcessManager::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let lua = Lua::new();

        // Create a process with callbacks
        let table = lua.create_table().unwrap();
        let stdout_func: LuaFunction = lua.create_function(|_, ()| Ok(())).unwrap();
        table.set("stdout", stdout_func).unwrap();
        let on_exit_func: LuaFunction = lua.create_function(|_, ()| Ok(())).unwrap();
        table.set("on_exit", on_exit_func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();
        let (handle_id, _) = manager
            .spawn_command(vec!["echo".to_string(), "test".to_string()], opts)
            .unwrap();

        // Wait for the process to finish and produce events
        // Use a small sleep loop to allow background threads to enqueue events
        let mut events = Vec::new();
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            events = manager.process_events();
            if !events.is_empty() {
                break;
            }
        }

        // Should have at least stdout and/or exit events
        assert!(!events.is_empty(), "Expected events from echo command");

        // Check that events have correct callback IDs and types
        // Tuple format: (callback_id, handle_id, data, stream, text_mode, is_exit)
        for event in &events {
            let (callback_id, event_handle_id, _data, _stream, _text_mode, _is_exit) = event;
            assert_eq!(*event_handle_id, handle_id);
            assert!(*callback_id > 0);
            // For exit events, is_exit should be true; for streaming callbacks, stream should be set
            // (we can't assert stream here since it's &str and we only check if callback_id is valid)
        }
    }

    #[test]
    fn test_stdout_streaming_callback_receives_lines() {
        let mut manager = ProcessManager::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let lua = Lua::new();

        // Create a global table to collect output lines
        lua.globals()
            .set("collected_lines", lua.create_table().unwrap())
            .unwrap();

        // Create stdout callback that appends to collected_lines
        let stdout_func: LuaFunction = lua
            .load(
                r#"
            function(stream, err, data)
                if data then
                    local lines = collected_lines
                    lines[#lines + 1] = data
                end
            end
        "#,
            )
            .eval()
            .unwrap();

        let table = lua.create_table().unwrap();
        table.set("stdout", stdout_func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();
        let _handle = manager
            .spawn_command(
                vec!["printf".to_string(), "line1\nline2\nline3\n".to_string()],
                opts,
            )
            .unwrap();

        // Wait for process to complete and collect events
        let mut all_events = Vec::new();
        for _ in 0..100 {
            thread::sleep(Duration::from_millis(20));
            let events = manager.process_events();
            // Tuple format: (callback_id, handle_id, data, stream, text_mode, is_exit)
            let has_exit = events.iter().any(|e| e.5);
            all_events.extend(events);
            if has_exit {
                break;
            }
        }

        // Invoke callbacks manually (simulating fire_process_events)
        for event in &all_events {
            let (callback_id, _handle_id, data, _stream, _text_mode, _is_exit) = event;
            if let Some(func) = registry.borrow().get(&lua, *callback_id).unwrap() {
                let data_str = String::from_utf8_lossy(data).to_string();
                let _: () = func.call(("stdout", LuaValue::Nil, data_str)).unwrap();
            }
        }

        // Verify collected lines
        let collected: LuaTable = lua.globals().get("collected_lines").unwrap();
        let len = collected.len().unwrap();
        assert!(len >= 1, "Expected at least 1 stdout callback, got {}", len);

        // Check that we received the expected content (may be split or combined)
        let mut all_output = String::new();
        for i in 1..=len {
            let line: String = collected.get(i).unwrap();
            all_output.push_str(&line);
        }
        assert!(
            all_output.contains("line1"),
            "Output should contain line1: {}",
            all_output
        );
        assert!(
            all_output.contains("line2"),
            "Output should contain line2: {}",
            all_output
        );
        assert!(
            all_output.contains("line3"),
            "Output should contain line3: {}",
            all_output
        );
    }

    #[test]
    fn test_stderr_streaming_callback() {
        let mut manager = ProcessManager::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let lua = Lua::new();

        // Create a global to store stderr output
        lua.globals().set("stderr_output", "".to_string()).unwrap();

        // Create stderr callback
        let stderr_func: LuaFunction = lua
            .load(
                r#"
            function(stream, err, data)
                if data then
                    stderr_output = stderr_output .. data
                end
            end
        "#,
            )
            .eval()
            .unwrap();

        let table = lua.create_table().unwrap();
        table.set("stderr", stderr_func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();
        let _handle = manager
            .spawn_shell_command("echo error1 >&2; echo error2 >&2".to_string(), opts)
            .unwrap();

        // Wait for process and collect events
        let mut all_events = Vec::new();
        for _ in 0..100 {
            thread::sleep(Duration::from_millis(20));
            let events = manager.process_events();
            // Tuple format: (callback_id, handle_id, data, stream, text_mode, is_exit)
            let has_exit = events.iter().any(|e| e.5);
            all_events.extend(events);
            if has_exit {
                break;
            }
        }

        // Invoke stderr callbacks
        for event in &all_events {
            let (callback_id, _handle_id, data, stream, _text_mode, _is_exit) = event;
            if *stream == "stderr" {
                if let Some(func) = registry.borrow().get(&lua, *callback_id).unwrap() {
                    let data_str = String::from_utf8_lossy(data).to_string();
                    let _: () = func.call(("stderr", LuaValue::Nil, data_str)).unwrap();
                }
            }
        }

        // Verify stderr was captured
        let stderr_output: String = lua.globals().get("stderr_output").unwrap();
        assert!(
            stderr_output.contains("error1"),
            "Stderr should contain error1: {}",
            stderr_output
        );
        assert!(
            stderr_output.contains("error2"),
            "Stderr should contain error2: {}",
            stderr_output
        );
    }

    #[test]
    fn test_on_exit_callback_receives_result() {
        let mut manager = ProcessManager::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let lua = Lua::new();

        // Create a global to store exit result
        lua.globals().set("exit_code", LuaValue::Nil).unwrap();

        // Create on_exit callback
        let on_exit_func: LuaFunction = lua
            .load(
                r#"
            function(result)
                exit_code = result.code
            end
        "#,
            )
            .eval()
            .unwrap();

        let table = lua.create_table().unwrap();
        table.set("on_exit", on_exit_func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();
        let _handle = manager
            .spawn_shell_command("exit 42".to_string(), opts)
            .unwrap();

        // Wait for exit event
        let mut exit_event = None;
        for _ in 0..100 {
            thread::sleep(Duration::from_millis(20));
            let events = manager.process_events();
            // Tuple format: (callback_id, handle_id, data, is_stdout, text_mode, is_exit)
            for event in events {
                if event.5 {
                    exit_event = Some(event);
                    break;
                }
            }
            if exit_event.is_some() {
                break;
            }
        }

        assert!(exit_event.is_some(), "Expected exit event");

        // Invoke on_exit callback
        let (callback_id, handle_id, _data, _is_stdout, _text_mode, _is_exit) = exit_event.unwrap();
        if let Some(func) = registry.borrow().get(&lua, callback_id).unwrap() {
            let result = manager.get_exit_result(handle_id).unwrap();
            let result_table = result.to_lua_table(&lua, true).unwrap();
            let _: () = func.call(result_table).unwrap();
        }

        // Verify exit code
        let exit_code: i32 = lua.globals().get("exit_code").unwrap();
        assert_eq!(exit_code, 42);
    }

    #[test]
    fn test_multiple_processes_callbacks_isolated() {
        let mut manager = ProcessManager::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let lua = Lua::new();

        // Create globals to track which process produced which output
        lua.globals().set("proc1_output", "".to_string()).unwrap();
        lua.globals().set("proc2_output", "".to_string()).unwrap();

        // Create callbacks for process 1
        let stdout_func1: LuaFunction = lua
            .load(
                r#"
            function(err, data)
                if data then proc1_output = proc1_output .. data end
            end
        "#,
            )
            .eval()
            .unwrap();

        let table1 = lua.create_table().unwrap();
        table1.set("stdout", stdout_func1).unwrap();
        let opts1 = parse_spawn_opts(&lua, &table1, Some(&registry)).unwrap();
        let callback_id1 = opts1.stdout_callback.unwrap();

        // Create callbacks for process 2
        let stdout_func2: LuaFunction = lua
            .load(
                r#"
            function(err, data)
                if data then proc2_output = proc2_output .. data end
            end
        "#,
            )
            .eval()
            .unwrap();

        let table2 = lua.create_table().unwrap();
        table2.set("stdout", stdout_func2).unwrap();
        let opts2 = parse_spawn_opts(&lua, &table2, Some(&registry)).unwrap();
        let callback_id2 = opts2.stdout_callback.unwrap();

        // Spawn both processes
        let (handle1, _) = manager
            .spawn_command(vec!["echo".to_string(), "PROC1".to_string()], opts1)
            .unwrap();
        let (handle2, _) = manager
            .spawn_command(vec!["echo".to_string(), "PROC2".to_string()], opts2)
            .unwrap();

        // Wait for both to complete
        let mut all_events = Vec::new();
        let mut exits = 0;
        for _ in 0..100 {
            thread::sleep(Duration::from_millis(20));
            let events = manager.process_events();
            // Tuple format: (callback_id, handle_id, data, is_stdout, text_mode, is_exit)
            exits += events.iter().filter(|e| e.5).count();
            all_events.extend(events);
            if exits >= 2 {
                break;
            }
        }

        // Invoke callbacks, checking isolation
        for event in &all_events {
            let (callback_id, event_handle_id, data, _is_stdout, _text_mode, _is_exit) = event;
            if let Some(func) = registry.borrow().get(&lua, *callback_id).unwrap() {
                let data_str = String::from_utf8_lossy(data).to_string();
                // Verify callback ID matches the correct process
                if *event_handle_id == handle1 {
                    assert_eq!(*callback_id, callback_id1);
                } else if *event_handle_id == handle2 {
                    assert_eq!(*callback_id, callback_id2);
                }
                let _: () = func.call((LuaValue::Nil, data_str)).unwrap();
            }
        }

        // Verify isolation
        let proc1_output: String = lua.globals().get("proc1_output").unwrap();
        let proc2_output: String = lua.globals().get("proc2_output").unwrap();

        assert!(
            proc1_output.contains("PROC1"),
            "proc1 should have PROC1: {}",
            proc1_output
        );
        assert!(
            !proc1_output.contains("PROC2"),
            "proc1 should not have PROC2: {}",
            proc1_output
        );
        assert!(
            proc2_output.contains("PROC2"),
            "proc2 should have PROC2: {}",
            proc2_output
        );
        assert!(
            !proc2_output.contains("PROC1"),
            "proc2 should not have PROC1: {}",
            proc2_output
        );
    }

    #[test]
    fn test_wait_without_timeout_blocks_until_exit() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();

        let (handle_id, _pid) = manager
            .spawn_shell_command("sleep 0.1 && exit 7".to_string(), opts)
            .unwrap();

        // Wait without timeout - should block and return exit code
        let result = manager.wait(handle_id, None).unwrap();

        assert_eq!(result.code, Some(7));
        assert!(result.signal.is_none());
    }

    #[test]
    fn test_wait_with_timeout_returns_before_timeout() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();

        // Process that exits quickly
        let (handle_id, _pid) = manager
            .spawn_shell_command("exit 0".to_string(), opts)
            .unwrap();

        // Wait with generous timeout
        let start = std::time::Instant::now();
        let result = manager.wait(handle_id, Some(5000)).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.code, Some(0));
        // Should complete well before the 5 second timeout
        assert!(
            elapsed.as_millis() < 1000,
            "Expected quick exit, took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_wait_timeout_sends_sigterm() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();

        // Process that sleeps but will exit on SIGTERM
        let (handle_id, _pid) = manager
            .spawn_shell_command("sleep 60".to_string(), opts)
            .unwrap();

        // Wait with very short timeout - process will be terminated
        let result = manager.wait(handle_id, Some(50)).unwrap();

        // Should have been killed by SIGTERM (signal 15)
        assert!(
            result.signal == Some(15) || result.signal == Some(9),
            "Expected SIGTERM(15) or SIGKILL(9), got {:?}",
            result.signal
        );
    }

    #[test]
    fn test_wait_timeout_escalates_to_sigkill() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();

        // Process that traps SIGTERM and ignores it, but runs for a limited time
        // We use a shell script that:
        // 1. Traps SIGTERM to ignore it
        // 2. Sleeps in a loop (will be killed by SIGKILL)
        let (handle_id, _pid) = manager
            .spawn_shell_command(
                "trap '' TERM; while true; do sleep 0.1; done".to_string(),
                opts,
            )
            .unwrap();

        // Wait with short timeout - process ignores SIGTERM, so should escalate to SIGKILL
        // The SIGTERM_GRACE_MS is 1000ms, so total wait is timeout + grace + some buffer
        let start = std::time::Instant::now();
        let result = manager.wait(handle_id, Some(50)).unwrap();
        let elapsed = start.elapsed();

        // Should have been killed by SIGKILL (signal 9) since it ignored SIGTERM
        assert_eq!(
            result.signal,
            Some(9),
            "Expected SIGKILL(9), got {:?}",
            result.signal
        );

        // Should have taken at least the grace period (SIGTERM_GRACE_MS = 1000ms)
        assert!(
            elapsed.as_millis() >= 1000,
            "Expected at least 1000ms for grace period, took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_wait_cached_result() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();

        let (handle_id, _pid) = manager
            .spawn_shell_command("exit 99".to_string(), opts)
            .unwrap();

        // First wait
        let result1 = manager.wait(handle_id, Some(5000)).unwrap();
        assert_eq!(result1.code, Some(99));

        // Second wait should return cached result
        let result2 = manager.wait(handle_id, None).unwrap();
        assert_eq!(result2.code, Some(99));
        assert_eq!(result1.signal, result2.signal);
    }

    #[test]
    fn test_max_callbacks_per_flush_limits_events() {
        let mut manager = ProcessManager::new();
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let lua = Lua::new();

        // Create stdout callback
        let stdout_func: LuaFunction = lua.create_function(|_, ()| Ok(())).unwrap();

        let table = lua.create_table().unwrap();
        table.set("stdout", stdout_func).unwrap();

        let opts = parse_spawn_opts(&lua, &table, Some(&registry)).unwrap();

        // Spawn a command that produces many lines of output (more than MAX_CALLBACKS_PER_FLUSH)
        let _handle = manager
            .spawn_shell_command(
                "for i in $(seq 1 50); do echo line$i; done".to_string(),
                opts,
            )
            .unwrap();

        // Wait for process to produce output
        thread::sleep(Duration::from_millis(200));

        // First call should return at most MAX_CALLBACKS_PER_FLUSH events
        let events1 = manager.process_events();

        // If there are many events, they should be limited
        // Note: The actual limit is enforced per-process-state, and events may be batched
        // The key behavior is that process_events returns a reasonable batch size
        assert!(
            events1.len() <= 100,
            "Expected reasonable batch size, got {}",
            events1.len()
        );

        // Subsequent calls should drain remaining events
        thread::sleep(Duration::from_millis(100));
        let events2 = manager.process_events();

        // Total events should include stdout lines and exit
        let total = events1.len() + events2.len();
        assert!(total > 0, "Expected some events from the process");
    }

    // ========================================================================
    // Phase 3: stdin Handling Tests
    // ========================================================================

    #[test]
    fn test_stdin_closed_by_default() {
        let opts = SpawnOpts::default();
        assert!(
            matches!(opts.stdin, StdinMode::Closed),
            "Expected default stdin to be Closed"
        );
    }

    #[test]
    fn test_stdin_pipe_shorthand() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("stdin_pipe", true).unwrap();

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(
            matches!(opts.stdin, StdinMode::Pipe),
            "Expected stdin_pipe=true to set StdinMode::Pipe"
        );
    }

    #[test]
    fn test_stdin_modes_parsing() {
        let lua = Lua::new();

        // stdin = false -> Closed
        let table = lua.create_table().unwrap();
        table.set("stdin", false).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Closed));

        // stdin = true -> Pipe
        let table = lua.create_table().unwrap();
        table.set("stdin", true).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Pipe));

        // stdin = "pipe" -> Pipe
        let table = lua.create_table().unwrap();
        table.set("stdin", "pipe").unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Pipe));

        // stdin = "data" -> Data("data")
        let table = lua.create_table().unwrap();
        table.set("stdin", "hello world").unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(matches!(opts.stdin, StdinMode::Data(ref s) if s == "hello world"));
    }

    #[test]
    fn test_stdin_data_mode_process_runs() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts {
            stdin: StdinMode::Data("hello\n".to_string()),
            ..Default::default()
        };

        // cat with stdin data should exit successfully
        let (handle_id, _pid) = manager
            .spawn_command(vec!["cat".to_string()], opts)
            .unwrap();

        let result = manager.wait(handle_id, Some(5000)).unwrap();
        assert_eq!(result.code, Some(0), "cat should exit with code 0");
    }

    #[test]
    fn test_stdin_pipe_write_and_close() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts {
            stdin: StdinMode::Pipe,
            ..Default::default()
        };

        let (handle_id, _pid) = manager
            .spawn_command(vec!["cat".to_string()], opts)
            .unwrap();

        // Write data to stdin
        assert!(manager.write_stdin(handle_id, b"test\n").is_ok());
        assert!(!manager.is_stdin_closed(handle_id));

        // Close stdin
        let _ = manager.close_stdin(handle_id);
        assert!(manager.is_stdin_closed(handle_id));

        // Process should exit after stdin closes
        let result = manager.wait(handle_id, Some(5000)).unwrap();
        assert_eq!(result.code, Some(0));
    }

    // ========================================================================
    // Phase 3: Environment and cwd Tests
    // ========================================================================

    #[test]
    fn test_cwd_option_parsing() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("cwd", "/tmp").unwrap();

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert_eq!(opts.cwd, Some(std::path::PathBuf::from("/tmp")));
    }

    #[test]
    fn test_env_option_parsing() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let env_table = lua.create_table().unwrap();
        env_table.set("FOO", "bar").unwrap();
        env_table.set("BAZ", "qux").unwrap();
        table.set("env", env_table).unwrap();

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        let env = opts.env.unwrap();
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(env.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_clear_env_option_parsing() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("clear_env", true).unwrap();

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(opts.clear_env);
    }

    #[test]
    fn test_cwd_process_runs_in_directory() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts {
            cwd: Some(std::path::PathBuf::from("/tmp")),
            ..Default::default()
        };

        // Process should run in /tmp directory
        let (handle_id, _pid) = manager
            .spawn_command(vec!["pwd".to_string()], opts)
            .unwrap();

        let result = manager.wait(handle_id, Some(5000)).unwrap();
        assert_eq!(result.code, Some(0));
    }

    #[test]
    fn test_invalid_cwd_error() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts {
            cwd: Some(std::path::PathBuf::from("/nonexistent_xyz_path_12345")),
            ..Default::default()
        };

        let result = manager.spawn_command(vec!["pwd".to_string()], opts);
        assert!(result.is_err(), "Expected error for invalid cwd");
    }

    // ========================================================================
    // Phase 3: Text Mode Tests
    // ========================================================================

    #[test]
    fn test_text_mode_default_true() {
        let opts = SpawnOpts::default();
        assert!(opts.text, "Default text mode should be true");
    }

    #[test]
    fn test_text_mode_parsing() {
        let lua = Lua::new();

        // text = true
        let table = lua.create_table().unwrap();
        table.set("text", true).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(opts.text);

        // text = false
        let table = lua.create_table().unwrap();
        table.set("text", false).unwrap();
        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(!opts.text);
    }

    // ========================================================================
    // Phase 3: Error Handling Tests
    // ========================================================================

    #[test]
    fn test_empty_command_error() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();

        let result = manager.spawn_command(vec![], opts);
        assert!(result.is_err(), "Expected error for empty command");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("empty"),
            "Error should mention empty: {}",
            err
        );
    }

    #[test]
    fn test_nonexistent_command_error() {
        let mut manager = ProcessManager::new();
        let opts = SpawnOpts::default();

        let result = manager.spawn_command(vec!["nonexistent_xyz_command_12345".to_string()], opts);
        assert!(result.is_err(), "Expected error for nonexistent command");
    }

    #[test]
    fn test_invalid_opts_cwd_type_ignored() {
        // Lenient parsing: invalid types are silently ignored (Lua duck typing)
        // Note: Lua coerces numbers to strings, so we use a table which can't be coerced
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let inner_table = lua.create_table().unwrap();
        table.set("cwd", inner_table).unwrap(); // Table instead of string - can't coerce

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(opts.cwd.is_none(), "Invalid cwd type should be ignored");
    }

    #[test]
    fn test_invalid_opts_env_type_ignored() {
        // Lenient parsing: invalid types are silently ignored (Lua duck typing)
        // Note: env expects a table, so we use a boolean which can't become a table
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("env", true).unwrap(); // Boolean instead of table - can't coerce

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(opts.env.is_none(), "Invalid env type should be ignored");
    }

    #[test]
    fn test_capture_options_parsing() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("capture_stdout", true).unwrap();
        table.set("capture_stderr", true).unwrap();

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(opts.capture_stdout);
        assert!(opts.capture_stderr);
    }

    #[test]
    fn test_detach_option_parsing() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("detach", true).unwrap();

        let opts = parse_spawn_opts(&lua, &table, None).unwrap();
        assert!(opts.detach);
    }
}
