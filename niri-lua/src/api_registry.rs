// Niri Lua API Registry - Complete schema of the Lua API.
//
// This file is the single source of truth for the Lua API type definitions.
// It is used by build.rs to generate `types/api.lua` for LSP support.

use crate::lua_api_schema::*;

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
    ],
    types: &[
        TIMER_TYPE,
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
        BIND_ENTRY_ALIAS,
        OUTPUT_CONFIG_ALIAS,
        WINDOW_RULE_CONFIG_ALIAS,
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
    ty: "{ name: string, make: string?, model: string?, serial: string?, physical_size: { width: integer, height: integer }?, current_mode: { width: integer, height: integer, refresh: integer }?, vrr_supported: boolean, vrr_enabled: boolean }",
    description: "Output/monitor information table",
};

const SIZE_CHANGE_ALIAS: AliasSchema = AliasSchema {
    name: "SizeChange",
    ty: "integer|string",
    description: "Size change value: integer for absolute, '+N'/'-N' for relative, 'N%' for percentage",
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
        FieldSchema { name: "utils", ty: "niri_utils", description: "Utility functions" },
        FieldSchema { name: "config", ty: "niri_config", description: "Configuration API" },
        FieldSchema { name: "events", ty: "niri_events", description: "Event system for subscribing to compositor events" },
        FieldSchema { name: "action", ty: "niri_action", description: "Compositor actions" },
        FieldSchema { name: "state", ty: "niri_state", description: "Runtime state queries" },
        FieldSchema { name: "loop", ty: "niri_loop", description: "Event loop and timers" },
        FieldSchema { name: "keymap", ty: "niri_keymap", description: "Keybinding configuration" },
        FieldSchema { name: "window", ty: "niri_window", description: "Window rules configuration" },
        FieldSchema { name: "overview", ty: "niri_overview", description: "Overview mode configuration" },
        FieldSchema { name: "screenshot", ty: "niri_screenshot", description: "Screenshot configuration" },
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
            params: &[ParamSchema { name: "...", ty: "any", description: "Values to log", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "debug",
            description: "Log a message at debug level",
            is_method: false,
            params: &[ParamSchema { name: "...", ty: "any", description: "Values to log", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "warn",
            description: "Log a message at warning level",
            is_method: false,
            params: &[ParamSchema { name: "...", ty: "any", description: "Values to log", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "error",
            description: "Log a message at error level",
            is_method: false,
            params: &[ParamSchema { name: "...", ty: "any", description: "Values to log", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "spawn",
            description: "Spawn a command asynchronously",
            is_method: false,
            params: &[ParamSchema { name: "command", ty: "string[]", description: "Command and arguments", optional: false }],
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
            params: &[ParamSchema { name: "enable", ty: "boolean", description: "Whether to auto-apply changes", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "version",
            description: "Returns the config API version",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema { ty: "string", description: "Config API version" }],
        },
    ],
    fields: &[
        // Scalar config sections (assignable as tables)
        FieldSchema { name: "input", ty: "ConfigSectionProxy", description: "Input device configuration (keyboard, mouse, touchpad, etc.)" },
        FieldSchema { name: "layout", ty: "ConfigSectionProxy", description: "Layout configuration (gaps, focus ring, border, shadow, etc.)" },
        FieldSchema { name: "cursor", ty: "ConfigSectionProxy", description: "Cursor configuration (size, theme, hide when typing)" },
        FieldSchema { name: "gestures", ty: "ConfigSectionProxy", description: "Gesture configuration (hot corners, touchpad gestures)" },
        FieldSchema { name: "recent_windows", ty: "ConfigSectionProxy", description: "Recent windows (MRU) configuration" },
        FieldSchema { name: "overview", ty: "ConfigSectionProxy", description: "Overview mode configuration (zoom, backdrop, shadows)" },
        FieldSchema { name: "animations", ty: "ConfigSectionProxy", description: "Animation configuration (off, slowdown)" },
        FieldSchema { name: "clipboard", ty: "ConfigSectionProxy", description: "Clipboard configuration" },
        FieldSchema { name: "hotkey_overlay", ty: "ConfigSectionProxy", description: "Hotkey overlay configuration" },
        FieldSchema { name: "config_notification", ty: "ConfigSectionProxy", description: "Config reload notification settings" },
        FieldSchema { name: "debug", ty: "ConfigSectionProxy", description: "Debug configuration options" },
        FieldSchema { name: "xwayland_satellite", ty: "ConfigSectionProxy", description: "Xwayland satellite configuration" },
        FieldSchema { name: "screenshot_path", ty: "string", description: "Screenshot save path pattern" },
        FieldSchema { name: "prefer_no_csd", ty: "boolean", description: "Prefer server-side decorations" },
        // Collection config sections (CRUD operations)
        FieldSchema { name: "binds", ty: "ConfigCollection", description: "Keybindings collection" },
        FieldSchema { name: "outputs", ty: "ConfigCollection", description: "Output/monitor configurations" },
        FieldSchema { name: "workspaces", ty: "ConfigCollection", description: "Named workspaces" },
        FieldSchema { name: "window_rules", ty: "ConfigCollection", description: "Window rules" },
        FieldSchema { name: "layer_rules", ty: "ConfigCollection", description: "Layer shell rules" },
        FieldSchema { name: "environment", ty: "ConfigCollection", description: "Environment variables" },
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
            description: "Subscribe to an event with a callback. Returns a handler ID for later removal.",
            is_method: true,
            params: &[
                ParamSchema { name: "event_name", ty: "string", description: "Event name (e.g., 'window:open', 'workspace:activate')", optional: false },
                ParamSchema { name: "callback", ty: "fun(event: table)", description: "Callback function receiving event data", optional: false },
            ],
            returns: &[ReturnSchema { ty: "EventHandlerId", description: "Handler ID for removal" }],
        },
        FunctionSchema {
            name: "once",
            description: "Subscribe to an event for a single occurrence. Handler is automatically removed after firing.",
            is_method: true,
            params: &[
                ParamSchema { name: "event_name", ty: "string", description: "Event name", optional: false },
                ParamSchema { name: "callback", ty: "fun(event: table)", description: "Callback function", optional: false },
            ],
            returns: &[ReturnSchema { ty: "EventHandlerId", description: "Handler ID for early removal" }],
        },
        FunctionSchema {
            name: "off",
            description: "Unsubscribe from an event using the handler ID",
            is_method: true,
            params: &[
                ParamSchema { name: "event_name", ty: "string", description: "Event name", optional: false },
                ParamSchema { name: "handler_id", ty: "EventHandlerId", description: "Handler ID from on() or once()", optional: false },
            ],
            returns: &[ReturnSchema { ty: "boolean", description: "True if handler was found and removed" }],
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
    functions: &[
        FunctionSchema {
            name: "set",
            description: "Set a keybinding",
            is_method: false,
            params: &[
                ParamSchema { name: "mode", ty: "string", description: "Binding mode (e.g., 'normal')", optional: false },
                ParamSchema { name: "key", ty: "string", description: "Key combination (e.g., 'Mod+Return')", optional: false },
                ParamSchema { name: "callback", ty: "fun()", description: "Callback function", optional: false },
            ],
            returns: &[],
        },
    ],
    fields: &[],
};

// ============================================================================
// niri.window
// ============================================================================

const NIRI_WINDOW_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.window",
    description: "Window rules configuration",
    functions: &[
        FunctionSchema {
            name: "rule",
            description: "Define a window rule",
            is_method: false,
            params: &[ParamSchema { name: "rule", ty: "table", description: "Window rule definition", optional: false }],
            returns: &[],
        },
    ],
    fields: &[],
};

// ============================================================================
// niri.overview
// ============================================================================

const NIRI_OVERVIEW_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.overview",
    description: "Overview mode configuration",
    functions: &[],
    fields: &[
        FieldSchema { name: "backdrop_color", ty: "string?", description: "Backdrop color in hex format" },
    ],
};

// ============================================================================
// niri.screenshot
// ============================================================================

const NIRI_SCREENSHOT_MODULE: ModuleSchema = ModuleSchema {
    path: "niri.screenshot",
    description: "Screenshot configuration",
    functions: &[],
    fields: &[
        FieldSchema { name: "path", ty: "string?", description: "Screenshot save path" },
    ],
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
            returns: &[ReturnSchema { ty: "Window[]", description: "Array of window information" }],
        },
        FunctionSchema {
            name: "focused_window",
            description: "Get the currently focused window",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema { ty: "Window?", description: "Focused window or nil" }],
        },
        FunctionSchema {
            name: "workspaces",
            description: "Get all workspaces",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema { ty: "Workspace[]", description: "Array of workspace information" }],
        },
        FunctionSchema {
            name: "outputs",
            description: "Get all outputs/monitors",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema { ty: "Output[]", description: "Array of output information" }],
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
            returns: &[ReturnSchema { ty: "Timer", description: "New timer instance" }],
        },
        FunctionSchema {
            name: "now",
            description: "Get current time in milliseconds since compositor start",
            is_method: false,
            params: &[],
            returns: &[ReturnSchema { ty: "integer", description: "Milliseconds since start" }],
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
            params: &[ParamSchema { name: "skip_confirmation", ty: "boolean", description: "Skip confirmation dialog", optional: true }],
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
            description: "Spawn a command",
            is_method: true,
            params: &[ParamSchema { name: "command", ty: "string[]", description: "Command and arguments", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "spawn_sh",
            description: "Spawn a command via shell",
            is_method: true,
            params: &[ParamSchema { name: "command", ty: "string", description: "Shell command string", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "do_screen_transition",
            description: "Trigger a screen transition animation",
            is_method: true,
            params: &[ParamSchema { name: "delay", ty: "boolean", description: "Whether to delay the transition", optional: true }],
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
            params: &[ParamSchema { name: "window_id", ty: "integer", description: "Window ID", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "focus_window_in_column",
            description: "Focus the window at index within current column",
            is_method: true,
            params: &[ParamSchema { name: "index", ty: "integer", description: "1-based window index", optional: false }],
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
            params: &[ParamSchema { name: "index", ty: "integer", description: "1-based column index", optional: false }],
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
            params: &[ParamSchema { name: "index", ty: "integer", description: "1-based target index", optional: false }],
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
            params: &[ParamSchema { name: "mode", ty: "\"normal\"|\"tabbed\"", description: "Display mode", optional: false }],
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
            params: &[ParamSchema { name: "reference", ty: "WorkspaceReference", description: "Workspace to focus", optional: false }],
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
            params: &[ParamSchema { name: "reference", ty: "WorkspaceReference", description: "Target workspace", optional: false }],
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
            params: &[ParamSchema { name: "reference", ty: "WorkspaceReference", description: "Target workspace", optional: false }],
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
            params: &[ParamSchema { name: "name", ty: "string", description: "Workspace name", optional: false }],
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
            params: &[ParamSchema { name: "name", ty: "string", description: "Monitor name", optional: false }],
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
            params: &[ParamSchema { name: "name", ty: "string", description: "Monitor name", optional: false }],
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
            params: &[ParamSchema { name: "name", ty: "string", description: "Monitor name", optional: false }],
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
            params: &[ParamSchema { name: "name", ty: "string", description: "Monitor name", optional: false }],
            returns: &[],
        },
        // === Size/Width/Height ===
        FunctionSchema {
            name: "set_window_width",
            description: "Set the window width",
            is_method: true,
            params: &[ParamSchema { name: "change", ty: "SizeChange", description: "Width change value", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "set_window_height",
            description: "Set the window height",
            is_method: true,
            params: &[ParamSchema { name: "change", ty: "SizeChange", description: "Height change value", optional: false }],
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
            params: &[ParamSchema { name: "change", ty: "SizeChange", description: "Width change value", optional: false }],
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
            params: &[ParamSchema { name: "target", ty: "LayoutSwitchTarget", description: "Layout to switch to", optional: false }],
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
                ParamSchema { name: "x", ty: "PositionChange", description: "X offset", optional: false },
                ParamSchema { name: "y", ty: "PositionChange", description: "Y offset", optional: false },
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
            params: &[ParamSchema { name: "window_id", ty: "integer?", description: "Window ID or nil for focused", optional: true }],
            returns: &[],
        },
        FunctionSchema {
            name: "set_dynamic_cast_monitor",
            description: "Set dynamic cast target to monitor",
            is_method: true,
            params: &[ParamSchema { name: "monitor", ty: "string?", description: "Monitor name or nil for focused", optional: true }],
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
    fields: &[
        FieldSchema { name: "id", ty: "integer", description: "Unique timer identifier" },
    ],
    methods: &[
        FunctionSchema {
            name: "start",
            description: "Start the timer with a delay and optional repeat interval",
            is_method: true,
            params: &[
                ParamSchema { name: "delay_ms", ty: "integer", description: "Delay in milliseconds before first callback", optional: false },
                ParamSchema { name: "repeat_ms", ty: "integer", description: "Repeat interval in milliseconds (0 for one-shot)", optional: true },
                ParamSchema { name: "callback", ty: "fun()", description: "Callback function", optional: false },
            ],
            returns: &[ReturnSchema { ty: "Timer", description: "Self for chaining" }],
        },
        FunctionSchema {
            name: "stop",
            description: "Stop the timer without closing it",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "Timer", description: "Self for chaining" }],
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
            returns: &[ReturnSchema { ty: "boolean", description: "True if timer is active" }],
        },
    ],
};

const ANIMATION_TYPE: TypeSchema = TypeSchema {
    name: "Animation",
    description: "Animation configuration",
    fields: &[
        FieldSchema { name: "name", ty: "string", description: "Animation name" },
        FieldSchema { name: "duration_ms", ty: "integer", description: "Duration in milliseconds" },
        FieldSchema { name: "curve", ty: "string", description: "Easing curve" },
    ],
    methods: &[
        FunctionSchema {
            name: "get_name",
            description: "Get the animation name",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "string", description: "" }],
        },
        FunctionSchema {
            name: "get_duration",
            description: "Get the duration in milliseconds",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "integer", description: "" }],
        },
        FunctionSchema {
            name: "get_curve",
            description: "Get the easing curve",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "string", description: "" }],
        },
        FunctionSchema {
            name: "with_duration",
            description: "Create a copy with different duration",
            is_method: true,
            params: &[ParamSchema { name: "ms", ty: "integer", description: "Duration in milliseconds", optional: false }],
            returns: &[ReturnSchema { ty: "Animation", description: "New animation with updated duration" }],
        },
        FunctionSchema {
            name: "with_curve",
            description: "Create a copy with different easing curve",
            is_method: true,
            params: &[ParamSchema { name: "curve", ty: "string", description: "Easing curve name", optional: false }],
            returns: &[ReturnSchema { ty: "Animation", description: "New animation with updated curve" }],
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
                ParamSchema { name: "app_id", ty: "string?", description: "Application ID", optional: false },
                ParamSchema { name: "title", ty: "string?", description: "Window title", optional: false },
            ],
            returns: &[ReturnSchema { ty: "boolean", description: "True if window matches" }],
        },
        FunctionSchema {
            name: "get_app_id",
            description: "Get the app_id pattern",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "string?", description: "" }],
        },
        FunctionSchema {
            name: "get_title",
            description: "Get the title pattern",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "string?", description: "" }],
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
                ParamSchema { name: "app_id", ty: "string?", description: "Application ID", optional: false },
                ParamSchema { name: "title", ty: "string?", description: "Window title", optional: false },
            ],
            returns: &[ReturnSchema { ty: "boolean", description: "True if window matches" }],
        },
        FunctionSchema {
            name: "get_floating",
            description: "Get floating setting",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "boolean?", description: "" }],
        },
        FunctionSchema {
            name: "get_fullscreen",
            description: "Get fullscreen setting",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "boolean?", description: "" }],
        },
        FunctionSchema {
            name: "get_tile",
            description: "Get tile setting",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "boolean?", description: "" }],
        },
        FunctionSchema {
            name: "with_floating",
            description: "Create a copy with floating setting",
            is_method: true,
            params: &[ParamSchema { name: "floating", ty: "boolean", description: "Floating state", optional: false }],
            returns: &[ReturnSchema { ty: "WindowRule", description: "New rule with updated setting" }],
        },
        FunctionSchema {
            name: "with_fullscreen",
            description: "Create a copy with fullscreen setting",
            is_method: true,
            params: &[ParamSchema { name: "fullscreen", ty: "boolean", description: "Fullscreen state", optional: false }],
            returns: &[ReturnSchema { ty: "WindowRule", description: "New rule with updated setting" }],
        },
        FunctionSchema {
            name: "with_tile",
            description: "Create a copy with tile setting",
            is_method: true,
            params: &[ParamSchema { name: "tile", ty: "boolean", description: "Tile state", optional: false }],
            returns: &[ReturnSchema { ty: "WindowRule", description: "New rule with updated setting" }],
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
            returns: &[ReturnSchema { ty: "string", description: "" }],
        },
        FunctionSchema {
            name: "get_fingers",
            description: "Get the number of fingers",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "integer", description: "" }],
        },
        FunctionSchema {
            name: "get_direction",
            description: "Get the gesture direction",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "string?", description: "" }],
        },
        FunctionSchema {
            name: "get_action",
            description: "Get the bound action",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "string?", description: "" }],
        },
        FunctionSchema {
            name: "with_direction",
            description: "Create a copy with different direction",
            is_method: true,
            params: &[ParamSchema { name: "direction", ty: "string", description: "Gesture direction", optional: false }],
            returns: &[ReturnSchema { ty: "Gesture", description: "New gesture with updated direction" }],
        },
        FunctionSchema {
            name: "with_action",
            description: "Create a copy with different action",
            is_method: true,
            params: &[ParamSchema { name: "action", ty: "string", description: "Action to bind", optional: false }],
            returns: &[ReturnSchema { ty: "Gesture", description: "New gesture with updated action" }],
        },
    ],
};

