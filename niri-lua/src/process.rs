//! Process spawning and control for the Lua API.
//!
//! This module provides the `ProcessHandle` userdata that enables Lua scripts
//! to spawn processes with output capture and control.
//!
//! # Overview
//!
//! When `niri.action:spawn(cmd, opts)` is called with options, it returns a
//! `ProcessHandle` that allows:
//! - Waiting for the process to complete (`proc:wait()`)
//! - Killing the process (`proc:kill()`)
//! - Writing to stdin (`proc:write()`)
//! - Checking if the process is closing (`proc:is_closing()`)
//!
//! # Example
//!
//! ```lua
//! -- Spawn with output capture
//! local proc = niri.action:spawn({"echo", "hello"}, {})
//! local result = proc:wait()
//! print(result.stdout)  -- "hello\n"
//!
//! -- With timeout
//! local proc = niri.action:spawn({"sleep", "10"}, {})
//! local result = proc:wait(1000)  -- 1 second timeout
//! if result.signal ~= 0 then
//!     print("Process was killed due to timeout")
//! end
//!
//! -- Interactive process
//! local proc = niri.action:spawn({"cat"}, { stdin = true })
//! proc:write("hello\n")
//! proc:kill()
//! local result = proc:wait()
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use log::{debug, error, warn};
use mlua::prelude::*;
use nix::sys::signal::{kill as nix_kill, Signal};
use nix::unistd::Pid;

/// Counter for generating unique process tracking IDs.
static NEXT_PROCESS_ID: AtomicU64 = AtomicU64::new(1);

/// Constants for process timeout escalation.
const SIGTERM_GRACE_PERIOD_MS: u64 = 100;
const SIGKILL_FINAL_WAIT_MS: u64 = 50;

/// Options for spawning a process.
#[derive(Debug, Clone, Default)]
pub struct SpawnOpts {
    /// Working directory for the process.
    pub cwd: Option<String>,
    /// Environment variables to set (merged with current environment).
    pub env: Option<HashMap<String, String>>,
    /// If true, start with an empty environment.
    pub clear_env: bool,
    /// Stdin input: None = no stdin, Some(String) = provide input, Some("") with stdin_pipe =
    /// enable :write()
    pub stdin: Option<String>,
    /// If true, enable stdin pipe for :write() method.
    pub stdin_pipe: bool,
    /// If true, capture stdout (default: true when opts provided).
    pub capture_stdout: bool,
    /// If true, capture stderr (default: true when opts provided).
    pub capture_stderr: bool,
    /// If true, interpret output as text and strip final newline (default: true).
    pub text: bool,
    /// If true, don't track process - fire and forget semantics.
    pub detach: bool,
}

impl SpawnOpts {
    /// Parse spawn options from a Lua table.
    pub fn from_lua_table(table: &LuaTable) -> LuaResult<Self> {
        let mut opts = SpawnOpts {
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        // cwd: string?
        if let Ok(cwd) = table.get::<Option<String>>("cwd") {
            opts.cwd = cwd;
        }

        // env: table<string, string>?
        if let Ok(LuaValue::Table(env_table)) = table.get::<LuaValue>("env") {
            let mut env = HashMap::new();
            for (k, v) in env_table.pairs::<String, String>().flatten() {
                env.insert(k, v);
            }
            if !env.is_empty() {
                opts.env = Some(env);
            }
        }

        // clear_env: boolean?
        if let Ok(clear_env) = table.get::<Option<bool>>("clear_env") {
            opts.clear_env = clear_env.unwrap_or(false);
        }

        // stdin: string | boolean | nil
        match table.get::<LuaValue>("stdin")? {
            LuaValue::String(s) => {
                opts.stdin = Some(s.to_str()?.to_string());
            }
            LuaValue::Boolean(true) => {
                opts.stdin_pipe = true;
            }
            LuaValue::Boolean(false) | LuaValue::Nil => {}
            _ => {
                return Err(LuaError::external(
                    "stdin must be a string, boolean, or nil",
                ));
            }
        }

        // stdout: boolean?
        if let Ok(stdout) = table.get::<Option<bool>>("stdout") {
            opts.capture_stdout = stdout.unwrap_or(true);
        }

        // stderr: boolean?
        if let Ok(stderr) = table.get::<Option<bool>>("stderr") {
            opts.capture_stderr = stderr.unwrap_or(true);
        }

        // text: boolean?
        if let Ok(text) = table.get::<Option<bool>>("text") {
            opts.text = text.unwrap_or(true);
        }

        // detach: boolean?
        if let Ok(detach) = table.get::<Option<bool>>("detach") {
            opts.detach = detach.unwrap_or(false);
        }

        Ok(opts)
    }
}

/// Result returned by `ProcessHandle:wait()`.
#[derive(Debug, Clone, Default)]
pub struct SpawnResult {
    /// Exit code (0 = success, -1 if killed by signal).
    pub code: i32,
    /// Signal that terminated the process (0 if not signaled).
    pub signal: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

impl SpawnResult {
    /// Convert to a Lua table.
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<LuaTable> {
        let table = lua.create_table()?;
        table.set("code", self.code)?;
        table.set("signal", self.signal)?;
        table.set("stdout", self.stdout.clone())?;
        table.set("stderr", self.stderr.clone())?;
        Ok(table)
    }
}

// ============================================================================
// Process Event System (F5.2)
// ============================================================================

/// Events generated by tracked processes.
///
/// These events are sent from reader threads to the main thread via a channel,
/// ensuring that Lua callbacks are only invoked on the main thread.
#[derive(Debug)]
pub enum ProcessEvent {
    /// A line was read from stdout.
    StdoutLine { tracking_id: u64, line: String },
    /// A line was read from stderr.
    StderrLine { tracking_id: u64, line: String },
    /// The process has exited.
    Exited { tracking_id: u64, result: SpawnResult },
}

/// State for a tracked process in the ProcessManager.
struct TrackedProcess {
    /// Unique tracking ID for this process.
    tracking_id: u64,
    /// OS process ID.
    pid: u32,
    /// Registry key for on_exit callback (if any).
    on_exit_key: Option<LuaRegistryKey>,
    /// Registry key for stdout streaming callback (if any).
    stdout_cb_key: Option<LuaRegistryKey>,
    /// Registry key for stderr streaming callback (if any).
    stderr_cb_key: Option<LuaRegistryKey>,
    /// Whether we've received the exit event.
    exited: bool,
    /// The exit result (populated when exited).
    exit_result: Option<SpawnResult>,
}

/// Manages tracked processes and their callbacks.
///
/// This struct follows the same pattern as `TimerManager` from loop_api.rs:
/// - Callbacks are stored as `LuaRegistryKey`
/// - Events are polled via `fire_due_process_events()` on the main thread
/// - Reader threads push events to an internal channel
pub struct ProcessManager {
    /// All tracked processes.
    processes: HashMap<u64, TrackedProcess>,
    /// Channel receiver for process events from reader threads.
    event_rx: mpsc::Receiver<ProcessEvent>,
    /// Channel sender (cloned to reader threads).
    event_tx: mpsc::Sender<ProcessEvent>,
}

impl ProcessManager {
    /// Create a new process manager.
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        Self {
            processes: HashMap::new(),
            event_rx,
            event_tx,
        }
    }

