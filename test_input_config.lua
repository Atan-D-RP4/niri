-- Test Input Configuration for Niri
-- This file tests touchpad and mouse configuration from Lua

local input = {
    keyboard = {
        xkb = {
            layout = "us",
            variant = "intl",
        },
        numlock = true,
    },

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

    mouse = {
        natural_scroll = false,
        accel_speed = -0.2,
        accel_profile = "adaptive",
        left_handed = false,
        middle_emulation = true,
        scroll_method = "no-scroll",
    },
}

return {
    input = input,
}
