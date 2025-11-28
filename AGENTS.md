## Niri Wayland Compositor Architecture

Comprehensive architecture map of Niri, a scrollable-tiling Wayland compositor. Covers startup sequence [1a-1d], input processing pipeline [2a-2d], innovative Overview feature [3a-3d], layout engine [4a-4d], rendering system [5a-5d], and IPC control interface [6a-6d].

### Sub-Crate Architecture Documentation

For detailed architecture documentation of individual crates, see:

- **[niri-ipc](niri-ipc/AGENTS.md)** - IPC communication system with Unix domain sockets, JSON serialization, and event stream state synchronization
- **[niri-config](niri-config/AGENTS.md)** - KDL configuration parsing with includes, key binding resolution, and modular subsystems
- **[niri-visual-tests](niri-visual-tests/AGENTS.md)** - GTK-based visual testing framework with Smithay rendering and animation control
- **[niri-lua](niri-lua/AGENTS.md)** - Lua scripting system with tiered APIs (configuration, runtime state, IPC execution)

### 1. Compositor Startup and Initialization

Main entry point and core system initialization sequence

### 1a. State Creation (`main.rs:172`)

Creates the main compositor state with all subsystems

```text
let mut state = State::new(config, event_loop.handle(), event_loop.get_signal(), display, false, true, cli.session).unwrap();
```

### 1b. Main Event Loop (`main.rs:264`)

Starts the compositor's main event loop

```text
event_loop.run(None, &mut state, |state| state.refresh_and_flush_clients()).unwrap();
```

### 1c. Niri State Constructor (`niri.rs:668`)

Initializes the core Niri compositor state

```text
pub fn new(config: Config, event_loop: LoopHandle<'static, State>, stop_signal: LoopSignal, display: Display<State>, headless: bool, create_wayland_socket: bool, is_session_instance: bool) -> Result<Self, Box<dyn std::error::Error>>
```

### 1d. Backend Integration (`niri.rs:697`)

Creates Niri state and integrates with rendering backend

```text
let mut niri = Niri::new(config.clone(), event_loop, stop_signal, display, &backend, create_wayland_socket, is_session_instance);
```

### 2. Input Event Processing Pipeline

How input events flow from devices to compositor actions

### 2a. Input Event Entry (`mod.rs:112`)

Main entry point for processing all input events

```text
pub fn process_input_event<I: InputBackend + 'static>(&mut self, event: InputEvent<I>)
```

### 2b. Event Dispatch (`mod.rs:152`)

Routes input events to specific handlers

```text
Keyboard { event } => self.on_keyboard::<I>(event, &mut consumed_by_a11y),
```

### 2c. Key Binding Resolution (`mod.rs:509`)

Resolves key presses to configured bindings

```text
let res = { let config = this.niri.config.borrow(); let bindings = make_binds_iter(&config, &mut this.niri.window_mru_ui, modifiers);
```

### 2d. Action Execution (`mod.rs:553`)

Executes the action associated with a key binding

```text
self.handle_bind(bind.clone());
```

### 3. Overview Mode Activation and Interaction

The innovative Overview feature for workspace navigation

### 3a. Overview State (`monitor.rs:79`)

Monitor tracks whether overview is open

```text
pub(super) overview_open: bool,
```

### 3b. Overview Toggle (`mod.rs:2534`)

Main function to toggle overview mode on/off

```text
pub fn toggle_overview(&mut self) {
```

### 3c. Overview Input Handling (`mod.rs:530`)

Special keyboard handling in overview mode

```text
if this.niri.keyboard_focus.is_overview() && pressed { if let Some(bind) = raw.and_then(|raw| hardcoded_overview_bind(raw, *mods)) {
```

### 3d. Touch Workspace Selection (`touch_overview_grab.rs:83`)

Touch interaction to select workspace in overview

```text
let ws_idx = if let Some((Some(mon), ws_idx, _)) = layout.workspaces().find(|(_, _, ws)| ws_matches(ws)) {
```

### 4. Window Layout and Tiling Management

Core layout engine for scrollable tiling windows

### 4a. Window Addition (`mod.rs:877`)

Adds a new window to the layout system

```text
pub fn add_window(&mut self, window: W, target: AddWindowTarget<W>, width: Option<PresetSize>, height: Option<PresetSize>, is_full_width: bool, is_floating: bool, activate: ActivateWindow) -> Option<&Output>
```

### 4b. Monitor Window Assignment (`mod.rs:958`)

Assigns window to specific monitor and workspace

```text
mon.add_window(window, target, activate, scrolling_width, is_full_width, is_floating);
```

### 4c. Monitor Layout Integration (`monitor.rs:847`)

Monitor handles window placement within its workspace

```text
pub fn add_window(&mut self, window: W, target: MonitorAddWindowTarget, activate: ActivateWindow, width: Option<ColumnWidth>, is_full_width: bool, is_floating: bool)
```

### 4d. Workspace Window Management (`monitor.rs:923`)

Workspace manages the actual window tiling and positioning

```text
ws.add_window(window, target, activate, width, is_full_width, is_floating);
```

### 5. Rendering and Frame Presentation

How frames are rendered and presented to displays

### 5a. Render Cycle Entry (`niri.rs:720`)

Main refresh function called each frame

```text
pub fn refresh_and_flush_clients(&mut self) {
```

### 5b. Output Redraw (`niri.rs:731`)

Queues redraws for all modified outputs

```text
self.niri.redraw_queued_outputs(&mut self.backend);
```

### 5c. Backend Rendering (`mod.rs:86`)

Backend-specific rendering implementation

```text
pub fn render(&mut self, niri: &mut Niri, output: &Output, target_presentation_time: Duration) -> RenderResult
```

### 5d. Frame Completion (`mod.rs:357`)

Finalizes and presents the rendered frame

```text
frame.finish().context("error finishing frame")
```

### 6. IPC Communication and Control

Inter-process communication for external control

### 6a. IPC Request Types (`lib.rs:67`)

Defines all possible IPC requests

```text
pub enum Request { Version, Outputs, Workspaces, Windows, Layers, KeyboardLayouts, FocusedOutput, FocusedWindow, PickWindow, PickColor, Action(Action),
```

### 6b. IPC Client Handler (`client.rs:9`)

Handles IPC messages from external clients

```text
pub fn handle_msg(msg: String, json: bool) -> Result<(), Box<dyn std::error::Error>> {
```

### 6c. IPC Action Execution (`mod.rs:642`)

Executes actions received via IPC

```text
self.do_action(action, bind.allow_when_locked);
```

### 6d. Overview Control (`mod.rs:3623`)

IPC can toggle overview mode remotely

```text
Action::ToggleOverview {} => {
```