    /// Get the event sender for reader threads.
    pub fn event_sender(&self) -> mpsc::Sender<ProcessEvent> {
        self.event_tx.clone()
    }

    /// Register a new tracked process.
    pub fn register(
        &mut self,
        tracking_id: u64,
        pid: u32,
        on_exit_key: Option<LuaRegistryKey>,
        stdout_cb_key: Option<LuaRegistryKey>,
        stderr_cb_key: Option<LuaRegistryKey>,
    ) {
        debug!(
            "ProcessManager: registering process tracking_id={} pid={}",
            tracking_id, pid
        );
        self.processes.insert(
            tracking_id,
            TrackedProcess {
                tracking_id,
                pid,
                on_exit_key,
                stdout_cb_key,
                stderr_cb_key,
                exited: false,
                exit_result: None,
            },
        );
    }

    /// Unregister a tracked process and return its callback keys for cleanup.
    pub fn unregister(&mut self, tracking_id: u64) -> Option<Vec<LuaRegistryKey>> {
        self.processes.remove(&tracking_id).map(|proc| {
            let mut keys = Vec::new();
            if let Some(key) = proc.on_exit_key {
                keys.push(key);
            }
            if let Some(key) = proc.stdout_cb_key {
                keys.push(key);
            }
            if let Some(key) = proc.stderr_cb_key {
                keys.push(key);
            }
            keys
        })
    }

    /// Check if a process is being tracked.
    pub fn is_tracked(&self, tracking_id: u64) -> bool {
        self.processes.contains_key(&tracking_id)
    }

    /// Get the number of tracked processes.
    pub fn count(&self) -> usize {
        self.processes.len()
    }

    /// Check if any processes are being tracked.
    pub fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }

