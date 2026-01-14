// Niri Lua API Data - Shared const definitions
//
// This file is designed to be included via include!() in both:
// - src/api_registry.rs (for runtime schema access)
// - build.rs (for EmmyLua generation)
//
// DO NOT add any use statements or mod declarations to this file.
// The schema types must be defined in the including file before this is included.

/// Complete niri Lua API schema.
pub const NIRI_LUA_API: LuaApiSchema = LuaApiSchema {
    modules: &[
        NIRI_MODULE,
        NIRI_UTILS_MODULE,
        NIRI_CONFIG_MODULE,
        NIRI_EVENTS_MODULE,
        NIRI_ACTION_MODULE,
        NIRI_STATE_MODULE,
        NIRI_LOOP_MODULE,
        NIRI_KEYMAP_MODULE,
        NIRI_WINDOW_MODULE,
        NIRI_OVERVIEW_MODULE,
        NIRI_SCREENSHOT_MODULE,
        NIRI_OS_MODULE,
    ],
    types: &[
        TIMER_TYPE,
        PROCESS_HANDLE_TYPE,
        ANIMATION_TYPE,
        FILTER_TYPE,
        WINDOW_RULE_TYPE,
        GESTURE_TYPE,
        CONFIG_COLLECTION_TYPE,
        CONFIG_SECTION_PROXY_TYPE,
    ],
    aliases: &[
        WINDOW_ALIAS,
        WORKSPACE_ALIAS,
        OUTPUT_ALIAS,
        SIZE_CHANGE_ALIAS,
        POSITION_CHANGE_ALIAS,
        LAYOUT_SWITCH_TARGET_ALIAS,
        WORKSPACE_REFERENCE_ALIAS,
        EVENT_HANDLER_ID_ALIAS,
        EVENT_SPEC_ALIAS,
        HANDLER_ID_MAP_ALIAS,
        BIND_ENTRY_ALIAS,
        OUTPUT_CONFIG_ALIAS,
        WINDOW_RULE_CONFIG_ALIAS,
        CURSOR_POSITION_ALIAS,
        RESERVED_SPACE_ALIAS,
        FOCUS_MODE_ALIAS,
        KEYBOARD_LAYOUTS_ALIAS,
        SPAWN_OPTS_ALIAS,
        SPAWN_RESULT_ALIAS,
    ],
};

// ============================================================================
// Type Aliases
// ============================================================================

const WINDOW_ALIAS: AliasSchema = AliasSchema {
    name: "Window",
    ty: "{ id: integer, title: string?, app_id: string?, workspace_id: integer?, is_focused: boolean, is_floating: boolean }",
    description: "Window information table",
};

const WORKSPACE_ALIAS: AliasSchema = AliasSchema {
    name: "Workspace",
    ty: "{ id: integer, idx: integer, name: string?, output: string?, is_active: boolean, is_focused: boolean, active_window_id: integer? }",
    description: "Workspace information table",
};

const OUTPUT_ALIAS: AliasSchema = AliasSchema {
    name: "Output",
    ty: "{ name: string, make: string?, model: string?, serial: string?, physical_size: { width: integer, height: integer }?, current_mode: { width: integer, height: integer, refresh: integer }?, vrr_supported: boolean, vrr_enabled: boolean, x: integer?, y: integer?, scale: number?, logical_width: integer?, logical_height: integer?, transform: integer?, refresh_hz: number? }",
    description: "Output/monitor information table with position, scale, and transform",
};

const SIZE_CHANGE_ALIAS: AliasSchema = AliasSchema {
    name: "SizeChange",
    ty: "integer|string",
    description:
        "Size change value: integer for absolute, '+N'/'-N' for relative, 'N%' for percentage",
};

const POSITION_CHANGE_ALIAS: AliasSchema = AliasSchema {
    name: "PositionChange",
    ty: "integer|string",
    description: "Position change value: integer for absolute, '+N'/'-N' for relative",
};

const LAYOUT_SWITCH_TARGET_ALIAS: AliasSchema = AliasSchema {
    name: "LayoutSwitchTarget",
    ty: "\"next\"|\"prev\"|string",
    description: "Layout switch target: 'next', 'prev', or layout name",
};

const WORKSPACE_REFERENCE_ALIAS: AliasSchema = AliasSchema {
    name: "WorkspaceReference",
    ty: "integer|string|{ id: integer }|{ name: string }|{ index: integer }",
    description: "Workspace reference: index, name, or table with id/name/index",
};

const EVENT_HANDLER_ID_ALIAS: AliasSchema = AliasSchema {
    name: "EventHandlerId",
    ty: "integer",
    description: "Event handler identifier returned by niri.events:on() or :once()",
};

const EVENT_SPEC_ALIAS: AliasSchema = AliasSchema {
    name: "EventSpec",
    ty: "string|string[]",
    description: "Event specification: single event name or array of event names",
};

const HANDLER_ID_MAP_ALIAS: AliasSchema = AliasSchema {
    name: "HandlerIdMap",
    ty: "table<string, EventHandlerId>",
    description: "Map of event names to handler IDs, returned when registering multiple events",
};

const BIND_ENTRY_ALIAS: AliasSchema = AliasSchema {
    name: "BindEntry",
    ty: "{ key: string, action: string, args: any[]?, cooldown_ms: integer?, allow_when_locked: boolean? }",
    description: "Keybinding entry with key combination, action, and optional parameters",
};

const OUTPUT_CONFIG_ALIAS: AliasSchema = AliasSchema {
    name: "OutputConfig",
    ty: "{ name: string, mode: string?, scale: number?, position: { x: integer, y: integer }?, transform: string?, vrr: boolean? }",
    description: "Output/monitor configuration",
};

const WINDOW_RULE_CONFIG_ALIAS: AliasSchema = AliasSchema {
    name: "WindowRuleConfig",
    ty: "{ match: { app_id: string?, title: string?, is_floating: boolean?, at_startup: boolean? }?, default_column_width: table?, open_floating: boolean?, open_fullscreen: boolean?, open_maximized: boolean?, block_out_from: string?, opacity: number?, draw_border_with_background: boolean?, geometry_corner_radius: table?, clip_to_geometry: boolean?, focus_ring: table?, border: table? }",
    description: "Window rule configuration with match criteria and properties",
};

