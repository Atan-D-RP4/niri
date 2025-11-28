## Niri Config Crate Architecture

Comprehensive architecture of the configuration system. Covers configuration file loading and parsing [1a-1f], key binding resolution [2a-2e], and configuration subsystems including layout, output, input, animations, and window rules.

The niri-config crate is responsible for parsing KDL-format configuration files, managing includes, resolving keybindings, and providing strongly-typed configuration structures for the compositor. It uses the `knuffel` KDL parser with custom context-based include handling.

### 1. Configuration File Loading and Parsing

How niri loads and merges configuration from multiple files with includes

### 1a. Configuration Path Management (`lib.rs:97`)

Determines config file location with user/system fallbacks

```text
pub enum ConfigPath {
    Explicit(PathBuf),
    Regular {
        user_path: PathBuf,
        system_path: PathBuf,
    },
}
```

**Purpose**: Supports explicit config paths or default XDG paths. Regular mode tries user path first, falls back to system path, then creates user path at startup.

### 1b. Config File Loading (`lib.rs:410`)

Loads configuration from a file path with error handling

```text
pub fn load(path: &Path) -> ConfigParseResult<Self, miette::Report> {
```

**Purpose**: Reads file contents and delegates to parse. Returns ConfigParseResult wrapping the config or error diagnostics.

### 1c. Content Reading and Preparation (`lib.rs:420`)

Reads file bytes and converts to string for parsing

```text
Self::parse(path, &contents).map_config_res(|res| {
```

**Purpose**: Separates I/O from parsing. Allows the parser to work with a String representation of the config.

### 1d. KDL Parsing with Knuffel Context (`lib.rs:439`)

Parses KDL configuration with context for include handling and error reporting

```text
let part = knuffel::parse_with_context::<ConfigPart, knuffel::span::Span, _>(
    content,
    Default::default(),
    Default::default(),
    {
        let mut ctx = knuffel::decode::Context::new();
        // Context setup for includes and merging
        ctx
    }
)
```

**Purpose**: Uses knuffel's context feature to pass custom data (recursion limit, include stack, base paths) to the parser. Enables stateful parsing for include directives.

### 1e. Include Directive Handling (`lib.rs:293`)

Processes include directives to merge additional config files

```text
"include" => {
    let include_path = extract_include_path(node)?;
    let resolved_path = resolve_include_path(include_path)?;
    load_and_parse_include(resolved_path, recursion_depth)?;
}
```

**Purpose**: Reads include paths from config, resolves them relative to the current file's directory, and recursively parses included files. Enforces recursion limits to prevent infinite includes.

### 1f. Config Part Merging (`lib.rs:179`)

Merges parsed configuration parts into the main configuration structure

```text
macro_rules! m_merge {
    ($field:ident) => {{
        let part = knuffel::Decode::decode_node(node, ctx)?;
        config.borrow_mut().$field.merge_with(&part);
    }};
}

match name {
    "input" => m_merge!(input),
    "animations" => m_merge!(animations),
    // ... other fields
}
```

**Purpose**: Uses a macro-based approach to decode each config section and merge it with existing values. MergeWith trait handles field-specific merge semantics.

### 2. Key Binding Configuration and Action Resolution

How keyboard shortcuts are parsed from config and converted to executable actions

### 2a. Binds Configuration Section (`binds.rs:18`)

Main binds structure containing a vector of individual key bindings

```text
#[derive(Debug, Default, PartialEq)]
pub struct Binds(pub Vec<Bind>);
```

**Purpose**: Wrapper type for the ordered list of all key bindings configured by the user.

### 2b. Key Binding Structure (`binds.rs:22`)

Defines a key binding with trigger, action, and behavior modifiers

```text
#[derive(Debug, Clone, PartialEq)]
pub struct Bind {
    pub key: Key,
    pub action: Action,
    pub repeat: bool,
    pub cooldown: Option<Duration>,
    pub allow_when_locked: bool,
    pub allow_inhibiting: bool,
    pub hotkey_overlay_title: Option<Option<String>>,
}
```

