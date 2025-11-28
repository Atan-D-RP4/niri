## Niri IPC Crate Architecture

Comprehensive architecture of the IPC communication system that enables external applications to interact with Niri. Covers client request flow [1a-1f], event stream state management [2a-2f], and serialization protocols.

The niri-ipc crate is the public API surface for inter-process communication with the Niri compositor, providing both blocking socket helpers and event stream capabilities for real-time state synchronization.

### 1. IPC Client Request Flow

How external applications communicate with niri through Unix domain sockets

### 1a. Socket Path Resolution (`socket.rs:27`)

Client discovers and connects to niri's IPC socket using the NIRI_SOCKET environment variable

```text
pub fn connect() -> io::Result<Self> {
    let socket_path = env::var_os(SOCKET_PATH_ENV).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("{SOCKET_PATH_ENV} is not set, are you running this within niri?"),
        )
    })?;
    Self::connect_to(socket_path)
}
```

**Purpose**: Safely discovers the socket path from the environment. Returns a clear error if niri is not running or the socket is not available.

### 1b. Unix Domain Socket Connection (`socket.rs:38`)

Establishes a buffered connection to the IPC socket at the discovered path

```text
pub fn connect_to(path: impl AsRef<Path>) -> io::Result<Self> {
    let stream = UnixStream::connect(path.as_ref())?;
    let stream = BufReader::new(stream);
    Ok(Self { stream })
}
```

**Purpose**: Creates a persistent connection using Unix domain sockets for efficient local IPC. BufReader provides efficient line-based reading for the JSON protocol.

### 1c. Request Serialization and Transmission (`socket.rs:51`)

Request is converted to JSON format and sent over the socket as a single line

```text
pub fn send(&mut self, request: Request) -> io::Result<Reply> {
    let mut buf = serde_json::to_string(&request).unwrap();
    buf.push('\n');
    self.stream.get_mut().write_all(buf.as_bytes())?;
```

**Purpose**: Serializes the request enum to JSON and sends it with a newline delimiter. The line-based format allows streaming protocols.

### 1d. Response Deserialization (`socket.rs:59`)

Response is read from socket line-by-line and deserialized from JSON format

```text
buf.clear();
self.stream.read_line(&mut buf)?;

let reply = serde_json::from_str(&buf)?;
Ok(reply)
```

**Purpose**: Reads the response as a JSON string and deserializes it to a Reply type. Line-based reading works with the newline-delimited protocol.

### 1e. Request Types (`lib.rs:67`)

All possible IPC requests including queries, actions, and event streams

```text
pub enum Request {
    Version,
    Outputs,
    Workspaces,
    Windows,
    Layers,
    KeyboardLayouts,
    FocusedOutput,
    FocusedWindow,
    PickWindow,
    PickColor,
    Action(Action),
    Output { output: String, action: OutputAction },
    EventStream,
    ReturnError,
    OverviewState,
    ExecuteLua { code: String },
}
```

**Purpose**: Defines all possible requests clients can make. Covers state queries (Version, Outputs, Windows), actions (Action, ExecuteLua), and streaming (EventStream).

### 1f. Standardized Response Format (`lib.rs:130`)

Replies wrap successful responses or error messages in a unified Result type

```text
pub type Reply = Result<Response, String>;
```

**Purpose**: All requests receive exactly one Reply. Ok(Response) for success, Err(String) for errors. Enables uniform error handling across all request types.

### 2. Event Stream State Management

How clients maintain synchronized state with niri through continuous event streams

### 2a. Event Stream Activation (`socket.rs:89`)

After sending EventStream request, creates a closure that continuously reads events

```text
pub fn read_events(self) -> impl FnMut() -> io::Result<Event> {
    let Self { mut stream } = self;
    let _ = stream.get_mut().shutdown(Shutdown::Write);

    let mut buf = String::new();
    move || {
        buf.clear();
        stream.read_line(&mut buf)?;
        let event = serde_json::from_str(&buf)?;
        Ok(event)
    }
}
```

**Purpose**: Transforms the socket into a read-only event stream. Shuts down the write end and returns a closure that blocks reading events line-by-line.

### 2b. State Part Trait Interface (`state.rs:15`)

Components of state can independently replicate and apply events

```text
pub trait EventStreamStatePart {
    fn replicate(&self) -> Vec<Event>;
    fn apply(&mut self, event: Event) -> Option<Event>;
}
```

**Purpose**: Allows composable state tracking. Each state component (workspaces, windows, keyboard layouts) can independently handle its events.

### 2c. Comprehensive Event Stream State (`state.rs:34`)

Central container for all state communicated via the event stream

```text
pub struct EventStreamState {
    pub workspaces: WorkspacesState,
    pub windows: WindowsState,
    pub keyboard_layouts: KeyboardLayoutsState,
    pub overview: OverviewState,
    pub config: ConfigState,
}
```

**Purpose**: Provides a complete view of the compositor state. Clients can track workspaces, windows, keyboard layouts, overview state, and config status independently.

### 2d. Event Application and Dispatching (`state.rs:97`)

Central event dispatcher that routes events to appropriate state handlers

```text
fn apply(&mut self, event: Event) -> Option<Event> {
    let event = self.workspaces.apply(event)?;
    let event = self.windows.apply(event)?;
    let event = self.keyboard_layouts.apply(event)?;
    let event = self.overview.apply(event)?;
    let event = self.config.apply(event)?;
    Some(event)
}
```

**Purpose**: Routes events through each state component. If a component handles the event, it returns None. Unhandled events propagate to the next component.

### 2e. Window State Management (`state.rs:163`)

Handles window list changes by updating the internal window map

```text
Event::WindowsChanged { windows } => {
    self.windows = windows.into_iter().map(|w| (w.id, w)).collect();
}
```

**Purpose**: Maintains a current mapping of window IDs to Window objects. Clients can query window state without separate requests.

### 2f. State Replication (`state.rs:87`)

Generates events needed to recreate the current state from scratch

```text
fn replicate(&self) -> Vec<Event> {
    let mut events = Vec::new();
    events.extend(self.workspaces.replicate());
    events.extend(self.windows.replicate());
    events.extend(self.keyboard_layouts.replicate());
    events.extend(self.overview.replicate());
    events.extend(self.config.replicate());
    events
}
```

**Purpose**: When a client joins the event stream, niri sends full state replication events first. This ensures clients start with complete, consistent state.

### 3. Protocol Details

The IPC protocol uses JSON serialization over Unix domain sockets with line-based message framing.

**Key Properties**:
- **Thread-safety**: The Socket struct is not thread-safe; use separate sockets for concurrent clients
- **Ordering**: Requests are processed sequentially; state changes may occur between requests
- **Consistency**: Event stream state updates are atomic where reasonable but not guaranteed across all events
- **Features**: The crate supports optional `json-schema` and `clap` derivation features for schema generation and CLI parsing

### 4. Error Handling

All responses follow a unified error pattern:
- Socket errors (connection failures, I/O errors) are propagated as `io::Result`
- Protocol errors (invalid JSON) are propagated as `io::Result`
- Niri application errors are returned in `Reply::Err(String)`

This allows clients to distinguish between communication failures and compositor-level errors.