const CURSOR_POSITION_ALIAS: AliasSchema = AliasSchema {
    name: "CursorPosition",
    ty: "{ x: number, y: number, output: string }",
    description: "Cursor position with coordinates and output name",
};

const RESERVED_SPACE_ALIAS: AliasSchema = AliasSchema {
    name: "ReservedSpace",
    ty: "{ top: integer, bottom: integer, left: integer, right: integer }",
    description: "Reserved (exclusive) space on each edge from layer-shell surfaces",
};

const FOCUS_MODE_ALIAS: AliasSchema = AliasSchema {
    name: "FocusMode",
    ty: "\"normal\"|\"overview\"|\"layer_shell\"|\"locked\"",
    description: "Current compositor focus mode",
};

const KEYBOARD_LAYOUTS_ALIAS: AliasSchema = AliasSchema {
    name: "KeyboardLayouts",
    ty: "{ names: string[], current_idx: integer }",
    description: "Keyboard layout names and current active index",
};


const SPAWN_OPTS_ALIAS: AliasSchema = AliasSchema {
    name: "SpawnOpts",
    ty: "{ cwd: string?, env: table<string, string>?, clear_env: boolean?, stdin: string|boolean|\"pipe\"?, stdin_pipe: boolean?, capture_stdout: boolean?, capture_stderr: boolean?, text: boolean?, detach: boolean?, stdout: boolean|fun(stream: \"stdout\"|\"stderr\", err: string?, chunk: string?)?, stderr: boolean|fun(stream: \"stdout\"|\"stderr\", err: string?, chunk: string?)?, on_exit: fun(result: SpawnResult)? }",
    description: "Options for spawn() and spawn_sh(). cwd: working directory, env: environment variables, stdin: input mode, capture_*: buffer output for wait(), text: decode as UTF-8, detach: fire-and-forget, stdout/stderr: streaming callbacks, on_exit: exit callback",
};

const SPAWN_RESULT_ALIAS: AliasSchema = AliasSchema {
    name: "SpawnResult",
    ty: "{ code: integer?, signal: integer?, stdout: string, stderr: string }",
    description: "Result from ProcessHandle:wait() or on_exit callback. code and signal are mutually exclusive.",
};

// ============================================================================
// niri (root module)
// ============================================================================