    /// Drain pending events from the channel.
    ///
    /// Returns events that need to be processed. This is non-blocking.
    pub fn drain_events(&mut self) -> Vec<ProcessEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared process manager for use across Lua and Rust.
pub type SharedProcessManager = Rc<RefCell<ProcessManager>>;

/// Create a new shared process manager.
pub fn create_process_manager() -> SharedProcessManager {
    Rc::new(RefCell::new(ProcessManager::new()))
}

/// Fire all pending process events and execute their callbacks.
///
/// This should be called from the compositor's event loop.
/// Returns the number of events processed and any errors encountered.
pub fn fire_due_process_events(lua: &Lua, manager: &SharedProcessManager) -> (usize, Vec<LuaError>) {
    let mut processed = 0;
    let mut errors = Vec::new();

    // Drain all pending events
    let events = manager.borrow_mut().drain_events();

    for event in events {
        match event {
            ProcessEvent::StdoutLine { tracking_id, line } => {
                // Get stdout callback if registered
                let callback_result = {
                    let mgr = manager.borrow();
                    if let Some(proc) = mgr.processes.get(&tracking_id) {
                        proc.stdout_cb_key
                            .as_ref()
                            .map(|key| lua.registry_value::<LuaFunction>(key))
                    } else {
                        None
                    }
                };

                if let Some(Ok(callback)) = callback_result {
                    // Call with (nil, line) as per spec: fun(err: string?, data: string?)
                    match callback.call::<()>((None::<String>, line)) {
                        Ok(()) => processed += 1,
                        Err(e) => {
                            error!("Process stdout callback error: {}", e);
                            errors.push(e);
                            processed += 1;
                        }
                    }
                }
            }
            ProcessEvent::StderrLine { tracking_id, line } => {
                // Get stderr callback if registered
                let callback_result = {
                    let mgr = manager.borrow();
                    if let Some(proc) = mgr.processes.get(&tracking_id) {
                        proc.stderr_cb_key
                            .as_ref()
                            .map(|key| lua.registry_value::<LuaFunction>(key))
                    } else {
                        None
                    }
                };

                if let Some(Ok(callback)) = callback_result {
                    // Call with (nil, line) as per spec: fun(err: string?, data: string?)
                    match callback.call::<()>((None::<String>, line)) {
                        Ok(()) => processed += 1,
                        Err(e) => {
                            error!("Process stderr callback error: {}", e);
                            errors.push(e);
                            processed += 1;
                        }
                    }
                }
            }
            ProcessEvent::Exited { tracking_id, result } => {
                // Get on_exit callback if registered
                let callback_result = {
                    let mgr = manager.borrow();
                    if let Some(proc) = mgr.processes.get(&tracking_id) {
                        proc.on_exit_key
                            .as_ref()
                            .map(|key| lua.registry_value::<LuaFunction>(key))
                    } else {
                        None
                    }
                };

                if let Some(Ok(callback)) = callback_result {
                    match result.to_lua_table(lua) {
                        Ok(result_table) => match callback.call::<()>(result_table) {
                            Ok(()) => processed += 1,
                            Err(e) => {
                                error!("Process on_exit callback error: {}", e);
                                errors.push(e);
                                processed += 1;
                            }
                        },
                        Err(e) => {
                            error!("Failed to create result table: {}", e);
                            errors.push(e);
                        }
                    }
                }

                // Clean up the tracked process and its registry keys
                if let Some(keys) = manager.borrow_mut().unregister(tracking_id) {
                    for key in keys {
                        if let Err(e) = lua.remove_registry_value(key) {
                            warn!("Failed to remove process callback from registry: {}", e);
                        }
                    }
                }
            }
        }
    }

    (processed, errors)
}

/// Generate a new unique tracking ID for a process.
pub fn next_tracking_id() -> u64 {
    NEXT_PROCESS_ID.fetch_add(1, Ordering::SeqCst)
}

// ============================================================================
// Process Handle Implementation
// ============================================================================

/// Inner state for a spawned process.
struct ProcessInner {
    /// The child process.
    child: Option<Child>,
    /// Process ID.
    pid: u32,
    /// Captured stdout.
    stdout_buf: Vec<u8>,
    /// Captured stderr.
    stderr_buf: Vec<u8>,
    /// Stdin writer (if stdin pipe was enabled).
    stdin_writer: Option<std::process::ChildStdin>,
    /// Exit status once the process has exited.
    exit_status: Option<ExitStatus>,
    /// Whether the process is in the process of being closed/killed.
    is_closing: AtomicBool,
    /// Whether text mode is enabled.
    text_mode: bool,
    /// Handle for stdout reader thread.
    stdout_thread: Option<JoinHandle<Vec<u8>>>,
    /// Handle for stderr reader thread.
    stderr_thread: Option<JoinHandle<Vec<u8>>>,
}

impl ProcessInner {
    /// Check if the process has exited.
    fn has_exited(&self) -> bool {
        self.exit_status.is_some()
    }

    /// Try to wait for the process without blocking.
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        if let Some(status) = self.exit_status {
            return Ok(Some(status));
        }

        if let Some(ref mut child) = self.child {
            if let Some(status) = child.try_wait()? {
                self.exit_status = Some(status);
                return Ok(Some(status));
            }
        }

        Ok(None)
    }

    /// Wait for the process to exit, blocking.
    fn wait(&mut self) -> std::io::Result<ExitStatus> {
        if let Some(status) = self.exit_status {
            return Ok(status);
        }

        if let Some(ref mut child) = self.child {
            let status = child.wait()?;
            self.exit_status = Some(status);
            return Ok(status);
        }

        Err(std::io::Error::other("No child process"))
    }

    /// Kill the process.
    fn kill(&mut self) -> std::io::Result<()> {
        self.is_closing.store(true, Ordering::SeqCst);

        if let Some(ref mut child) = self.child {
            child.kill()?;
        }

        Ok(())
    }

    /// Write to stdin.
    fn write_stdin(&mut self, data: &[u8]) -> std::io::Result<()> {
        if let Some(ref mut stdin) = self.stdin_writer {
            stdin.write_all(data)?;
            stdin.flush()?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "stdin not available (spawn with stdin=true to enable)",
            ))
        }
    }

    /// Close stdin.
    fn close_stdin(&mut self) {
        self.stdin_writer = None;
    }

    /// Collect output from reader threads.
    fn collect_output(&mut self) {
        // Take ownership of threads
        if let Some(handle) = self.stdout_thread.take() {
            match handle.join() {
                Ok(data) => self.stdout_buf = data,
                Err(e) => error!("stdout reader thread panicked: {:?}", e),
            }
        }

        if let Some(handle) = self.stderr_thread.take() {
            match handle.join() {
                Ok(data) => self.stderr_buf = data,
                Err(e) => error!("stderr reader thread panicked: {:?}", e),
            }
        }
    }

    /// Get the result after process has exited.
    fn get_result(&mut self) -> SpawnResult {
        // Collect output from threads first
        self.collect_output();

        let (code, signal) = if let Some(status) = self.exit_status {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                let code = status.code().unwrap_or(-1);
                let signal = status.signal().unwrap_or(0);
                (code, signal)
            }
            #[cfg(not(unix))]
            {
                let code = status.code().unwrap_or(-1);
                (code, 0)
            }
        } else {
            (-1, 0)
        };

        let mut stdout = String::from_utf8_lossy(&self.stdout_buf).to_string();
        let mut stderr = String::from_utf8_lossy(&self.stderr_buf).to_string();

        // Strip trailing newline in text mode
        if self.text_mode {
            if stdout.ends_with('\n') {
                stdout.pop();
                if stdout.ends_with('\r') {
                    stdout.pop();
                }
            }
            if stderr.ends_with('\n') {
                stderr.pop();
                if stderr.ends_with('\r') {
                    stderr.pop();
                }
            }
        }

        SpawnResult {
            code,
            signal,
            stdout,
            stderr,
        }
    }
}

