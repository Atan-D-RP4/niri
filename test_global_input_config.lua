-- Test configuration for global input settings
input = {
  -- Global settings
  disable_power_key_handling = true,
  workspace_auto_back_and_forth = true,
  mod_key = "Super",
  mod_key_nested = "Alt",
  
  -- Warp mouse to focus (table format)
  warp_mouse_to_focus = {
    mode = "center-xy-always"
  },
  
  -- Focus follows mouse (table format)
  focus_follows_mouse = {
    max_scroll_amount = 0.5
  },
  
  -- Trackpoint configuration
  trackpoint = {
    off = false,
    natural_scroll = true,
    left_handed = false,
    middle_emulation = true,
    scroll_button_lock = false,
    accel_speed = 0.3,
    accel_profile = "flat",
    scroll_method = "on-button-down",
    scroll_button = 9
  },
  
  -- Trackball configuration
  trackball = {
    off = false,
    natural_scroll = false,
    left_handed = true,
    middle_emulation = false,
    scroll_button_lock = true,
    accel_speed = -0.2,
    accel_profile = "adaptive",
    scroll_method = "no-scroll",
    scroll_button = 8
  },
  
  -- Tablet configuration
  tablet = {
    off = false,
    left_handed = true,
    map_to_output = "HDMI-1",
    calibration_matrix = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0}
  },
  
  -- Touch configuration
  touch = {
    off = false,
    map_to_output = "eDP-1",
    calibration_matrix = {0.9, 0.0, 0.1, 0.0, 0.9, 0.1}
  }
}
