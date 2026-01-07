use mlua::prelude::{LuaError, LuaValue, *};
use niri_config::appearance::TabIndicatorPosition;
use niri_config::debug::PreviewRender;
use niri_config::input::{AccelProfile, ScrollMethod, TapButtonMap};
use niri_config::CenterFocusedColumn;
use niri_config::PresetSize;
use std::path::PathBuf;

use crate::accessor_macros::{
    accessor_anim_kind, accessor_anim_kind_named, accessor_bool, accessor_color, accessor_enum,
    accessor_float, accessor_float_or_int, accessor_int, accessor_inverted_bool,
    accessor_option_bool, accessor_option_color, accessor_option_enum,
    accessor_option_enum_normalize, accessor_option_gradient, accessor_option_int,
    accessor_option_path, accessor_option_string, accessor_string, accessor_wrapped_option_string,
};
use crate::property_registry::PropertyRegistry;
use crate::traits::{preset_sizes_from_lua, preset_sizes_to_lua, LuaFieldConvert, PresetSizeTable};

pub fn register_config_accessors(registry: &mut PropertyRegistry) {
    accessor_bool!(registry, "animations.off", animations.off);
    accessor_float!(registry, "animations.slowdown", animations.slowdown);

    accessor_anim_kind!(
        registry,
        "animations.workspace_switch.kind",
        animations.workspace_switch
    );
    accessor_anim_kind_named!(
        registry,
        "animations.window_open.kind",
        animations.window_open
    );
    accessor_anim_kind_named!(
        registry,
        "animations.window_close.kind",
        animations.window_close
    );
    accessor_anim_kind!(
        registry,
        "animations.horizontal_view_movement.kind",
        animations.horizontal_view_movement
    );
    accessor_anim_kind!(
        registry,
        "animations.window_movement.kind",
        animations.window_movement
    );
    accessor_anim_kind_named!(
        registry,
        "animations.window_resize.kind",
        animations.window_resize
    );
    accessor_anim_kind!(
        registry,
        "animations.config_notification_open_close.kind",
        animations.config_notification_open_close
    );
    accessor_anim_kind!(
        registry,
        "animations.screenshot_ui_open.kind",
        animations.screenshot_ui_open
    );
    accessor_anim_kind!(
        registry,
        "animations.exit_confirmation_open_close.kind",
        animations.exit_confirmation_open_close
    );
    accessor_anim_kind!(
        registry,
        "animations.overview_open_close.kind",
        animations.overview_open_close
    );
    accessor_anim_kind!(
        registry,
        "animations.recent_windows_close.kind",
        animations.recent_windows_close
    );

    accessor_bool!(
        registry,
        "input.mouse.natural_scroll",
        input.mouse.natural_scroll
    );

    accessor_float_or_int!(
        registry,
        "input.mouse.accel_speed",
        input.mouse.accel_speed,
        -1,
        1
    );

    accessor_option_enum!(
        registry,
        "input.mouse.accel_profile",
        input.mouse.accel_profile,
        AccelProfile,
        Adaptive => "adaptive",
        Flat => "flat",
    );

    accessor_option_enum_normalize!(
        registry,
        "input.mouse.scroll_method",
        input.mouse.scroll_method,
        ScrollMethod,
        NoScroll => "no_scroll",
        TwoFinger => "two_finger",
        Edge => "edge",
        OnButtonDown => "on_button_down",
    );

    accessor_bool!(registry, "input.touchpad.tap", input.touchpad.tap);
    accessor_bool!(registry, "input.touchpad.dwt", input.touchpad.dwt);
    accessor_bool!(registry, "input.touchpad.dwtp", input.touchpad.dwtp);
    accessor_option_bool!(registry, "input.touchpad.drag", input.touchpad.drag);
    accessor_bool!(
        registry,
        "input.touchpad.drag_lock",
        input.touchpad.drag_lock
    );
    accessor_bool!(
        registry,
        "input.touchpad.natural_scroll",
        input.touchpad.natural_scroll
    );

    accessor_float_or_int!(
        registry,
        "input.touchpad.accel_speed",
        input.touchpad.accel_speed,
        -1,
        1
    );

    accessor_option_enum!(
        registry,
        "input.touchpad.accel_profile",
        input.touchpad.accel_profile,
        AccelProfile,
        Adaptive => "adaptive",
        Flat => "flat",
    );

    accessor_option_enum_normalize!(
        registry,
        "input.touchpad.tap_button_map",
        input.touchpad.tap_button_map,
        TapButtonMap,
        LeftRightMiddle => "left_right_middle",
        LeftMiddleRight => "left_middle_right",
    );

    accessor_option_enum_normalize!(
        registry,
        "input.touchpad.scroll_method",
        input.touchpad.scroll_method,
        ScrollMethod,
        NoScroll => "no_scroll",
        TwoFinger => "two_finger",
        Edge => "edge",
        OnButtonDown => "on_button_down",
    );

    accessor_bool!(registry, "input.touch.off", input.touch.off);
    accessor_bool!(
        registry,
        "input.touch.natural_scroll",
        input.touch.natural_scroll
    );
    accessor_option_string!(
        registry,
        "input.touch.map_to_output",
        input.touch.map_to_output
    );

    registry.update_accessor(
        "input.touch.calibration_matrix",
        |lua, config| match &config.input.touch.calibration_matrix {
            Some(v) => {
                let arr = lua.create_table()?;
                for (i, val) in v.iter().enumerate() {
                    arr.set(i + 1, *val as f64)?;
                }
                Ok(LuaValue::Table(arr))
            }
            None => Ok(LuaValue::Nil),
        },
        |lua, config, value| {
            let v: Option<Vec<f64>> = Option::from_lua(value, lua)?;
            config.input.touch.calibration_matrix =
                v.map(|arr| arr.iter().map(|x| *x as f32).collect());
            Ok(())
        },
    );

    accessor_bool!(
        registry,
        "input.trackpoint.natural_scroll",
        input.trackpoint.natural_scroll
    );

    accessor_float_or_int!(
        registry,
        "input.trackpoint.accel_speed",
        input.trackpoint.accel_speed,
        -1,
        1
    );

    accessor_option_enum!(
        registry,
        "input.trackpoint.accel_profile",
        input.trackpoint.accel_profile,
        AccelProfile,
        Adaptive => "adaptive",
        Flat => "flat",
    );

    accessor_bool!(
        registry,
        "input.trackball.natural_scroll",
        input.trackball.natural_scroll
    );

    accessor_float_or_int!(
        registry,
        "input.trackball.accel_speed",
        input.trackball.accel_speed,
        -1,
        1
    );

    accessor_option_enum!(
        registry,
        "input.trackball.accel_profile",
        input.trackball.accel_profile,
        AccelProfile,
        Adaptive => "adaptive",
        Flat => "flat",
    );

    accessor_bool!(
        registry,
        "clipboard.disable_primary",
        clipboard.disable_primary
    );

    accessor_wrapped_option_string!(registry, "screenshot_path", screenshot_path);

    accessor_bool!(
        registry,
        "hotkey_overlay.skip_at_startup",
        hotkey_overlay.skip_at_startup
    );
    accessor_bool!(
        registry,
        "config_notification.disable_failed",
        config_notification.disable_failed
    );

    accessor_string!(registry, "cursor.xcursor_theme", cursor.xcursor_theme);
    accessor_int!(registry, "cursor.xcursor_size", cursor.xcursor_size, u8);
    accessor_bool!(registry, "cursor.hide_when_typing", cursor.hide_when_typing);
    accessor_option_int!(
        registry,
        "cursor.hide_after_inactive_ms",
        cursor.hide_after_inactive_ms,
        u32
    );

    accessor_option_enum!(
        registry,
        "debug.preview_render",
        debug.preview_render,
        PreviewRender,
        Screencast => "screencast",
        ScreenCapture => "screen_capture",
    );

    accessor_bool!(
        registry,
        "debug.dbus_interfaces_in_non_session_instances",
        debug.dbus_interfaces_in_non_session_instances
    );
    accessor_bool!(
        registry,
        "debug.wait_for_frame_completion_before_queueing",
        debug.wait_for_frame_completion_before_queueing
    );
    accessor_bool!(
        registry,
        "debug.enable_overlay_planes",
        debug.enable_overlay_planes
    );
    accessor_bool!(
        registry,
        "debug.disable_cursor_plane",
        debug.disable_cursor_plane
    );
    accessor_bool!(
        registry,
        "debug.disable_direct_scanout",
        debug.disable_direct_scanout
    );
    accessor_bool!(
        registry,
        "debug.keep_max_bpc_unchanged",
        debug.keep_max_bpc_unchanged
    );

    accessor_bool!(
        registry,
        "debug.restrict_primary_scanout_to_matching_format",
        debug.restrict_primary_scanout_to_matching_format
    );
    accessor_bool!(
        registry,
        "debug.force_disable_connectors_on_resume",
        debug.force_disable_connectors_on_resume
    );

    accessor_option_path!(registry, "debug.render_drm_device", debug.render_drm_device);

    registry.update_accessor(
        "debug.ignored_drm_devices",
        |_lua, config| {
            let tbl = _lua.create_table()?;
            for (i, dev) in config.debug.ignored_drm_devices.iter().enumerate() {
                tbl.set((i + 1) as i64, dev.to_string_lossy().to_string())?;
            }
            Ok(LuaValue::Table(tbl))
        },
        |_lua, config, value| {
            let devices = match value {
                LuaValue::Nil => Vec::new(),
                other => Vec::<String>::from_lua(other, _lua)?,
            };
            config.debug.ignored_drm_devices = devices.into_iter().map(PathBuf::from).collect();
            Ok(())
        },
    );

    accessor_bool!(
        registry,
        "debug.force_pipewire_invalid_modifier",
        debug.force_pipewire_invalid_modifier
    );
    accessor_bool!(
        registry,
        "debug.emulate_zero_presentation_time",
        debug.emulate_zero_presentation_time
    );
    accessor_bool!(
        registry,
        "debug.disable_resize_throttling",
        debug.disable_resize_throttling
    );
    accessor_bool!(
        registry,
        "debug.disable_transactions",
        debug.disable_transactions
    );
    accessor_bool!(
        registry,
        "debug.keep_laptop_panel_on_when_lid_is_closed",
        debug.keep_laptop_panel_on_when_lid_is_closed
    );
    accessor_bool!(
        registry,
        "debug.disable_monitor_names",
        debug.disable_monitor_names
    );

    accessor_bool!(
        registry,
        "debug.strict_new_window_focus_policy",
        debug.strict_new_window_focus_policy
    );
    accessor_bool!(
        registry,
        "debug.honor_xdg_activation_with_invalid_serial",
        debug.honor_xdg_activation_with_invalid_serial
    );
    accessor_bool!(
        registry,
        "debug.deactivate_unfocused_windows",
        debug.deactivate_unfocused_windows
    );
    accessor_bool!(
        registry,
        "debug.skip_cursor_only_updates_during_vrr",
        debug.skip_cursor_only_updates_during_vrr
    );

    accessor_float!(registry, "layout.gaps", layout.gaps);

    accessor_enum!(
        registry,
        "layout.center_focused_column",
        layout.center_focused_column,
        CenterFocusedColumn,
        Never => "never",
        Always => "always",
        OnOverflow => "on_overflow",
    );

    accessor_float_or_int!(
        registry,
        "layout.struts.left",
        layout.struts.left,
        -65535,
        65535
    );
    accessor_float_or_int!(
        registry,
        "layout.struts.right",
        layout.struts.right,
        -65535,
        65535
    );
    accessor_float_or_int!(
        registry,
        "layout.struts.top",
        layout.struts.top,
        -65535,
        65535
    );
    accessor_float_or_int!(
        registry,
        "layout.struts.bottom",
        layout.struts.bottom,
        -65535,
        65535
    );

    accessor_bool!(registry, "layout.shadow.on", layout.shadow.on);

    accessor_inverted_bool!(registry, "layout.shadow.off", layout.shadow.on);

    accessor_float!(registry, "layout.shadow.softness", layout.shadow.softness);
    accessor_float!(registry, "layout.shadow.spread", layout.shadow.spread);
    accessor_bool!(
        registry,
        "layout.shadow.draw_behind_window",
        layout.shadow.draw_behind_window
    );

    accessor_color!(registry, "layout.shadow.color", layout.shadow.color);
    accessor_option_color!(
        registry,
        "layout.shadow.inactive_color",
        layout.shadow.inactive_color
    );

    registry.update_accessor(
        "layout.shadow.offset.x",
        |_lua, config| Ok(LuaValue::Number(config.layout.shadow.offset.x.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.shadow.offset.x = niri_config::utils::FloatOrInt(v);
            Ok(())
        },
    );
    registry.update_accessor(
        "layout.shadow.offset.y",
        |_lua, config| Ok(LuaValue::Number(config.layout.shadow.offset.y.0)),
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.layout.shadow.offset.y = niri_config::utils::FloatOrInt(v);
            Ok(())
        },
    );

    accessor_bool!(registry, "layout.focus_ring.off", layout.focus_ring.off);
    accessor_float!(registry, "layout.focus_ring.width", layout.focus_ring.width);
    accessor_color!(
        registry,
        "layout.focus_ring.active_color",
        layout.focus_ring.active_color
    );
    accessor_color!(
        registry,
        "layout.focus_ring.inactive_color",
        layout.focus_ring.inactive_color
    );
    accessor_color!(
        registry,
        "layout.focus_ring.urgent_color",
        layout.focus_ring.urgent_color
    );
    accessor_option_gradient!(
        registry,
        "layout.focus_ring.active_gradient",
        layout.focus_ring.active_gradient
    );
    accessor_option_gradient!(
        registry,
        "layout.focus_ring.inactive_gradient",
        layout.focus_ring.inactive_gradient
    );
    accessor_option_gradient!(
        registry,
        "layout.focus_ring.urgent_gradient",
        layout.focus_ring.urgent_gradient
    );

    accessor_bool!(registry, "layout.border.off", layout.border.off);
    accessor_float!(registry, "layout.border.width", layout.border.width);
    accessor_color!(
        registry,
        "layout.border.active_color",
        layout.border.active_color
    );
    accessor_color!(
        registry,
        "layout.border.inactive_color",
        layout.border.inactive_color
    );
    accessor_color!(
        registry,
        "layout.border.urgent_color",
        layout.border.urgent_color
    );
    accessor_option_gradient!(
        registry,
        "layout.border.active_gradient",
        layout.border.active_gradient
    );
    accessor_option_gradient!(
        registry,
        "layout.border.inactive_gradient",
        layout.border.inactive_gradient
    );
    accessor_option_gradient!(
        registry,
        "layout.border.urgent_gradient",
        layout.border.urgent_gradient
    );

    // Additional layout accessors
    accessor_bool!(
        registry,
        "layout.always_center_single_column",
        layout.always_center_single_column
    );
    accessor_bool!(
        registry,
        "layout.empty_workspace_above_first",
        layout.empty_workspace_above_first
    );
    accessor_color!(registry, "layout.background_color", layout.background_color);

    // Tab indicator accessors
    accessor_bool!(
        registry,
        "layout.tab_indicator.off",
        layout.tab_indicator.off
    );
    accessor_bool!(
        registry,
        "layout.tab_indicator.hide_when_single_tab",
        layout.tab_indicator.hide_when_single_tab
    );
    accessor_bool!(
        registry,
        "layout.tab_indicator.place_within_column",
        layout.tab_indicator.place_within_column
    );
    accessor_float!(
        registry,
        "layout.tab_indicator.gap",
        layout.tab_indicator.gap
    );
    accessor_float!(
        registry,
        "layout.tab_indicator.width",
        layout.tab_indicator.width
    );
    accessor_float!(
        registry,
        "layout.tab_indicator.gaps_between_tabs",
        layout.tab_indicator.gaps_between_tabs
    );
    accessor_float!(
        registry,
        "layout.tab_indicator.corner_radius",
        layout.tab_indicator.corner_radius
    );
    accessor_enum!(
        registry,
        "layout.tab_indicator.position",
        layout.tab_indicator.position,
        TabIndicatorPosition,
        Left => "left",
        Right => "right",
        Top => "top",
        Bottom => "bottom",
    );
    accessor_option_color!(
        registry,
        "layout.tab_indicator.active_color",
        layout.tab_indicator.active_color
    );
    accessor_option_color!(
        registry,
        "layout.tab_indicator.inactive_color",
        layout.tab_indicator.inactive_color
    );
    accessor_option_color!(
        registry,
        "layout.tab_indicator.urgent_color",
        layout.tab_indicator.urgent_color
    );
    accessor_option_gradient!(
        registry,
        "layout.tab_indicator.active_gradient",
        layout.tab_indicator.active_gradient
    );
    accessor_option_gradient!(
        registry,
        "layout.tab_indicator.inactive_gradient",
        layout.tab_indicator.inactive_gradient
    );
    accessor_option_gradient!(
        registry,
        "layout.tab_indicator.urgent_gradient",
        layout.tab_indicator.urgent_gradient
    );

    // Insert hint accessors
    accessor_bool!(registry, "layout.insert_hint.off", layout.insert_hint.off);
    accessor_color!(
        registry,
        "layout.insert_hint.color",
        layout.insert_hint.color
    );
    accessor_option_gradient!(
        registry,
        "layout.insert_hint.gradient",
        layout.insert_hint.gradient
    );

    // Gestures accessors
    accessor_float!(
        registry,
        "gestures.dnd_edge_view_scroll.trigger_width",
        gestures.dnd_edge_view_scroll.trigger_width
    );
    accessor_int!(
        registry,
        "gestures.dnd_edge_view_scroll.delay_ms",
        gestures.dnd_edge_view_scroll.delay_ms,
        u16
    );
    accessor_float!(
        registry,
        "gestures.dnd_edge_view_scroll.max_speed",
        gestures.dnd_edge_view_scroll.max_speed
    );
    accessor_float!(
        registry,
        "gestures.dnd_edge_workspace_switch.trigger_height",
        gestures.dnd_edge_workspace_switch.trigger_height
    );
    accessor_int!(
        registry,
        "gestures.dnd_edge_workspace_switch.delay_ms",
        gestures.dnd_edge_workspace_switch.delay_ms,
        u16
    );
    accessor_float!(
        registry,
        "gestures.dnd_edge_workspace_switch.max_speed",
        gestures.dnd_edge_workspace_switch.max_speed
    );
    accessor_bool!(
        registry,
        "gestures.hot_corners.off",
        gestures.hot_corners.off
    );
    accessor_bool!(
        registry,
        "gestures.hot_corners.top_left",
        gestures.hot_corners.top_left
    );
    accessor_bool!(
        registry,
        "gestures.hot_corners.top_right",
        gestures.hot_corners.top_right
    );
    accessor_bool!(
        registry,
        "gestures.hot_corners.bottom_left",
        gestures.hot_corners.bottom_left
    );
    accessor_bool!(
        registry,
        "gestures.hot_corners.bottom_right",
        gestures.hot_corners.bottom_right
    );

    // Overview accessors
    accessor_float!(registry, "overview.zoom", overview.zoom);
    accessor_color!(registry, "overview.backdrop_color", overview.backdrop_color);
    accessor_bool!(
        registry,
        "overview.workspace_shadow.off",
        overview.workspace_shadow.off
    );
    accessor_float!(
        registry,
        "overview.workspace_shadow.softness",
        overview.workspace_shadow.softness
    );
    accessor_float!(
        registry,
        "overview.workspace_shadow.spread",
        overview.workspace_shadow.spread
    );
    accessor_color!(
        registry,
        "overview.workspace_shadow.color",
        overview.workspace_shadow.color
    );
    registry.update_accessor(
        "overview.workspace_shadow.offset.x",
        |_lua, config| {
            Ok(LuaValue::Number(
                config.overview.workspace_shadow.offset.x.0,
            ))
        },
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.overview.workspace_shadow.offset.x = niri_config::utils::FloatOrInt(v);
            Ok(())
        },
    );
    registry.update_accessor(
        "overview.workspace_shadow.offset.y",
        |_lua, config| {
            Ok(LuaValue::Number(
                config.overview.workspace_shadow.offset.y.0,
            ))
        },
        |_lua, config, value| {
            let v = f64::from_lua(value, _lua)?;
            config.overview.workspace_shadow.offset.y = niri_config::utils::FloatOrInt(v);
            Ok(())
        },
    );

    // Hotkey overlay accessor
    accessor_bool!(
        registry,
        "hotkey_overlay.hide_not_bound",
        hotkey_overlay.hide_not_bound
    );

    // Column width and height preset accessors
    registry.update_accessor(
        "layout.default_column_width",
        |lua, config| match config.layout.default_column_width.as_ref() {
            Some(size) => {
                let tbl: PresetSizeTable = LuaFieldConvert::to_lua(size);
                tbl.into_lua(lua)
            }
            None => Ok(mlua::Nil),
        },
        |lua, config, value| {
            if value.is_nil() {
                config.layout.default_column_width = None;
            } else {
                let pst = PresetSizeTable::from_lua(value, lua)?;
                let size = PresetSize::from_lua_field(pst)?;
                config.layout.default_column_width = Some(size);
            }
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.preset_column_widths",
        |lua, config| preset_sizes_to_lua(lua, &config.layout.preset_column_widths),
        |_lua, config, value| {
            config.layout.preset_column_widths = preset_sizes_from_lua(value)?;
            Ok(())
        },
    );

    registry.update_accessor(
        "layout.preset_window_heights",
        |lua, config| preset_sizes_to_lua(lua, &config.layout.preset_window_heights),
        |_lua, config, value| {
            config.layout.preset_window_heights = preset_sizes_from_lua(value)?;
            Ok(())
        },
    );

    accessor_enum!(
        registry,
        "layout.default_column_display",
        layout.default_column_display,
        niri_ipc::ColumnDisplay,
        Normal => "normal",
        Tabbed => "tabbed",
    );
}