impl Drop for ProcessInner {
    fn drop(&mut self) {
        // Kill process if still running
        if !self.has_exited() {
            debug!(
                "ProcessInner dropped while process still running, killing pid={}",
                self.pid
            );
            if let Err(e) = self.kill() {
                warn!("Failed to kill process on drop: {}", e);
            }
        }
    }
}

/// Lua userdata for process control.
///
/// This is returned by `niri.action:spawn(cmd, opts)` when opts is provided.
pub struct ProcessHandle {
    inner: Arc<Mutex<ProcessInner>>,
    pid: u32,
}

impl std::fmt::Debug for ProcessHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessHandle")
            .field("pid", &self.pid)
            .finish_non_exhaustive()
    }
}

impl ProcessHandle {
    /// Get the process ID.
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl LuaUserData for ProcessHandle {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // pid: readonly field
        fields.add_field_method_get("pid", |_, this| Ok(this.pid));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // proc:wait(timeout_ms?) -> SpawnResult
        methods.add_method("wait", |lua, this, timeout_ms: Option<u64>| {
            let deadline = timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms));

            loop {
                // Check if process has exited
                {
                    let mut inner = this.inner.lock().map_err(|e| {
                        LuaError::external(format!("Failed to lock process state: {}", e))
                    })?;

                    // Try non-blocking wait first
                    match inner.try_wait() {
                        Ok(Some(_status)) => {
                            // Process exited, get result
                            let result = inner.get_result();
                            return result.to_lua_table(lua);
                        }
                        Ok(None) => {
                            // Still running
                        }
                        Err(e) => {
                            return Err(LuaError::external(format!(
                                "Failed to wait for process: {}",
                                e
                            )));
                        }
                    }
                }

                // Check timeout
                if let Some(deadline) = deadline {
                    if Instant::now() >= deadline {
                        // Timeout reached - escalation sequence: SIGTERM -> grace period -> SIGKILL
                        let inner = this.inner.lock().map_err(|e| {
                            LuaError::external(format!("Failed to lock process state: {}", e))
                        })?;

                        // Step 1: Send SIGTERM for graceful shutdown
                        inner.is_closing.store(true, Ordering::SeqCst);
                        let _ = nix_kill(Pid::from_raw(inner.pid as i32), Signal::SIGTERM);
                        drop(inner);

                        // Step 2: Wait grace period, polling for exit
                        let grace_deadline =
                            Instant::now() + Duration::from_millis(SIGTERM_GRACE_PERIOD_MS);
                        loop {
                            thread::sleep(Duration::from_millis(5));

                            let mut inner = this.inner.lock().map_err(|e| {
                                LuaError::external(format!("Failed to lock process state: {}", e))
                            })?;

                            if let Ok(Some(_)) = inner.try_wait() {
                                // Process exited gracefully
                                let result = inner.get_result();
                                return result.to_lua_table(lua);
                            }

                            if Instant::now() >= grace_deadline {
                                break;
                            }
                            drop(inner);
                        }

                        // Step 3: Still running after grace period - send SIGKILL
                        let inner = this.inner.lock().map_err(|e| {
                            LuaError::external(format!("Failed to lock process state: {}", e))
                        })?;
                        let _ = nix_kill(Pid::from_raw(inner.pid as i32), Signal::SIGKILL);
                        drop(inner);

                        // Step 4: Brief wait for SIGKILL to take effect
                        thread::sleep(Duration::from_millis(SIGKILL_FINAL_WAIT_MS));

                        let mut inner = this.inner.lock().map_err(|e| {
                            LuaError::external(format!("Failed to lock process state: {}", e))
                        })?;
                        let _ = inner.wait();
                        let result = inner.get_result();
                        return result.to_lua_table(lua);
                    }
                }

                // Brief sleep to avoid busy-waiting
                thread::sleep(Duration::from_millis(1));
            }
        });

        // proc:kill(signal?) -> boolean
        methods.add_method("kill", |_lua, this, signal: Option<i32>| {
            let inner = this
                .inner
                .lock()
                .map_err(|e| LuaError::external(format!("Failed to lock process state: {}", e)))?;

            let sig = match signal {
                Some(n) => Signal::try_from(n)
                    .map_err(|e| LuaError::external(format!("Invalid signal: {}", e)))?,
                None => Signal::SIGTERM, // Default to SIGTERM for graceful shutdown
            };

            inner.is_closing.store(true, Ordering::SeqCst);

            match nix_kill(Pid::from_raw(inner.pid as i32), sig) {
                Ok(()) => Ok(true),
                Err(e) => {
                    debug!("kill() failed: {}", e);
                    Ok(false)
                }
            }
        });

        // proc:write(data) -> boolean
        methods.add_method("write", |_lua, this, data: String| {
            let mut inner = this
                .inner
                .lock()
                .map_err(|e| LuaError::external(format!("Failed to lock process state: {}", e)))?;

            match inner.write_stdin(data.as_bytes()) {
                Ok(()) => Ok(true),
                Err(e) => Err(LuaError::external(format!(
                    "Failed to write to stdin: {}",
                    e
                ))),
            }
        });

        // proc:is_closing() -> boolean
        methods.add_method("is_closing", |_lua, this, ()| {
            let inner = this
                .inner
                .lock()
                .map_err(|e| LuaError::external(format!("Failed to lock process state: {}", e)))?;

            Ok(inner.is_closing.load(Ordering::SeqCst) || inner.has_exited())
        });

        // proc:close_stdin()
        methods.add_method("close_stdin", |_lua, this, ()| {
            let mut inner = this
                .inner
                .lock()
                .map_err(|e| LuaError::external(format!("Failed to lock process state: {}", e)))?;

            inner.close_stdin();
            Ok(())
        });
    }
}

