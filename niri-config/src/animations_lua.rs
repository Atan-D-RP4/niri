use mlua::prelude::*;
use niri_lua_traits::{
    extract_bool_opt, extract_float_opt, extract_int_opt, extract_string_opt, extract_table_opt,
    FromLuaTable,
};

use super::animations::{
    Animation, Animations, ConfigNotificationOpenCloseAnim, Curve, EasingParams,
    ExitConfirmationOpenCloseAnim, HorizontalViewMovementAnim, Kind, OverviewOpenCloseAnim,
    RecentWindowsCloseAnim, ScreenshotUiOpenAnim, SpringParams, WindowCloseAnim,
    WindowMovementAnim, WindowOpenAnim, WindowResizeAnim, WorkspaceSwitchAnim,
};

impl FromLuaTable for Animations {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let mut result = Self::default();
        let mut has_any = false;

        if let Some(v) = extract_bool_opt(table, "off")? {
            result.off = v;
            has_any = true;
        }
        if let Some(v) = extract_float_opt(table, "slowdown")? {
            result.slowdown = v;
            has_any = true;
        }
        if let Some(t) = extract_table_opt(table, "workspace-switch")? {
            if let Some(v) = WorkspaceSwitchAnim::from_lua_table(&t)? {
                result.workspace_switch = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "window-open")? {
            if let Some(v) = WindowOpenAnim::from_lua_table(&t)? {
                result.window_open = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "window-close")? {
            if let Some(v) = WindowCloseAnim::from_lua_table(&t)? {
                result.window_close = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "horizontal-view-movement")? {
            if let Some(v) = HorizontalViewMovementAnim::from_lua_table(&t)? {
                result.horizontal_view_movement = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "window-movement")? {
            if let Some(v) = WindowMovementAnim::from_lua_table(&t)? {
                result.window_movement = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "window-resize")? {
            if let Some(v) = WindowResizeAnim::from_lua_table(&t)? {
                result.window_resize = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "config-notification-open-close")? {
            if let Some(v) = ConfigNotificationOpenCloseAnim::from_lua_table(&t)? {
                result.config_notification_open_close = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "exit-confirmation-open-close")? {
            if let Some(v) = ExitConfirmationOpenCloseAnim::from_lua_table(&t)? {
                result.exit_confirmation_open_close = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "screenshot-ui-open")? {
            if let Some(v) = ScreenshotUiOpenAnim::from_lua_table(&t)? {
                result.screenshot_ui_open = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "overview-open-close")? {
            if let Some(v) = OverviewOpenCloseAnim::from_lua_table(&t)? {
                result.overview_open_close = v;
                has_any = true;
            }
        }
        if let Some(t) = extract_table_opt(table, "recent-windows-close")? {
            if let Some(v) = RecentWindowsCloseAnim::from_lua_table(&t)? {
                result.recent_windows_close = v;
                has_any = true;
            }
        }

        if has_any {
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

fn default_easing_params() -> EasingParams {
    EasingParams {
        duration_ms: 250,
        curve: Curve::EaseOutCubic,
    }
}

fn default_spring_params() -> SpringParams {
    SpringParams {
        damping_ratio: 1.0,
        stiffness: 800,
        epsilon: 0.0001,
    }
}

fn parse_curve_value(value: LuaValue) -> LuaResult<Option<Curve>> {
    match value {
        LuaValue::String(s) => {
            let curve_str = s.to_string_lossy().to_lowercase();
            let curve = match curve_str.as_str() {
                "linear" => Some(Curve::Linear),
                "ease-out-quad" | "ease_out_quad" => Some(Curve::EaseOutQuad),
                "ease-out-cubic" | "ease_out_cubic" => Some(Curve::EaseOutCubic),
                "ease-out-expo" | "ease_out_expo" => Some(Curve::EaseOutExpo),
                _ => None,
            };
            Ok(curve)
        }
        LuaValue::Table(t) => {
            let get_coord = |key: &str| -> LuaResult<Option<f64>> {
                match t.get::<LuaValue>(key) {
                    Ok(LuaValue::Number(n)) => Ok(Some(n)),
                    Ok(LuaValue::Integer(i)) => Ok(Some(i as f64)),
                    _ => Ok(None),
                }
            };

            let coords = if let Some(x1) = get_coord("x1")? {
                let y1 = get_coord("y1")?.unwrap_or(0.0);
                let x2 = get_coord("x2")?.unwrap_or(0.0);
                let y2 = get_coord("y2")?.unwrap_or(0.0);
                Some((x1, y1, x2, y2))
            } else if t.len()? >= 4 {
                Some((
                    t.get::<f64>(1)?,
                    t.get::<f64>(2)?,
                    t.get::<f64>(3)?,
                    t.get::<f64>(4)?,
                ))
            } else {
                None
            };

            Ok(coords.map(|(x1, y1, x2, y2)| Curve::CubicBezier(x1, y1, x2, y2)))
        }
        _ => Ok(None),
    }
}

impl FromLuaTable for SpringParams {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let damping_ratio = extract_float_opt(table, "damping_ratio")?;
        let stiffness = extract_int_opt(table, "stiffness")?.map(|v| v as u32);
        let epsilon = extract_float_opt(table, "epsilon")?;

        if damping_ratio.is_none() && stiffness.is_none() && epsilon.is_none() {
            return Ok(None);
        }

        let mut params = default_spring_params();
        if let Some(d) = damping_ratio {
            params.damping_ratio = d;
        }
        if let Some(s) = stiffness {
            params.stiffness = s;
        }
        if let Some(e) = epsilon {
            params.epsilon = e;
        }

        if !(0.1..=10.0).contains(&params.damping_ratio) {
            return Err(LuaError::external(
                "damping_ratio must be between 0.1 and 10.0",
            ));
        }
        if params.stiffness < 1 {
            return Err(LuaError::external("stiffness must be >= 1"));
        }
        if !(0.00001..=0.1).contains(&params.epsilon) {
            return Err(LuaError::external(
                "epsilon must be between 0.00001 and 0.1",
            ));
        }

        Ok(Some(params))
    }
}

impl FromLuaTable for EasingParams {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let duration_ms = extract_int_opt(table, "duration_ms")?.map(|v| v as u32);
        let curve = match table.get::<LuaValue>("curve") {
            Ok(v) => parse_curve_value(v)?,
            Err(_) => None,
        };

        if duration_ms.is_none() && curve.is_none() {
            return Ok(None);
        }

        let mut params = default_easing_params();
        if let Some(d) = duration_ms {
            params.duration_ms = d;
        }
        if let Some(c) = curve {
            params.curve = c;
        }

        Ok(Some(params))
    }
}

impl FromLuaTable for Kind {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let spring = if let Some(spring_table) = extract_table_opt(table, "spring")? {
            SpringParams::from_lua_table(&spring_table)?
        } else {
            None
        };
        let easing = if let Some(easing_table) = extract_table_opt(table, "easing")? {
            EasingParams::from_lua_table(&easing_table)?
        } else {
            None
        };

        if spring.is_some() && easing.is_some() {
            return Err(LuaError::external(
                "cannot set both spring and easing parameters at once",
            ));
        }

        if let Some(s) = spring {
            return Ok(Some(Kind::Spring(s)));
        }
        if let Some(e) = easing {
            return Ok(Some(Kind::Easing(e)));
        }

        Ok(None)
    }
}

fn apply_animation_overrides(
    table: &LuaTable,
    mut base: Animation,
) -> LuaResult<Option<Animation>> {
    let off = extract_bool_opt(table, "off")?;

    let kind = if let Some(kind_table) = extract_table_opt(table, "kind")? {
        Kind::from_lua_table(&kind_table)?
    } else {
        Kind::from_lua_table(table)?
    };

    if off.is_none() && kind.is_none() {
        return Ok(None);
    }

    if let Some(v) = off {
        base.off = v;
    }
    if let Some(k) = kind {
        base.kind = k;
    }

    Ok(Some(base))
}

impl FromLuaTable for Animation {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(
            table,
            Animation {
                off: false,
                kind: Kind::Easing(default_easing_params()),
            },
        )
    }
}

impl FromLuaTable for WorkspaceSwitchAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for HorizontalViewMovementAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for WindowMovementAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for ConfigNotificationOpenCloseAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for ExitConfirmationOpenCloseAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for ScreenshotUiOpenAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for OverviewOpenCloseAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for RecentWindowsCloseAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        apply_animation_overrides(table, Self::default().0).map(|opt| opt.map(Self))
    }
}

impl FromLuaTable for WindowOpenAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let mut has_any = false;
        let custom_shader = extract_string_opt(table, "custom_shader")?;
        if custom_shader.is_some() {
            has_any = true;
        }

        if let Some(anim) = apply_animation_overrides(table, Self::default().anim)? {
            Ok(Some(Self {
                anim,
                custom_shader,
            }))
        } else if has_any {
            Ok(Some(Self {
                anim: Self::default().anim,
                custom_shader,
            }))
        } else {
            Ok(None)
        }
    }
}

impl FromLuaTable for WindowCloseAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let mut has_any = false;
        let custom_shader = extract_string_opt(table, "custom_shader")?;
        if custom_shader.is_some() {
            has_any = true;
        }

        if let Some(anim) = apply_animation_overrides(table, Self::default().anim)? {
            Ok(Some(Self {
                anim,
                custom_shader,
            }))
        } else if has_any {
            Ok(Some(Self {
                anim: Self::default().anim,
                custom_shader,
            }))
        } else {
            Ok(None)
        }
    }
}

impl FromLuaTable for WindowResizeAnim {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let mut has_any = false;
        let custom_shader = extract_string_opt(table, "custom_shader")?;
        if custom_shader.is_some() {
            has_any = true;
        }

        if let Some(anim) = apply_animation_overrides(table, Self::default().anim)? {
            Ok(Some(Self {
                anim,
                custom_shader,
            }))
        } else if has_any {
            Ok(Some(Self {
                anim: Self::default().anim,
                custom_shader,
            }))
        } else {
            Ok(None)
        }
    }
}