**Purpose**: Encapsulates a single binding with:
- `key`: The keyboard/mouse trigger and modifiers
- `action`: What to execute
- `repeat`: Whether action repeats while held
- `cooldown`: Minimum time between repeated executions
- `allow_when_locked`: Execute even with screensaver/lock active
- `allow_inhibiting`: Execute despite input method inhibitors
- `hotkey_overlay_title`: Text to display in hotkey overlay

### 2c. Key Definition (`binds.rs:33`)

Combines keyboard triggers with modifier keys for binding matching

```text
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Key {
    pub trigger: Trigger,
    pub modifiers: Modifiers,
}
```

**Purpose**: Identifies a unique keystroke. Trigger specifies which key/button, Modifiers specifies Ctrl/Shift/Alt/Super/etc.

### 2d. Trigger Types (`binds.rs:39`)

Supported input triggers including keysym, mouse buttons, and scroll events

```text
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Trigger {
    Keysym(Keysym),
    MouseLeft,
    MouseRight,
    MouseMiddle,
    MouseBack,
    MouseForward,
    WheelScrollDown,
    WheelScrollUp,
    WheelScrollLeft,
    WheelScrollRight,
    TouchpadScrollDown,
    TouchpadScrollUp,
    TouchpadScrollLeft,
    TouchpadScrollRight,
}
```

**Purpose**: Covers all input types that can trigger actions - keyboard keys by keysym, mouse buttons, and scroll wheel in four directions (wheel and touchpad).

### 2e. Modifier Flags (`binds.rs:56`)

Bitflags for keyboard modifiers in key bindings

```text
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers : u8 {
        const CTRL = 1;
        const SHIFT = 1 << 1;
        const ALT = 1 << 2;
        const SUPER = 1 << 3;
        const ISO_LEVEL3_SHIFT = 1 << 4;
        const ISO_LEVEL5_SHIFT = 1 << 5;
        const COMPOSITOR = 1 << 6;
    }
}
```

**Purpose**: Uses bitflags for efficient modifier combination checking. Supports standard modifiers plus ISO level shifts and a special COMPOSITOR modifier.

### 3. Configuration Subsystems

The crate provides modular configuration for distinct compositor features:

### 3a. Layout Configuration (`layout.rs`)

Settings for window layout modes including tiling columns, fullscreen, and floating windows.

### 3b. Input Configuration (`input.rs`)

Keyboard/mouse handling including XKB layouts, scroll methods, pointer warping, and touchpad settings.

### 3c. Appearance Configuration (`appearance.rs`)

Visual settings including colors, fonts, borders, shadows, and cursor styling.

### 3d. Animation Configuration (`animations.rs`)

Timing and easing settings for window movements, transitions, and UI animations.

### 3e. Output Configuration (`output.rs`)

Per-monitor settings including resolution, refresh rate, position, scaling, and VRR.

### 3f. Window Rules (`window_rule.rs`)

Patterns for matching windows and applying automatic configurations like floating mode, size, position, workspace assignment.

### 3g. Layer Rules (`layer_rule.rs`)

Patterns for matching layer-shell surfaces and applying settings.

### 3h. Gestures Configuration (`gestures.rs`)

Touchpad gesture bindings for workspace switching and other compositor actions.

### 4. Parsing Infrastructure

### 4a. Knuffel Parser Integration

Uses the `knuffel` crate for KDL (KDL Document Language) parsing with full error diagnostics via miette.

### 4b. Merge Strategy

Configuration sections use a `MergeWith` trait to combine values from multiple files. Allows:
- Multiple output configurations (accumulated)
- Multiple window/layer rules (accumulated)
- Single sections like input/animations (merged/overridden)

### 4c. Error Handling

Rich error reporting with file locations and suggestions using miette. Includes handling for:
- Parse errors in KDL syntax
- Type mismatches in configuration values
- Circular includes
- Missing or unreadable files

### 5. Default Values

The crate provides `Default` implementations matching `default-config.kdl`, ensuring sensible fallbacks if configuration is missing or incomplete.