/// Spawn a command with the given options, returning a ProcessHandle.
///
/// This is the main entry point for spawning processes with output capture.
pub fn spawn_command(command: Vec<String>, opts: SpawnOpts) -> Result<ProcessHandle, String> {
    if command.is_empty() {
        return Err("Command cannot be empty".to_string());
    }

    let program = &command[0];
    let args = &command[1..];

    let mut cmd = Command::new(program);
    cmd.args(args);

    // Set working directory
    if let Some(ref cwd) = opts.cwd {
        cmd.current_dir(cwd);
    }

    // Configure environment
    if opts.clear_env {
        cmd.env_clear();
    }
    if let Some(ref env) = opts.env {
        for (k, v) in env {
            cmd.env(k, v);
        }
    }

    // Configure stdin
    if opts.stdin.is_some() || opts.stdin_pipe {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    // Configure stdout/stderr capture
    if opts.capture_stdout {
        cmd.stdout(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null());
    }

    if opts.capture_stderr {
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::null());
    }

    // Spawn the process
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn process: {}", e))?;

    let pid = child.id();
    debug!("Spawned process pid={} cmd={:?}", pid, command);

    // Take stdin if we need to write to it
    let stdin_writer = if opts.stdin_pipe {
        child.stdin.take()
    } else {
        None
    };

    // Write initial stdin data if provided
    if let Some(ref stdin_data) = opts.stdin {
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(stdin_data.as_bytes()) {
                warn!("Failed to write stdin data: {}", e);
            }
            // Close stdin after writing initial data (unless stdin_pipe is also set)
            if !opts.stdin_pipe {
                drop(stdin);
            }
        }
    }

    // Start reader threads for stdout/stderr
    let stdout_thread = if opts.capture_stdout {
        child.stdout.take().map(|mut stdout| {
            thread::spawn(move || {
                let mut buf = Vec::new();
                if let Err(e) = stdout.read_to_end(&mut buf) {
                    error!("Error reading stdout: {}", e);
                }
                buf
            })
        })
    } else {
        None
    };

    let stderr_thread = if opts.capture_stderr {
        child.stderr.take().map(|mut stderr| {
            thread::spawn(move || {
                let mut buf = Vec::new();
                if let Err(e) = stderr.read_to_end(&mut buf) {
                    error!("Error reading stderr: {}", e);
                }
                buf
            })
        })
    } else {
        None
    };

    let inner = ProcessInner {
        child: Some(child),
        pid,
        stdout_buf: Vec::new(),
        stderr_buf: Vec::new(),
        stdin_writer,
        exit_status: None,
        is_closing: AtomicBool::new(false),
        text_mode: opts.text,
        stdout_thread,
        stderr_thread,
    };

    Ok(ProcessHandle {
        inner: Arc::new(Mutex::new(inner)),
        pid,
    })
}

/// Spawn a shell command with the given options, returning a ProcessHandle.
///
/// This wraps the command in a shell (sh -c "...").
pub fn spawn_shell_command(command: String, opts: SpawnOpts) -> Result<ProcessHandle, String> {
    let shell_cmd = vec!["sh".to_string(), "-c".to_string(), command];
    spawn_command(shell_cmd, opts)
}

// ============================================================================
// Async Spawn (with callbacks via ProcessManager)
// ============================================================================

/// Callback configuration for async process spawning.
pub struct ProcessCallbacks {
    /// Tracking ID for this process (used to correlate events).
    pub tracking_id: u64,
    /// Event sender to the ProcessManager.
    pub event_tx: mpsc::Sender<ProcessEvent>,
    /// Whether to send stdout line-by-line events.
    pub stream_stdout: bool,
    /// Whether to send stderr line-by-line events.
    pub stream_stderr: bool,
}