const NIRI_MODULE: ModuleSchema = ModuleSchema {
    path: "niri",
    description: "Root niri namespace providing access to compositor functionality",
    functions: &[
        FunctionSchema {
            name: "version",
            description: "Returns the niri version string",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema { ty: "string", description: "niri version" }],
        },
        FunctionSchema {
            name: "print",
            description: "Print values to the niri log (info level)",
            is_method: false,
            params: &[ParamSchema { name: "...", ty: "any", description: "Values to print", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "apply_config",
            description: "Apply configuration from a Lua table",
            is_method: false,
            params: &[ParamSchema { name: "config", ty: "table", description: "Configuration table", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "schedule",
            description: "Schedule a callback to run on the next event loop iteration. Useful for deferring work or avoiding reentrancy issues.",
            is_method: false,
            params: &[ParamSchema { name: "callback", ty: "function", description: "Function to execute on next iteration", optional: false }],
            returns: &[],
        },
    ],
    fields: &[
        FieldSchema { name: "utils", ty: "niri.utils", description: "Utility functions" },
        FieldSchema { name: "config", ty: "niri.config", description: "Configuration API" },
        FieldSchema { name: "events", ty: "niri.events", description: "Event system for subscribing to compositor events" },
        FieldSchema { name: "action", ty: "niri.action", description: "Compositor actions" },
        FieldSchema { name: "state", ty: "niri.state", description: "Runtime state queries" },
        FieldSchema { name: "loop", ty: "niri.loop", description: "Event loop and timers" },
        FieldSchema { name: "keymap", ty: "niri.keymap", description: "Keybinding configuration" },
        FieldSchema { name: "window", ty: "niri.window", description: "Window rules configuration" },
        FieldSchema { name: "overview", ty: "niri.overview", description: "Overview mode configuration" },
        FieldSchema { name: "screenshot", ty: "niri.screenshot", description: "Screenshot configuration" },
        FieldSchema { name: "os", ty: "niri.os", description: "Operating system utilities (hostname, getenv)" },
    ],
};

// ============================================================================
// niri.utils
// ============================================================================

const NIRI_UTILS_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.utils",
    description: "Utility functions for logging and process spawning",
    functions: &[
        FunctionSchema {
            name: "log",
            description: "Log a message at info level",
            is_method: false,
            params: &[ParamSchema {
                name: "...",
                ty: "any",
                description: "Values to log",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "debug",
            description: "Log a message at debug level",
            is_method: false,
            params: &[ParamSchema {
                name: "...",
                ty: "any",
                description: "Values to log",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "warn",
            description: "Log a message at warning level",
            is_method: false,
            params: &[ParamSchema {
                name: "...",
                ty: "any",
                description: "Values to log",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "error",
            description: "Log a message at error level",
            is_method: false,
            params: &[ParamSchema {
                name: "...",
                ty: "any",
                description: "Values to log",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "spawn",
            description: "Spawn a command asynchronously",
            is_method: false,
            params: &[ParamSchema {
                name: "command",
                ty: "string[]",
                description: "Command and arguments",
                optional: false,
            }],
            returns: &[],
        },
    ],
    fields: &[],
};

// ============================================================================
// niri.config
// ============================================================================

const NIRI_CONFIG_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.config",
    description: "Configuration proxy for reading and modifying compositor settings",
    functions: &[
        FunctionSchema {
            name: "apply",
            description: "Apply all staged configuration changes to the compositor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "auto_apply",
            description: "Enable or disable automatic application of config changes",
            is_method: true,
            params: &[ParamSchema {
                name: "enable",
                ty: "boolean",
                description: "Whether to auto-apply changes",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "version",
            description: "Returns the config API version",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Config API version",
            }],
        },
    ],
    fields: &[
        // Scalar config sections (assignable as tables)
        FieldSchema {
            name: "input",
            ty: "ConfigSectionProxy",
            description: "Input device configuration (keyboard, mouse, touchpad, etc.)",
        },
        FieldSchema {
            name: "layout",
            ty: "ConfigSectionProxy",
            description: "Layout configuration (gaps, focus ring, border, shadow, etc.)",
        },
        FieldSchema {
            name: "cursor",
            ty: "ConfigSectionProxy",
            description: "Cursor configuration (size, theme, hide when typing)",
        },
        FieldSchema {
            name: "gestures",
            ty: "ConfigSectionProxy",
            description: "Gesture configuration (hot corners, touchpad gestures)",
        },
        FieldSchema {
            name: "recent_windows",
            ty: "ConfigSectionProxy",
            description: "Recent windows (MRU) configuration",
        },
        FieldSchema {
            name: "overview",
            ty: "ConfigSectionProxy",
            description: "Overview mode configuration (zoom, backdrop, shadows)",
        },
        FieldSchema {
            name: "animations",
            ty: "ConfigSectionProxy",
            description: "Animation configuration (off, slowdown)",
        },
        FieldSchema {
            name: "clipboard",
            ty: "ConfigSectionProxy",
            description: "Clipboard configuration",
        },
        FieldSchema {
            name: "hotkey_overlay",
            ty: "ConfigSectionProxy",
            description: "Hotkey overlay configuration",
        },
        FieldSchema {
            name: "config_notification",
            ty: "ConfigSectionProxy",
            description: "Config reload notification settings",
        },
        FieldSchema {
            name: "debug",
            ty: "ConfigSectionProxy",
            description: "Debug configuration options",
        },
        FieldSchema {
            name: "xwayland_satellite",
            ty: "ConfigSectionProxy",
            description: "Xwayland satellite configuration",
        },
        FieldSchema {
            name: "screenshot_path",
            ty: "string",
            description: "Screenshot save path pattern",
        },
        FieldSchema {
            name: "prefer_no_csd",
            ty: "boolean",
            description: "Prefer server-side decorations",
        },
        // Collection config sections (CRUD operations)
        FieldSchema {
            name: "binds",
            ty: "ConfigCollection",
            description: "Keybindings collection",
        },
        FieldSchema {
            name: "outputs",
            ty: "ConfigCollection",
            description: "Output/monitor configurations",
        },
        FieldSchema {
            name: "workspaces",
            ty: "ConfigCollection",
            description: "Named workspaces",
        },
        FieldSchema {
            name: "window_rules",
            ty: "ConfigCollection",
            description: "Window rules",
        },
        FieldSchema {
            name: "layer_rules",
            ty: "ConfigCollection",
            description: "Layer shell rules",
        },
        FieldSchema {
            name: "environment",
            ty: "ConfigCollection",
            description: "Environment variables",
        },
    ],
};

// ============================================================================
// niri.events
// ============================================================================

const NIRI_EVENTS_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.events",
    description: "Event system for subscribing to compositor events",
    functions: &[
        FunctionSchema {
            name: "on",
            description: "Subscribe to event(s) with a callback. Pass a single event name to get a handler ID, or an array of event names to get a table mapping event names to handler IDs.",
            is_method: true,
            params: &[
                ParamSchema { name: "event_spec", ty: "EventSpec", description: "Event name or array of event names (e.g., 'window:open' or {'window:open', 'window:close'})", optional: false },
                ParamSchema { name: "callback", ty: "fun(event: table)", description: "Callback function receiving event data", optional: false },
            ],
            returns: &[ReturnSchema { ty: "EventHandlerId|HandlerIdMap", description: "Handler ID (single event) or table of handler IDs (multiple events)" }],
        },
        FunctionSchema {
            name: "once",
            description: "Subscribe to event(s) for a single occurrence. Handler is automatically removed after firing. Pass a single event name to get a handler ID, or an array of event names to get a table mapping event names to handler IDs.",
            is_method: true,
            params: &[
                ParamSchema { name: "event_spec", ty: "EventSpec", description: "Event name or array of event names", optional: false },
                ParamSchema { name: "callback", ty: "fun(event: table)", description: "Callback function", optional: false },
            ],
            returns: &[ReturnSchema { ty: "EventHandlerId|HandlerIdMap", description: "Handler ID (single event) or table of handler IDs (multiple events)" }],
        },
        FunctionSchema {
            name: "off",
            description: "Unsubscribe from event(s). Pass (event_name, handler_id) to remove a single handler, or pass a HandlerIdMap table to remove multiple handlers at once.",
            is_method: true,
            params: &[
                ParamSchema { name: "event_or_map", ty: "string|HandlerIdMap", description: "Event name (with handler_id) or handler ID map from on()/once()", optional: false },
                ParamSchema { name: "handler_id", ty: "EventHandlerId", description: "Handler ID (only when first arg is event name)", optional: true },
            ],
            returns: &[ReturnSchema { ty: "boolean|table<string, boolean>", description: "True if handler removed (single) or table of results (multiple)" }],
        },
        FunctionSchema {
            name: "emit",
            description: "Emit a custom event (for testing or custom integrations)",
            is_method: true,
            params: &[
                ParamSchema { name: "event_name", ty: "string", description: "Event name", optional: false },
                ParamSchema { name: "data", ty: "table?", description: "Event data", optional: true },
            ],
            returns: &[],
        },
    ],
    fields: &[],
};

// ============================================================================
// niri.keymap
// ============================================================================

const NIRI_KEYMAP_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.keymap",
    description: "Keybinding configuration",
    functions: &[FunctionSchema {
        name: "set",
        description: "Set a keybinding",
        is_method: false,
        params: &[
            ParamSchema {
                name: "mode",
                ty: "string",
                description: "Binding mode (e.g., 'normal')",
                optional: false,
            },
            ParamSchema {
                name: "key",
                ty: "string",
                description: "Key combination (e.g., 'Mod+Return')",
                optional: false,
            },
            ParamSchema {
                name: "callback",
                ty: "fun()",
                description: "Callback function",
                optional: false,
            },
        ],
        returns: &[],
    }],
    fields: &[],
};

// ============================================================================
// niri.window
// ============================================================================

const NIRI_WINDOW_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.window",
    description: "Window rules configuration",
    functions: &[FunctionSchema {
        name: "rule",
        description: "Define a window rule",
        is_method: false,
        params: &[ParamSchema {
            name: "rule",
            ty: "table",
            description: "Window rule definition",
            optional: false,
        }],
        returns: &[],
    }],
    fields: &[],
};

// ============================================================================
// niri.overview
// ============================================================================

const NIRI_OVERVIEW_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.overview",
    description: "Overview mode configuration",
    functions: &[],
    fields: &[FieldSchema {
        name: "backdrop_color",
        ty: "string?",
        description: "Backdrop color in hex format",
    }],
};

// ============================================================================
// niri.screenshot
// ============================================================================

const NIRI_SCREENSHOT_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.screenshot",
    description: "Screenshot configuration",
    functions: &[],
    fields: &[FieldSchema {
        name: "path",
        ty: "string?",
        description: "Screenshot save path",
    }],
};

// ============================================================================
// niri.state
// ============================================================================

const NIRI_STATE_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.state",
    description: "Runtime state queries for windows, workspaces, and outputs",
    functions: &[
        FunctionSchema {
            name: "windows",
            description: "Get all windows",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "Window[]",
                description: "Array of window information",
            }],
        },
        FunctionSchema {
            name: "focused_window",
            description: "Get the currently focused window",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "Window?",
                description: "Focused window or nil",
            }],
        },
        FunctionSchema {
            name: "workspaces",
            description: "Get all workspaces",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "Workspace[]",
                description: "Array of workspace information",
            }],
        },
        FunctionSchema {
            name: "outputs",
            description: "Get all outputs/monitors with position, scale, and transform info",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "Output[]",
                description: "Array of output information",
            }],
        },
        FunctionSchema {
            name: "keyboard_layouts",
            description: "Get keyboard layout information",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "KeyboardLayouts",
                description: "Keyboard layout names and current index",
            }],
        },
        FunctionSchema {
            name: "cursor_position",
            description: "Get current cursor position and output",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "CursorPosition?",
                description: "Cursor position {x, y, output} or nil if no cursor",
            }],
        },
        FunctionSchema {
            name: "reserved_space",
            description: "Get reserved (exclusive) space for an output from layer-shell surfaces",
            is_method: false,
            params: &[ParamSchema {
                name: "output_name",
                ty: "string",
                description: "Output name to query",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "ReservedSpace?",
                description: "Reserved space {top, bottom, left, right} or nil if output not found",
            }],
        },
        FunctionSchema {
            name: "focus_mode",
            description: "Get current focus mode (normal, overview, layer_shell, or locked)",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "FocusMode",
                description: "Current focus mode string",
            }],
        },
    ],
    fields: &[],
};

