-- Test Input Configuration for Niri (v2 API)
-- This file tests keyboard, touchpad, mouse, and other input devices

niri.utils.log("Loading input configuration test")

-- Configure input using v2 proxy API
niri.config.input = {
    -- Keyboard configuration
    keyboard = {
        xkb = {
            layout = "us",
            variant = "intl",
        },
        numlock = true,
    },

    -- Touchpad configuration
    touchpad = {
        tap = true,
        natural_scroll = true,
        accel_speed = 0.3,
        accel_profile = "flat",
        scroll_method = "two-finger",
        dwt = true,
        dwtp = false,
        drag = true,
        drag_lock = false,
        left_handed = false,
        click_method = "clickfinger",
        tap_button_map = "left-right-middle",
    },

    -- Mouse configuration
    mouse = {
        natural_scroll = false,
        accel_speed = -0.2,
        accel_profile = "adaptive",
        left_handed = false,
        middle_emulation = true,
        scroll_method = "no-scroll",
    },

    -- Trackpoint configuration
    trackpoint = {
        natural_scroll = true,
        middle_emulation = true,
        accel_speed = 0.3,
        accel_profile = "flat",
        scroll_method = "on-button-down",
        scroll_button = 9,
    },

    -- Trackball configuration
    trackball = {
        natural_scroll = false,
        left_handed = true,
        scroll_button_lock = true,
        accel_speed = -0.2,
        accel_profile = "adaptive",
        scroll_method = "no-scroll",
    },

    -- Tablet configuration
    tablet = {
        left_handed = true,
        map_to_output = "HDMI-1",
    },

    -- Touch (touchscreen) configuration
    touch = {
        natural_scroll = true,
        map_to_output = "eDP-1",
    },

    -- Global input settings
    disable_power_key_handling = true,
    workspace_auto_back_and_forth = true,
}

-- Apply the configuration
niri.config:apply()

niri.utils.log("Input configuration loaded successfully")