/// Spawn a command with async callbacks.
///
/// This spawns the process and sets up background threads that:
/// - Send stdout/stderr line events if streaming is enabled
/// - Send an Exited event when the process completes
///
/// The ProcessHandle is still returned for direct control (kill, write, etc.).
/// Callbacks must be registered with the ProcessManager separately.
pub fn spawn_command_async(
    command: Vec<String>,
    opts: SpawnOpts,
    callbacks: ProcessCallbacks,
) -> Result<ProcessHandle, String> {
    if command.is_empty() {
        return Err("Command cannot be empty".to_string());
    }

    let program = &command[0];
    let args = &command[1..];

    let mut cmd = Command::new(program);
    cmd.args(args);

    // Set working directory
    if let Some(ref cwd) = opts.cwd {
        cmd.current_dir(cwd);
    }

    // Configure environment
    if opts.clear_env {
        cmd.env_clear();
    }
    if let Some(ref env) = opts.env {
        for (k, v) in env {
            cmd.env(k, v);
        }
    }

    // Configure stdin
    if opts.stdin.is_some() || opts.stdin_pipe {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    // Configure stdout/stderr capture
    if opts.capture_stdout {
        cmd.stdout(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null());
    }

    if opts.capture_stderr {
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::null());
    }

    // Spawn the process
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn process: {}", e))?;

    let pid = child.id();
    let tracking_id = callbacks.tracking_id;
    let text_mode = opts.text;
    debug!(
        "Spawned async process pid={} tracking_id={} cmd={:?}",
        pid, tracking_id, command
    );

    // Take stdin if we need to write to it
    let stdin_writer = if opts.stdin_pipe {
        child.stdin.take()
    } else {
        None
    };

    // Write initial stdin data if provided
    if let Some(ref stdin_data) = opts.stdin {
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(stdin_data.as_bytes()) {
                warn!("Failed to write stdin data: {}", e);
            }
            if !opts.stdin_pipe {
                drop(stdin);
            }
        }
    }

    // Shared buffers for accumulating output (needed for final SpawnResult)
    let stdout_buf = Arc::new(Mutex::new(Vec::new()));
    let stderr_buf = Arc::new(Mutex::new(Vec::new()));

    // Start stdout reader thread
    let stdout_thread = if opts.capture_stdout {
        child.stdout.take().map(|stdout| {
            let event_tx = callbacks.event_tx.clone();
            let stream = callbacks.stream_stdout;
            let buf_clone = Arc::clone(&stdout_buf);

            thread::spawn(move || {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                let mut all_data = Vec::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            all_data.extend_from_slice(line.as_bytes());

                            if stream {
                                // Send line event (trim trailing newline for the event)
                                let line_trimmed = line.trim_end_matches('\n').to_string();
                                let _ = event_tx.send(ProcessEvent::StdoutLine {
                                    tracking_id,
                                    line: line_trimmed,
                                });
                            }
                        }
                        Err(e) => {
                            error!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }

                // Store accumulated data
                if let Ok(mut buf) = buf_clone.lock() {
                    *buf = all_data.clone();
                }
                all_data
            })
        })
    } else {
        None
    };

    // Start stderr reader thread
    let stderr_thread = if opts.capture_stderr {
        child.stderr.take().map(|stderr| {
            let event_tx = callbacks.event_tx.clone();
            let stream = callbacks.stream_stderr;
            let buf_clone = Arc::clone(&stderr_buf);

            thread::spawn(move || {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                let mut all_data = Vec::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            all_data.extend_from_slice(line.as_bytes());

                            if stream {
                                let line_trimmed = line.trim_end_matches('\n').to_string();
                                let _ = event_tx.send(ProcessEvent::StderrLine {
                                    tracking_id,
                                    line: line_trimmed,
                                });
                            }
                        }
                        Err(e) => {
                            error!("Error reading stderr: {}", e);
                            break;
                        }
                    }
                }

                if let Ok(mut buf) = buf_clone.lock() {
                    *buf = all_data.clone();
                }
                all_data
            })
        })
    } else {
        None
    };

    // Start exit monitor thread
    let event_tx = callbacks.event_tx;
    let stdout_buf_for_exit = Arc::clone(&stdout_buf);
    let stderr_buf_for_exit = Arc::clone(&stderr_buf);

    thread::spawn(move || {
        // Wait for stdout/stderr threads to complete first
        // We use a simple polling approach with the child process
        let mut child_for_wait = child;
        let status = match child_for_wait.wait() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to wait for process: {}", e);
                return;
            }
        };

        // Give reader threads a moment to finish
        thread::sleep(Duration::from_millis(10));

        // Build the result
        let (code, signal) = {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                let code = status.code().unwrap_or(-1);
                let signal = status.signal().unwrap_or(0);
                (code, signal)
            }
            #[cfg(not(unix))]
            {
                let code = status.code().unwrap_or(-1);
                (code, 0)
            }
        };

        let stdout_data = stdout_buf_for_exit
            .lock()
            .map(|b| b.clone())
            .unwrap_or_default();
        let stderr_data = stderr_buf_for_exit
            .lock()
            .map(|b| b.clone())
            .unwrap_or_default();

        let mut stdout = String::from_utf8_lossy(&stdout_data).to_string();
        let mut stderr = String::from_utf8_lossy(&stderr_data).to_string();

        // Strip trailing newline in text mode
        if text_mode {
            if stdout.ends_with('\n') {
                stdout.pop();
                if stdout.ends_with('\r') {
                    stdout.pop();
                }
            }
            if stderr.ends_with('\n') {
                stderr.pop();
                if stderr.ends_with('\r') {
                    stderr.pop();
                }
            }
        }

        let result = SpawnResult {
            code,
            signal,
            stdout,
            stderr,
        };

        // Send the exit event
        let _ = event_tx.send(ProcessEvent::Exited { tracking_id, result });
    });

    // Create the ProcessHandle (without the reader threads since they're managed differently)
    let inner = ProcessInner {
        child: None, // Child is now owned by the exit monitor thread
        pid,
        stdout_buf: Vec::new(),
        stderr_buf: Vec::new(),
        stdin_writer,
        exit_status: None,
        is_closing: AtomicBool::new(false),
        text_mode: opts.text,
        stdout_thread,
        stderr_thread,
    };

    Ok(ProcessHandle {
        inner: Arc::new(Mutex::new(inner)),
        pid,
    })
}