const CONFIG_COLLECTION_TYPE: TypeSchema = TypeSchema {
    name: "ConfigCollection",
    description: "Collection proxy for CRUD operations on config arrays (binds, outputs, window_rules, etc.)",
    fields: &[],
    methods: &[
        FunctionSchema {
            name: "list",
            description: "Get all items in the collection",
            is_method: true,
            params: &[],
            returns: &[ReturnSchema { ty: "table[]", description: "Array of all items" }],
        },
        FunctionSchema {
            name: "get",
            description: "Get items matching criteria",
            is_method: true,
            params: &[ParamSchema { name: "criteria", ty: "table", description: "Match criteria (e.g., { key = 'Mod+T' })", optional: false }],
            returns: &[ReturnSchema { ty: "table[]", description: "Matching items" }],
        },
        FunctionSchema {
            name: "add",
            description: "Add one or more items to the collection",
            is_method: true,
            params: &[ParamSchema { name: "items", ty: "table|table[]", description: "Item or array of items to add", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "set",
            description: "Replace the entire collection with new items",
            is_method: true,
            params: &[ParamSchema { name: "items", ty: "table[]", description: "New items to replace collection", optional: false }],
            returns: &[],
        },
        FunctionSchema {
            name: "remove",
            description: "Remove items matching criteria",
            is_method: true,
            params: &[ParamSchema { name: "criteria", ty: "table", description: "Match criteria for removal", optional: false }],
            returns: &[ReturnSchema { ty: "integer", description: "Number of items removed" }],
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
    description: "Proxy for config sections supporting direct table assignment and nested property access",
    fields: &[],
    methods: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Verify that all modules have non-empty paths
    #[test]
    fn all_modules_have_valid_paths() {
        for module in NIRI_LUA_API.modules {
            assert!(!module.path.is_empty(), "Module has empty path");
            assert!(
                module.path.starts_with("niri"),
                "Module path '{}' should start with 'niri'",
                module.path
            );
        }
    }

    /// Verify that all modules have descriptions
    #[test]
    fn all_modules_have_descriptions() {
        for module in NIRI_LUA_API.modules {
            assert!(
                !module.description.is_empty(),
                "Module '{}' has empty description",
                module.path
            );
        }
    }

    /// Verify that all functions have non-empty names and descriptions
    #[test]
    fn all_functions_have_valid_names_and_descriptions() {
        for module in NIRI_LUA_API.modules {
            for func in module.functions {
                assert!(
                    !func.name.is_empty(),
                    "Function in module '{}' has empty name",
                    module.path
                );
                assert!(
                    !func.description.is_empty(),
                    "Function '{}' in module '{}' has empty description",
                    func.name,
                    module.path
                );
            }
        }
    }

    /// Verify that all function parameters have valid names and types
    #[test]
    fn all_function_params_are_valid() {
        for module in NIRI_LUA_API.modules {
            for func in module.functions {
                for param in func.params {
                    assert!(
                        !param.name.is_empty(),
                        "Parameter in function '{}::{}' has empty name",
                        module.path,
                        func.name
                    );
                    assert!(
                        !param.ty.is_empty(),
                        "Parameter '{}' in function '{}::{}' has empty type",
                        param.name,
                        module.path,
                        func.name
                    );
                }
            }
        }
    }

    /// Verify that all function returns have valid types
    #[test]
    fn all_function_returns_have_types() {
        for module in NIRI_LUA_API.modules {
            for func in module.functions {
                for ret in func.returns {
                    assert!(
                        !ret.ty.is_empty(),
                        "Return in function '{}::{}' has empty type",
                        module.path,
                        func.name
                    );
                }
            }
        }
    }

    /// Verify that all types have valid names and descriptions
    #[test]
    fn all_types_have_valid_names_and_descriptions() {
        for ty in NIRI_LUA_API.types {
            assert!(!ty.name.is_empty(), "Type has empty name");
            assert!(
                !ty.description.is_empty(),
                "Type '{}' has empty description",
                ty.name
            );
        }
    }

    /// Verify that all type methods have valid names and descriptions
    #[test]
    fn all_type_methods_are_valid() {
        for ty in NIRI_LUA_API.types {
            for method in ty.methods {
                assert!(
                    !method.name.is_empty(),
                    "Method in type '{}' has empty name",
                    ty.name
                );
                assert!(
                    !method.description.is_empty(),
                    "Method '{}' in type '{}' has empty description",
                    method.name,
                    ty.name
                );
                assert!(
                    method.is_method,
                    "Method '{}' in type '{}' should have is_method=true",
                    method.name,
                    ty.name
                );
            }
        }
    }

    /// Verify that all type aliases have valid names and types
    #[test]
    fn all_aliases_are_valid() {
        for alias in NIRI_LUA_API.aliases {
            assert!(!alias.name.is_empty(), "Alias has empty name");
            assert!(
                !alias.ty.is_empty(),
                "Alias '{}' has empty type",
                alias.name
            );
        }
    }

    /// Verify module paths are unique
    #[test]
    fn module_paths_are_unique() {
        let mut paths = HashSet::new();
        for module in NIRI_LUA_API.modules {
            assert!(
                paths.insert(module.path),
                "Duplicate module path: '{}'",
                module.path
            );
        }
    }

    /// Verify type names are unique
    #[test]
    fn type_names_are_unique() {
        let mut names = HashSet::new();
        for ty in NIRI_LUA_API.types {
            assert!(names.insert(ty.name), "Duplicate type name: '{}'", ty.name);
        }
    }

    /// Verify alias names are unique and don't conflict with type names
    #[test]
    fn alias_names_are_unique_and_no_type_conflicts() {
        let type_names: HashSet<_> = NIRI_LUA_API.types.iter().map(|t| t.name).collect();
        let mut alias_names = HashSet::new();

        for alias in NIRI_LUA_API.aliases {
            assert!(
                alias_names.insert(alias.name),
                "Duplicate alias name: '{}'",
                alias.name
            );
            assert!(
                !type_names.contains(alias.name),
                "Alias '{}' conflicts with a type name",
                alias.name
            );
        }
    }

    /// Verify expected modules are present
    #[test]
    fn expected_modules_present() {
        let module_paths: HashSet<_> = NIRI_LUA_API.modules.iter().map(|m| m.path).collect();

        let expected = [
            "niri",
            "niri.utils",
            "niri.config",
            "niri.action",
            "niri.state",
            "niri.loop",
            "niri.keymap",
        ];

        for path in expected {
            assert!(
                module_paths.contains(path),
                "Expected module '{}' not found",
                path
            );
        }
    }

    /// Verify expected types are present
    #[test]
    fn expected_types_present() {
        let type_names: HashSet<_> = NIRI_LUA_API.types.iter().map(|t| t.name).collect();

        let expected = ["Timer", "Animation", "Filter", "WindowRule", "Gesture"];

        for name in expected {
            assert!(
                type_names.contains(name),
                "Expected type '{}' not found",
                name
            );
        }
    }

    /// Verify expected aliases are present
    #[test]
    fn expected_aliases_present() {
        let alias_names: HashSet<_> = NIRI_LUA_API.aliases.iter().map(|a| a.name).collect();

        let expected = ["Window", "Workspace", "Output", "SizeChange", "WorkspaceReference"];

        for name in expected {
            assert!(
                alias_names.contains(name),
                "Expected alias '{}' not found",
                name
            );
        }
    }

    /// Count statistics for the API schema
    #[test]
    fn schema_statistics() {
        let module_count = NIRI_LUA_API.modules.len();
        let type_count = NIRI_LUA_API.types.len();
        let alias_count = NIRI_LUA_API.aliases.len();

        let function_count: usize = NIRI_LUA_API
            .modules
            .iter()
            .map(|m| m.functions.len())
            .sum();
        let field_count: usize = NIRI_LUA_API.modules.iter().map(|m| m.fields.len()).sum();
        let method_count: usize = NIRI_LUA_API.types.iter().map(|t| t.methods.len()).sum();

        // These assertions document expected minimums and will fail if schema shrinks unexpectedly
        assert!(module_count >= 7, "Expected at least 7 modules, got {module_count}");
        assert!(type_count >= 5, "Expected at least 5 types, got {type_count}");
        assert!(alias_count >= 5, "Expected at least 5 aliases, got {alias_count}");
        assert!(
            function_count >= 50,
            "Expected at least 50 functions, got {function_count}"
        );
        assert!(field_count >= 3, "Expected at least 3 fields, got {field_count}");
        assert!(method_count >= 10, "Expected at least 10 methods, got {method_count}");
    }

    /// Verify niri.action module has key compositor actions
    #[test]
    fn action_module_has_key_functions() {
        let action_module = NIRI_LUA_API
            .modules
            .iter()
            .find(|m| m.path == "niri.action")
            .expect("niri.action module not found");

        let func_names: HashSet<_> = action_module.functions.iter().map(|f| f.name).collect();

        let expected = [
            "quit",
            "spawn",
            "close_window",
            "focus_window_up",
            "focus_window_down",
            "move_window_up",
            "move_window_down",
        ];

        for name in expected {
            assert!(
                func_names.contains(name),
                "Expected action '{}' not found in niri.action",
                name
            );
        }
    }

    /// Verify niri.state module has state query functions
    #[test]
    fn state_module_has_query_functions() {
        let state_module = NIRI_LUA_API
            .modules
            .iter()
            .find(|m| m.path == "niri.state")
            .expect("niri.state module not found");

        let func_names: HashSet<_> = state_module.functions.iter().map(|f| f.name).collect();

        let expected = ["windows", "focused_window", "workspaces", "outputs"];

        for name in expected {
            assert!(
                func_names.contains(name),
                "Expected state query '{}' not found in niri.state",
                name
            );
        }
    }
}