// ============================================================================
// niri.loop
// ============================================================================

const NIRI_LOOP_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.loop",
    description: "Event loop and timer functionality",
    functions: &[
        FunctionSchema {
            name: "new_timer",
            description: "Create a new timer",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "Timer",
                description: "New timer instance",
            }],
        },
        FunctionSchema {
            name: "now",
            description: "Get current time in milliseconds since compositor start",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "integer",
                description: "Milliseconds since start",
            }],
        },
    ],
    fields: &[],
};

// ============================================================================
// niri.action
// ============================================================================

const NIRI_ACTION_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.action",
    description: "Compositor actions for window management, navigation, and system control",
    functions: &[
        // === System ===
        FunctionSchema {
            name: "quit",
            description: "Quit the compositor",
            is_method: true,
            params: &[ParamSchema {
                name: "skip_confirmation",
                ty: "boolean",
                description: "Skip confirmation dialog",
                optional: true,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "power_off_monitors",
            description: "Turn off all monitors",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "power_on_monitors",
            description: "Turn on all monitors",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "spawn",
            description: "Spawn a command. Without opts: fire-and-forget, returns nil. With opts: returns ProcessHandle for process control.",
            is_method: true,
            params: &[
                ParamSchema {
                    name: "command",
                    ty: "string[]",
                    description: "Command and arguments",
                    optional: false,
                },
                ParamSchema {
                    name: "opts",
                    ty: "SpawnOpts",
                    description: "Spawn options (cwd, env, stdin, capture, callbacks)",
                    optional: true,
                },
            ],
            returns: &[ReturnSchema {
                ty: "ProcessHandle?",
                description: "Process handle when opts provided and detach is not true, nil otherwise",
            }],
        },
        FunctionSchema {
            name: "spawn_sh",
            description: "Spawn a shell command. Without opts: fire-and-forget, returns nil. With opts: returns ProcessHandle for process control.",
            is_method: true,
            params: &[
                ParamSchema {
                    name: "command",
                    ty: "string",
                    description: "Shell command string",
                    optional: false,
                },
                ParamSchema {
                    name: "opts",
                    ty: "SpawnOpts",
                    description: "Spawn options (cwd, env, stdin, capture, callbacks)",
                    optional: true,
                },
            ],
            returns: &[ReturnSchema {
                ty: "ProcessHandle?",
                description: "Process handle when opts provided and detach is not true, nil otherwise",
            }],
        },
        FunctionSchema {
            name: "do_screen_transition",
            description: "Trigger a screen transition animation",
            is_method: true,
            params: &[ParamSchema {
                name: "delay",
                ty: "boolean",
                description: "Whether to delay the transition",
                optional: true,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "load_config_file",
            description: "Reload configuration from file",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Screenshot ===
        FunctionSchema {
            name: "screenshot",
            description: "Take a screenshot (interactive selection)",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "screenshot_screen",
            description: "Screenshot the entire screen",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "screenshot_window",
            description: "Screenshot the focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Window ===
        FunctionSchema {
            name: "close_window",
            description: "Close the focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "fullscreen_window",
            description: "Toggle fullscreen on the focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "toggle_windowed_fullscreen",
            description: "Toggle windowed fullscreen mode",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window",
            description: "Focus a specific window by ID",
            is_method: true,
            params: &[ParamSchema {
                name: "window_id",
                ty: "integer",
                description: "Window ID",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_in_column",
            description: "Focus the window at index within current column",
            is_method: true,
            params: &[ParamSchema {
                name: "index",
                ty: "integer",
                description: "1-based window index",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_previous",
            description: "Focus the previously focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "toggle_keyboard_shortcuts_inhibit",
            description: "Toggle keyboard shortcuts inhibit for focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Column Focus ===
        FunctionSchema {
            name: "focus_column_left",
            description: "Focus the column to the left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column_right",
            description: "Focus the column to the right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column_first",
            description: "Focus the first column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column_last",
            description: "Focus the last column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column_right_or_first",
            description: "Focus column to the right, wrapping to first",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column_left_or_last",
            description: "Focus column to the left, wrapping to last",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column",
            description: "Focus column at specific index",
            is_method: true,
            params: &[ParamSchema {
                name: "index",
                ty: "integer",
                description: "1-based column index",
                optional: false,
            }],
            returns: &[],
        },
        // === Window Focus ===
        FunctionSchema {
            name: "focus_window_down",
            description: "Focus the window below in the column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_up",
            description: "Focus the window above in the column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_down_or_column_left",
            description: "Focus window below or column left if at bottom",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_down_or_column_right",
            description: "Focus window below or column right if at bottom",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_up_or_column_left",
            description: "Focus window above or column left if at top",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_up_or_column_right",
            description: "Focus window above or column right if at top",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_or_monitor_up",
            description: "Focus window above or monitor above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_or_monitor_down",
            description: "Focus window below or monitor below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column_or_monitor_left",
            description: "Focus column left or monitor left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_column_or_monitor_right",
            description: "Focus column right or monitor right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Column Move ===
        FunctionSchema {
            name: "move_column_left",
            description: "Move column to the left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_right",
            description: "Move column to the right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_first",
            description: "Move column to first position",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_last",
            description: "Move column to last position",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_left_or_to_monitor_left",
            description: "Move column left or to monitor left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_right_or_to_monitor_right",
            description: "Move column right or to monitor right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_index",
            description: "Move column to specific index",
            is_method: true,
            params: &[ParamSchema {
                name: "index",
                ty: "integer",
                description: "1-based target index",
                optional: false,
            }],
            returns: &[],
        },
        // === Window Move ===
        FunctionSchema {
            name: "move_window_down",
            description: "Move window down within column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_up",
            description: "Move window up within column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_down_or_to_workspace_down",
            description: "Move window down or to workspace below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_up_or_to_workspace_up",
            description: "Move window up or to workspace above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Consume/Expel ===
        FunctionSchema {
            name: "consume_or_expel_window_left",
            description: "Consume window from left column or expel to left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "consume_or_expel_window_right",
            description: "Consume window from right column or expel to right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "consume_window_into_column",
            description: "Consume adjacent window into current column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "expel_window_from_column",
            description: "Expel focused window from column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "swap_window_right",
            description: "Swap window with the one to the right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "swap_window_left",
            description: "Swap window with the one to the left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Column Display ===
        FunctionSchema {
            name: "toggle_column_tabbed_display",
            description: "Toggle tabbed display for column",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "set_column_display",
            description: "Set column display mode",
            is_method: true,
            params: &[ParamSchema {
                name: "mode",
                ty: "\"normal\"|\"tabbed\"",
                description: "Display mode",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "center_column",
            description: "Center the current column on screen",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "center_window",
            description: "Center the focused window on screen",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "center_visible_columns",
            description: "Center all visible columns",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Workspace ===
        FunctionSchema {
            name: "focus_workspace_down",
            description: "Focus the workspace below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_workspace_up",
            description: "Focus the workspace above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_workspace",
            description: "Focus a specific workspace",
            is_method: true,
            params: &[ParamSchema {
                name: "reference",
                ty: "WorkspaceReference",
                description: "Workspace to focus",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_workspace_previous",
            description: "Focus the previously active workspace",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_workspace_down",
            description: "Move window to workspace below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_workspace_up",
            description: "Move window to workspace above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_workspace",
            description: "Move window to specific workspace",
            is_method: true,
            params: &[ParamSchema {
                name: "reference",
                ty: "WorkspaceReference",
                description: "Target workspace",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_workspace_down",
            description: "Move column to workspace below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_workspace_up",
            description: "Move column to workspace above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_workspace",
            description: "Move column to specific workspace",
            is_method: true,
            params: &[ParamSchema {
                name: "reference",
                ty: "WorkspaceReference",
                description: "Target workspace",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_down",
            description: "Move current workspace down",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_up",
            description: "Move current workspace up",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "set_workspace_name",
            description: "Set the name of current workspace",
            is_method: true,
            params: &[ParamSchema {
                name: "name",
                ty: "string",
                description: "Workspace name",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "unset_workspace_name",
            description: "Clear the name of current workspace",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Monitor Focus ===
        FunctionSchema {
            name: "focus_monitor_left",
            description: "Focus monitor to the left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_monitor_right",
            description: "Focus monitor to the right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_monitor_down",
            description: "Focus monitor below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_monitor_up",
            description: "Focus monitor above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_monitor_previous",
            description: "Focus previously active monitor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_monitor_next",
            description: "Focus next monitor in order",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_monitor",
            description: "Focus specific monitor by name",
            is_method: true,
            params: &[ParamSchema {
                name: "name",
                ty: "string",
                description: "Monitor name",
                optional: false,
            }],
            returns: &[],
        },
        // === Window to Monitor ===
        FunctionSchema {
            name: "move_window_to_monitor_left",
            description: "Move window to monitor on the left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_monitor_right",
            description: "Move window to monitor on the right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_monitor_down",
            description: "Move window to monitor below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_monitor_up",
            description: "Move window to monitor above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_monitor_previous",
            description: "Move window to previously active monitor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_monitor_next",
            description: "Move window to next monitor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_monitor",
            description: "Move window to specific monitor",
            is_method: true,
            params: &[ParamSchema {
                name: "name",
                ty: "string",
                description: "Monitor name",
                optional: false,
            }],
            returns: &[],
        },
        // === Column to Monitor ===
        FunctionSchema {
            name: "move_column_to_monitor_left",
            description: "Move column to monitor on the left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_monitor_right",
            description: "Move column to monitor on the right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_monitor_down",
            description: "Move column to monitor below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_monitor_up",
            description: "Move column to monitor above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_monitor_previous",
            description: "Move column to previously active monitor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_monitor_next",
            description: "Move column to next monitor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_column_to_monitor",
            description: "Move column to specific monitor",
            is_method: true,
            params: &[ParamSchema {
                name: "name",
                ty: "string",
                description: "Monitor name",
                optional: false,
            }],
            returns: &[],
        },
        // === Workspace to Monitor ===
        FunctionSchema {
            name: "move_workspace_to_monitor_left",
            description: "Move workspace to monitor on the left",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_to_monitor_right",
            description: "Move workspace to monitor on the right",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_to_monitor_down",
            description: "Move workspace to monitor below",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_to_monitor_up",
            description: "Move workspace to monitor above",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_to_monitor_previous",
            description: "Move workspace to previously active monitor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_to_monitor_next",
            description: "Move workspace to next monitor",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_workspace_to_monitor",
            description: "Move workspace to specific monitor",
            is_method: true,
            params: &[ParamSchema {
                name: "name",
                ty: "string",
                description: "Monitor name",
                optional: false,
            }],
            returns: &[],
        },
        // === Size/Width/Height ===
        FunctionSchema {
            name: "set_window_width",
            description: "Set the window width",
            is_method: true,
            params: &[ParamSchema {
                name: "change",
                ty: "SizeChange",
                description: "Width change value",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "set_window_height",
            description: "Set the window height",
            is_method: true,
            params: &[ParamSchema {
                name: "change",
                ty: "SizeChange",
                description: "Height change value",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "reset_window_height",
            description: "Reset window height to default",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "switch_preset_column_width",
            description: "Switch to next preset column width",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "switch_preset_column_width_reverse",
            description: "Switch to previous preset column width",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "switch_preset_window_width",
            description: "Switch to next preset window width",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "switch_preset_window_width_reverse",
            description: "Switch to previous preset window width",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "switch_preset_window_height",
            description: "Switch to next preset window height",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "switch_preset_window_height_reverse",
            description: "Switch to previous preset window height",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "maximize_column",
            description: "Maximize column to fill workspace",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "maximize_window_to_edges",
            description: "Maximize window to edges",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "set_column_width",
            description: "Set column width",
            is_method: true,
            params: &[ParamSchema {
                name: "change",
                ty: "SizeChange",
                description: "Width change value",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "expand_column_to_available_width",
            description: "Expand column to fill available width",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Layout ===
        FunctionSchema {
            name: "switch_layout",
            description: "Switch keyboard layout",
            is_method: true,
            params: &[ParamSchema {
                name: "target",
                ty: "LayoutSwitchTarget",
                description: "Layout to switch to",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "show_hotkey_overlay",
            description: "Show the hotkey overlay",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Debug ===
        FunctionSchema {
            name: "toggle_debug_tint",
            description: "Toggle debug tint visualization",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "debug_toggle_opaque_regions",
            description: "Toggle opaque regions debug visualization",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "debug_toggle_damage",
            description: "Toggle damage debug visualization",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Floating ===
        FunctionSchema {
            name: "toggle_window_floating",
            description: "Toggle floating state of focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_floating",
            description: "Move focused window to floating layer",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_window_to_tiling",
            description: "Move focused window to tiling layer",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_floating",
            description: "Focus floating layer",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_tiling",
            description: "Focus tiling layer",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "switch_focus_between_floating_and_tiling",
            description: "Switch focus between floating and tiling layers",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "move_floating_window",
            description: "Move floating window by offset",
            is_method: true,
            params: &[
                ParamSchema {
                    name: "x",
                    ty: "PositionChange",
                    description: "X offset",
                    optional: false,
                },
                ParamSchema {
                    name: "y",
                    ty: "PositionChange",
                    description: "Y offset",
                    optional: false,
                },
            ],
            returns: &[],
        },
        FunctionSchema {
            name: "toggle_window_rule_opacity",
            description: "Toggle opacity rule for focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Dynamic Cast ===
        FunctionSchema {
            name: "set_dynamic_cast_window",
            description: "Set dynamic cast target to window",
            is_method: true,
            params: &[ParamSchema {
                name: "window_id",
                ty: "integer?",
                description: "Window ID or nil for focused",
                optional: true,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "set_dynamic_cast_monitor",
            description: "Set dynamic cast target to monitor",
            is_method: true,
            params: &[ParamSchema {
                name: "monitor",
                ty: "string?",
                description: "Monitor name or nil for focused",
                optional: true,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "clear_dynamic_cast_target",
            description: "Clear dynamic cast target",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Overview ===
        FunctionSchema {
            name: "toggle_overview",
            description: "Toggle overview mode",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "open_overview",
            description: "Open overview mode",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "close_overview",
            description: "Close overview mode",
            is_method: true,
            params: &[],
            returns: &[],
        },
        // === Window Urgent ===
        FunctionSchema {
            name: "toggle_window_urgent",
            description: "Toggle urgent state of focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "set_window_urgent",
            description: "Set focused window as urgent",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "unset_window_urgent",
            description: "Clear urgent state of focused window",
            is_method: true,
            params: &[],
            returns: &[],
        },
    ],
    fields: &[],
};

// ============================================================================
// UserData Types
// ============================================================================

const TIMER_TYPE: TypeSchema = TypeSchema {
    name: "Timer",
    description: "A timer for scheduling callbacks",
    fields: &[FieldSchema {
        name: "id",
        ty: "integer",
        description: "Unique timer identifier",
    }],
    methods: &[
        FunctionSchema {
            name: "start",
            description: "Start the timer with a delay and optional repeat interval",
            is_method: true,
            params: &[
                ParamSchema {
                    name: "delay_ms",
                    ty: "integer",
                    description: "Delay in milliseconds before first callback",
                    optional: false,
                },
                ParamSchema {
                    name: "repeat_ms",
                    ty: "integer",
                    description: "Repeat interval in milliseconds (0 for one-shot)",
                    optional: true,
                },
                ParamSchema {
                    name: "callback",
                    ty: "fun()",
                    description: "Callback function",
                    optional: false,
                },
            ],
            returns: &[ReturnSchema {
                ty: "Timer",
                description: "Self for chaining",
            }],
        },
        FunctionSchema {
            name: "stop",
            description: "Stop the timer without closing it",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "Timer",
                description: "Self for chaining",
            }],
        },
        FunctionSchema {
            name: "close",
            description: "Stop and close the timer, releasing resources",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "is_active",
            description: "Check if the timer is currently active",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "boolean",
                description: "True if timer is active",
            }],
        },
    ],
};

const PROCESS_HANDLE_TYPE: TypeSchema = TypeSchema {
    name: "ProcessHandle",
    description: "Handle for a spawned process, returned by spawn()/spawn_sh() when opts are provided",
    fields: &[FieldSchema {
        name: "pid",
        ty: "integer",
        description: "Process ID (read-only)",
    }],
    methods: &[
        FunctionSchema {
            name: "wait",
            description: "Wait for process to exit. With timeout: sends SIGTERM then SIGKILL after grace period.",
            is_method: true,
            params: &[ParamSchema {
                name: "timeout_ms",
                ty: "integer",
                description: "Timeout in milliseconds (optional)",
                optional: true,
            }],
            returns: &[ReturnSchema {
                ty: "SpawnResult",
                description: "Process exit result with code/signal and captured output",
            }],
        },
        FunctionSchema {
            name: "kill",
            description: "Send signal to process. Default is SIGTERM.",
            is_method: true,
            params: &[ParamSchema {
                name: "signal",
                ty: "integer|string",
                description: "Signal number or name (e.g., 15, 'TERM', 'KILL')",
                optional: true,
            }],
            returns: &[ReturnSchema {
                ty: "boolean",
                description: "True if signal was sent successfully",
            }],
        },
        FunctionSchema {
            name: "write",
            description: "Write data to process stdin (only works if stdin='pipe')",
            is_method: true,
            params: &[ParamSchema {
                name: "data",
                ty: "string",
                description: "Data to write",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "boolean",
                description: "True if write succeeded",
            }],
        },
        FunctionSchema {
            name: "close_stdin",
            description: "Close the stdin pipe",
            is_method: true,
            params: &[],
            returns: &[],
        },
        FunctionSchema {
            name: "is_closing",
            description: "Check if stdin has been closed",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "boolean",
                description: "True if stdin is closed",
            }],
        },
    ],
};

const ANIMATION_TYPE: TypeSchema = TypeSchema {
    name: "Animation",
    description: "Animation configuration",
    fields: &[
        FieldSchema {
            name: "name",
            ty: "string",
            description: "Animation name",
        },
        FieldSchema {
            name: "duration_ms",
            ty: "integer",
            description: "Duration in milliseconds",
        },
        FieldSchema {
            name: "curve",
            ty: "string",
            description: "Easing curve",
        },
    ],
    methods: &[
        FunctionSchema {
            name: "get_name",
            description: "Get the animation name",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_duration",
            description: "Get the duration in milliseconds",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "integer",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_curve",
            description: "Get the easing curve",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "",
            }],
        },
        FunctionSchema {
            name: "with_duration",
            description: "Create a copy with different duration",
            is_method: true,
            params: &[ParamSchema {
                name: "ms",
                ty: "integer",
                description: "Duration in milliseconds",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "Animation",
                description: "New animation with updated duration",
            }],
        },
        FunctionSchema {
            name: "with_curve",
            description: "Create a copy with different easing curve",
            is_method: true,
            params: &[ParamSchema {
                name: "curve",
                ty: "string",
                description: "Easing curve name",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "Animation",
                description: "New animation with updated curve",
            }],
        },
    ],
};

const FILTER_TYPE: TypeSchema = TypeSchema {
    name: "Filter",
    description: "Window filter for matching windows",
    fields: &[],
    methods: &[
        FunctionSchema {
            name: "matches",
            description: "Check if a window matches this filter",
            is_method: true,
            params: &[
                ParamSchema {
                    name: "app_id",
                    ty: "string?",
                    description: "Application ID",
                    optional: false,
                },
                ParamSchema {
                    name: "title",
                    ty: "string?",
                    description: "Window title",
                    optional: false,
                },
            ],
            returns: &[ReturnSchema {
                ty: "boolean",
                description: "True if window matches",
            }],
        },
        FunctionSchema {
            name: "get_app_id",
            description: "Get the app_id pattern",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_title",
            description: "Get the title pattern",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "",
            }],
        },
    ],
};

const WINDOW_RULE_TYPE: TypeSchema = TypeSchema {
    name: "WindowRule",
    description: "Window rule configuration",
    fields: &[],
    methods: &[
        FunctionSchema {
            name: "matches",
            description: "Check if a window matches this rule",
            is_method: true,
            params: &[
                ParamSchema {
                    name: "app_id",
                    ty: "string?",
                    description: "Application ID",
                    optional: false,
                },
                ParamSchema {
                    name: "title",
                    ty: "string?",
                    description: "Window title",
                    optional: false,
                },
            ],
            returns: &[ReturnSchema {
                ty: "boolean",
                description: "True if window matches",
            }],
        },
        FunctionSchema {
            name: "get_floating",
            description: "Get floating setting",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "boolean?",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_fullscreen",
            description: "Get fullscreen setting",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "boolean?",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_tile",
            description: "Get tile setting",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "boolean?",
                description: "",
            }],
        },
        FunctionSchema {
            name: "with_floating",
            description: "Create a copy with floating setting",
            is_method: true,
            params: &[ParamSchema {
                name: "floating",
                ty: "boolean",
                description: "Floating state",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "WindowRule",
                description: "New rule with updated setting",
            }],
        },
        FunctionSchema {
            name: "with_fullscreen",
            description: "Create a copy with fullscreen setting",
            is_method: true,
            params: &[ParamSchema {
                name: "fullscreen",
                ty: "boolean",
                description: "Fullscreen state",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "WindowRule",
                description: "New rule with updated setting",
            }],
        },
        FunctionSchema {
            name: "with_tile",
            description: "Create a copy with tile setting",
            is_method: true,
            params: &[ParamSchema {
                name: "tile",
                ty: "boolean",
                description: "Tile state",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "WindowRule",
                description: "New rule with updated setting",
            }],
        },
    ],
};

const GESTURE_TYPE: TypeSchema = TypeSchema {
    name: "Gesture",
    description: "Gesture configuration",
    fields: &[],
    methods: &[
        FunctionSchema {
            name: "get_type",
            description: "Get the gesture type",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_fingers",
            description: "Get the number of fingers",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "integer",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_direction",
            description: "Get the gesture direction",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "",
            }],
        },
        FunctionSchema {
            name: "get_action",
            description: "Get the bound action",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "",
            }],
        },
        FunctionSchema {
            name: "with_direction",
            description: "Create a copy with different direction",
            is_method: true,
            params: &[ParamSchema {
                name: "direction",
                ty: "string",
                description: "Gesture direction",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "Gesture",
                description: "New gesture with updated direction",
            }],
        },
        FunctionSchema {
            name: "with_action",
            description: "Create a copy with different action",
            is_method: true,
            params: &[ParamSchema {
                name: "action",
                ty: "string",
                description: "Action to bind",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "Gesture",
                description: "New gesture with updated action",
            }],
        },
    ],
};

const CONFIG_COLLECTION_TYPE: TypeSchema = TypeSchema {
    name: "ConfigCollection",
    description:
        "Collection proxy for CRUD operations on config arrays (binds, outputs, window_rules, etc.)",
    fields: &[],
    methods: &[
        FunctionSchema {
            name: "list",
            description: "Get all items in the collection",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema {
                ty: "table[]",
                description: "Array of all items",
            }],
        },
        FunctionSchema {
            name: "get",
            description: "Get items matching criteria",
            is_method: true,
            params: &[ParamSchema {
                name: "criteria",
                ty: "table",
                description: "Match criteria (e.g., { key = 'Mod+T' })",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "table[]",
                description: "Matching items",
            }],
        },
        FunctionSchema {
            name: "add",
            description: "Add one or more items to the collection",
            is_method: true,
            params: &[ParamSchema {
                name: "items",
                ty: "table|table[]",
                description: "Item or array of items to add",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "set",
            description: "Replace the entire collection with new items",
            is_method: true,
            params: &[ParamSchema {
                name: "items",
                ty: "table[]",
                description: "New items to replace collection",
                optional: false,
            }],
            returns: &[],
        },
        FunctionSchema {
            name: "remove",
            description: "Remove items matching criteria",
            is_method: true,
            params: &[ParamSchema {
                name: "criteria",
                ty: "table",
                description: "Match criteria for removal",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "integer",
                description: "Number of items removed",
            }],
        },
        FunctionSchema {
            name: "clear",
            description: "Remove all items from the collection",
            is_method: true,
            params: &[],
            returns: &[],
        },
    ],
};

const CONFIG_SECTION_PROXY_TYPE: TypeSchema = TypeSchema {
    name: "ConfigSectionProxy",
    description:
        "Proxy for config sections supporting direct table assignment and nested property access",
    fields: &[],
    methods: &[],
};

// ============================================================================
// niri.os
// ============================================================================

const NIRI_OS_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.os",
    description: "Operating system utilities for conditional configuration",
    functions: &[
        // === System Info (F1.1) ===
        FunctionSchema {
            name: "hostname",
            description: "Get the system hostname. Throws on invalid UTF-8 (rare); returns empty string on other system errors.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "System hostname",
            }],
        },
        FunctionSchema {
            name: "getenv",
            description: "Get the value of an environment variable. Returns nil if not set, empty string if set to empty.",
            is_method: false,
            params: &[ParamSchema {
                name: "name",
                ty: "string",
                description: "Environment variable name",
                optional: false,
            }],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "Variable value or nil if not set",
            }],
        },
        FunctionSchema {
            name: "username",
            description: "Get the current username. Returns nil if unavailable.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "Current username or nil",
            }],
        },
        FunctionSchema {
            name: "home",
            description: "Get the user's home directory path. Returns nil if unavailable.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "Home directory path or nil",
            }],
        },
        FunctionSchema {
            name: "tmpdir",
            description: "Get the system temporary directory path.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Temporary directory path",
            }],
        },
        FunctionSchema {
            name: "platform",
            description: "Get the operating system name (e.g., 'linux', 'macos', 'windows').",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Operating system name",
            }],
        },
        FunctionSchema {
            name: "arch",
            description: "Get the CPU architecture (e.g., 'x86_64', 'aarch64').",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "CPU architecture",
            }],
        },
        // === XDG Directories (F1.2) ===
        FunctionSchema {
            name: "xdg_config_home",
            description: "Get XDG_CONFIG_HOME (~/.config by default).",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Config home directory path",
            }],
        },
        FunctionSchema {
            name: "xdg_data_home",
            description: "Get XDG_DATA_HOME (~/.local/share by default).",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Data home directory path",
            }],
        },
        FunctionSchema {
            name: "xdg_cache_home",
            description: "Get XDG_CACHE_HOME (~/.cache by default).",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Cache home directory path",
            }],
        },
        FunctionSchema {
            name: "xdg_state_home",
            description: "Get XDG_STATE_HOME (~/.local/state by default).",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "State home directory path",
            }],
        },
        FunctionSchema {
            name: "xdg_runtime_dir",
            description: "Get XDG_RUNTIME_DIR. Returns nil if not set (uncommon on modern systems).",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string?",
                description: "Runtime directory path or nil",
            }],
        },
        FunctionSchema {
            name: "xdg_data_dirs",
            description: "Get XDG_DATA_DIRS as an array of paths (/usr/local/share:/usr/share by default).",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string[]",
                description: "Array of data directory paths",
            }],
        },
        FunctionSchema {
            name: "xdg_config_dirs",
            description: "Get XDG_CONFIG_DIRS as an array of paths (/etc/xdg by default).",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string[]",
                description: "Array of config directory paths",
            }],
        },
        // === Niri-specific Directories (F2.4) ===
        FunctionSchema {
            name: "niri_config_dir",
            description: "Get the niri config directory ($XDG_CONFIG_HOME/niri). Creates if needed.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Path to niri config directory",
            }],
        },
        FunctionSchema {
            name: "niri_data_dir",
            description: "Get the niri data directory ($XDG_DATA_HOME/niri). Creates if needed.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Path to niri data directory",
            }],
        },
        FunctionSchema {
            name: "niri_cache_dir",
            description: "Get the niri cache directory ($XDG_CACHE_HOME/niri). Creates if needed.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Path to niri cache directory",
            }],
        },
        FunctionSchema {
            name: "niri_state_dir",
            description: "Get the niri state directory ($XDG_STATE_HOME/niri). Creates if needed.",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema {
                ty: "string",
                description: "Path to niri state directory",
            }],
        },
        // === Environment Mutation (F3.5) ===
        FunctionSchema {
            name: "setenv",
            description: "Set or remove an environment variable. Pass nil as value to remove. Changes only affect current process and children.",
            is_method: false,
            params: &[
                ParamSchema {
                    name: "name",
                    ty: "string",
                    description: "Environment variable name",
                    optional: false,
                },
                ParamSchema {
                    name: "value",
                    ty: "string?",
                    description: "Value to set, or nil to remove the variable",
                    optional: true,
                },
            ],
            returns: &[],
        },
    ],
    fields: &[],
};