/// Spawn a shell command with async callbacks.
pub fn spawn_shell_command_async(
    command: String,
    opts: SpawnOpts,
    callbacks: ProcessCallbacks,
) -> Result<ProcessHandle, String> {
    let shell_cmd = vec!["sh".to_string(), "-c".to_string(), command];
    spawn_command_async(shell_cmd, opts, callbacks)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // SpawnOpts Tests
    // ========================================================================

    #[test]
    fn spawn_opts_default() {
        let opts = SpawnOpts::default();
        assert!(opts.cwd.is_none());
        assert!(opts.env.is_none());
        assert!(!opts.clear_env);
        assert!(opts.stdin.is_none());
        assert!(!opts.stdin_pipe);
        assert!(!opts.capture_stdout); // default is false, but from_lua_table sets true
        assert!(!opts.capture_stderr);
        assert!(!opts.text);
        assert!(!opts.detach);
    }

    #[test]
    fn spawn_opts_from_empty_table() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let opts = SpawnOpts::from_lua_table(&table).unwrap();

        // Defaults when opts table is provided
        assert!(opts.capture_stdout);
        assert!(opts.capture_stderr);
        assert!(opts.text);
        assert!(!opts.detach);
    }

    #[test]
    fn spawn_opts_with_cwd() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("cwd", "/tmp").unwrap();
        let opts = SpawnOpts::from_lua_table(&table).unwrap();

        assert_eq!(opts.cwd, Some("/tmp".to_string()));
    }

    #[test]
    fn spawn_opts_with_env() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let env = lua.create_table().unwrap();
        env.set("FOO", "bar").unwrap();
        env.set("BAZ", "qux").unwrap();
        table.set("env", env).unwrap();

        let opts = SpawnOpts::from_lua_table(&table).unwrap();

        let env = opts.env.unwrap();
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(env.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn spawn_opts_with_stdin_string() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("stdin", "hello world").unwrap();

        let opts = SpawnOpts::from_lua_table(&table).unwrap();

        assert_eq!(opts.stdin, Some("hello world".to_string()));
        assert!(!opts.stdin_pipe);
    }

    #[test]
    fn spawn_opts_with_stdin_true() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("stdin", true).unwrap();

        let opts = SpawnOpts::from_lua_table(&table).unwrap();

        assert!(opts.stdin.is_none());
        assert!(opts.stdin_pipe);
    }

    #[test]
    fn spawn_opts_with_detach() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("detach", true).unwrap();

        let opts = SpawnOpts::from_lua_table(&table).unwrap();

        assert!(opts.detach);
    }

    // ========================================================================
    // spawn_command Tests
    // ========================================================================

    #[test]
    fn spawn_command_empty_fails() {
        let opts = SpawnOpts::default();
        let result = spawn_command(vec![], opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn spawn_command_echo() {
        let opts = SpawnOpts {
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["echo".to_string(), "hello".to_string()], opts).unwrap();
        assert!(handle.pid() > 0);

        // Wait for completion
        let mut inner = handle.inner.lock().unwrap();
        let status = inner.wait().unwrap();
        assert!(status.success());

        let result = inner.get_result();
        assert_eq!(result.code, 0);
        assert_eq!(result.signal, 0);
        assert_eq!(result.stdout, "hello");
    }

    #[test]
    fn spawn_command_false_exits_nonzero() {
        let opts = SpawnOpts {
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["false".to_string()], opts).unwrap();

        let mut inner = handle.inner.lock().unwrap();
        let _ = inner.wait();
        let result = inner.get_result();

        assert_ne!(result.code, 0);
    }

    #[test]
    fn spawn_command_with_stdin() {
        let opts = SpawnOpts {
            stdin: Some("hello from stdin".to_string()),
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["cat".to_string()], opts).unwrap();

        let mut inner = handle.inner.lock().unwrap();
        let _ = inner.wait();
        let result = inner.get_result();

        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "hello from stdin");
    }

    #[test]
    fn spawn_command_with_cwd() {
        let opts = SpawnOpts {
            cwd: Some("/tmp".to_string()),
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["pwd".to_string()], opts).unwrap();

        let mut inner = handle.inner.lock().unwrap();
        let _ = inner.wait();
        let result = inner.get_result();

        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "/tmp");
    }

    #[test]
    fn spawn_command_with_env() {
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "my_value".to_string());

        let opts = SpawnOpts {
            env: Some(env),
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo $MY_VAR".to_string(),
            ],
            opts,
        )
        .unwrap();

        let mut inner = handle.inner.lock().unwrap();
        let _ = inner.wait();
        let result = inner.get_result();

        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "my_value");
    }

    #[test]
    fn spawn_shell_command_works() {
        let opts = SpawnOpts {
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_shell_command("echo hello | tr 'h' 'H'".to_string(), opts).unwrap();

        let mut inner = handle.inner.lock().unwrap();
        let _ = inner.wait();
        let result = inner.get_result();

        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "Hello");
    }

    #[test]
    fn spawn_command_kill() {
        let opts = SpawnOpts {
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["sleep".to_string(), "60".to_string()], opts).unwrap();

        {
            let mut inner = handle.inner.lock().unwrap();
            // Kill the process
            inner.kill().unwrap();
            // Wait for it to actually exit
            let _ = inner.wait();
        }

        let inner = handle.inner.lock().unwrap();
        assert!(inner.has_exited());
    }

    #[test]
    fn spawn_command_write_stdin() {
        let opts = SpawnOpts {
            stdin_pipe: true,
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["cat".to_string()], opts).unwrap();

        {
            let mut inner = handle.inner.lock().unwrap();
            inner.write_stdin(b"line1\n").unwrap();
            inner.write_stdin(b"line2\n").unwrap();
            inner.close_stdin();
        }

        let mut inner = handle.inner.lock().unwrap();
        let _ = inner.wait();
        let result = inner.get_result();

        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "line1\nline2");
    }

    // ========================================================================
    // ProcessHandle Lua Tests
    // ========================================================================

    #[test]
    fn process_handle_lua_wait() {
        let lua = Lua::new();

        let opts = SpawnOpts {
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["echo".to_string(), "test".to_string()], opts).unwrap();
        let ud = lua.create_userdata(handle).unwrap();

        lua.globals().set("proc", ud).unwrap();

        let result: LuaTable = lua.load("return proc:wait()").eval().unwrap();

        let code: i32 = result.get("code").unwrap();
        let stdout: String = result.get("stdout").unwrap();

        assert_eq!(code, 0);
        assert_eq!(stdout, "test");
    }

    #[test]
    fn process_handle_lua_wait_with_timeout() {
        let lua = Lua::new();

        let opts = SpawnOpts {
            capture_stdout: true,
            capture_stderr: true,
            text: true,
            ..Default::default()
        };

        // Spawn a long-running process
        let handle = spawn_command(vec!["sleep".to_string(), "60".to_string()], opts).unwrap();
        let ud = lua.create_userdata(handle).unwrap();

        lua.globals().set("proc", ud).unwrap();

        // Wait with short timeout
        let result: LuaTable = lua.load("return proc:wait(100)").eval().unwrap();

        let signal: i32 = result.get("signal").unwrap();
        // Process should have been killed (signal != 0)
        assert_ne!(signal, 0);
    }

    #[test]
    fn process_handle_lua_pid() {
        let lua = Lua::new();

        let opts = SpawnOpts {
            capture_stdout: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["echo".to_string(), "hi".to_string()], opts).unwrap();
        let expected_pid = handle.pid();
        let ud = lua.create_userdata(handle).unwrap();

        lua.globals().set("proc", ud).unwrap();

        let pid: u32 = lua.load("return proc.pid").eval().unwrap();
        assert_eq!(pid, expected_pid);
    }

    #[test]
    fn process_handle_lua_kill() {
        let lua = Lua::new();

        let opts = SpawnOpts {
            capture_stdout: true,
            text: true,
            ..Default::default()
        };

        let handle = spawn_command(vec!["sleep".to_string(), "60".to_string()], opts).unwrap();
        let ud = lua.create_userdata(handle).unwrap();

        lua.globals().set("proc", ud).unwrap();

        let killed: bool = lua.load("return proc:kill()").eval().unwrap();
        assert!(killed);

        // Wait should complete quickly
        let result: LuaTable = lua.load("return proc:wait(1000)").eval().unwrap();
        let signal: i32 = result.get("signal").unwrap();
        assert_ne!(signal, 0);
    }

    #[test]
    fn process_handle_lua_kill_with_signal() {
        let lua = Lua::new();

        let opts = SpawnOpts {
            capture_stdout: true,
            text: true,
            ..Default::default()
        };

        // Test SIGTERM (15)
        let handle =
            spawn_command(vec!["sleep".to_string(), "60".to_string()], opts.clone()).unwrap();
        let ud = lua.create_userdata(handle).unwrap();
        lua.globals().set("proc", ud).unwrap();

        let killed: bool = lua.load("return proc:kill(15)").eval().unwrap();
        assert!(killed);

        let result: LuaTable = lua.load("return proc:wait(1000)").eval().unwrap();
        let signal: i32 = result.get("signal").unwrap();
        assert_eq!(signal, 15); // Should be killed by SIGTERM

        // Test SIGKILL (9)
        let handle2 = spawn_command(vec!["sleep".to_string(), "60".to_string()], opts).unwrap();
        let ud2 = lua.create_userdata(handle2).unwrap();
        lua.globals().set("proc2", ud2).unwrap();

        let killed2: bool = lua.load("return proc2:kill(9)").eval().unwrap();
        assert!(killed2);

        let result2: LuaTable = lua.load("return proc2:wait(1000)").eval().unwrap();
        let signal2: i32 = result2.get("signal").unwrap();
        assert_eq!(signal2, 9); // Should be killed by SIGKILL
    }

    #[test]
    fn process_handle_lua_is_closing() {
        let lua = Lua::new();

        let opts = SpawnOpts {
            capture_stdout: true,
            text: true,
            ..Default::default()
        };

        // Quick command that exits immediately
        let handle = spawn_command(vec!["true".to_string()], opts).unwrap();
        let ud = lua.create_userdata(handle).unwrap();

        lua.globals().set("proc", ud).unwrap();

        // Wait for it to complete
        lua.load("proc:wait()").exec().unwrap();

        // Should be closing/closed
        let is_closing: bool = lua.load("return proc:is_closing()").eval().unwrap();
        assert!(is_closing);
    }

    #[test]
    fn test_wait_timeout_escalation() {
        let lua = Lua::new();

        // Spawn a process that ignores SIGTERM (sleep ignores it)
        let result =
            spawn_shell_command("trap '' TERM; sleep 10".to_string(), SpawnOpts::default());
        let handle = result.expect("Failed to spawn");

        // Wait with short timeout - should escalate to SIGKILL
        let start = std::time::Instant::now();
        lua.scope(|scope| -> mlua::Result<()> {
            let handle_ud = scope.create_userdata_ref(&handle)?;
            lua.globals().set("proc", handle_ud)?;

            let result: LuaTable = lua.load("return proc:wait(200)").eval()?;
            let signal: Option<i32> = result.get("signal")?;

            // Should have been killed with SIGKILL (9)
            assert_eq!(signal, Some(9));
            Ok(())
        })
        .unwrap();

        let elapsed = start.elapsed();
        // Should take roughly: 200ms timeout + 100ms grace + 50ms final = 350ms
        // Allow some tolerance
        assert!(elapsed.as_millis() < 600, "Took too long: {:?}", elapsed);
    }

    // ========================================================================
    // Snapshot Tests
    // ========================================================================

    #[test]
    fn snapshot_spawn_result() {
        let result = SpawnResult {
            code: 0,
            signal: 0,
            stdout: "hello world".to_string(),
            stderr: "".to_string(),
        };
        insta::assert_debug_snapshot!("process_spawn_result", result);
    }

    #[test]
    fn snapshot_spawn_result_with_error() {
        let result = SpawnResult {
            code: 1,
            signal: 0,
            stdout: "".to_string(),
            stderr: "error: file not found".to_string(),
        };
        insta::assert_debug_snapshot!("process_spawn_result_with_error", result);
    }

    #[test]
    fn snapshot_spawn_result_killed() {
        let result = SpawnResult {
            code: -1,
            signal: 9,
            stdout: "partial output".to_string(),
            stderr: "".to_string(),
        };
        insta::assert_debug_snapshot!("process_spawn_result_killed", result);
    }
}
