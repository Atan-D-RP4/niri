//! Traits for Lua configuration type conversion.
//!
//! These traits provide bidirectional conversion between Rust configuration types
//! and their Lua representations, used by the derive macros to generate proxy methods.

use std::time::Duration;

use mlua::{Error as LuaError, FromLua, IntoLua, Lua, Result as LuaResult, Value};

/// Trait for converting configuration field types to/from Lua.
///
/// This trait is automatically implemented for primitive types and can be
/// derived for enums using `#[derive(LuaEnum)]`.
pub trait LuaFieldConvert: Sized {
    /// The intermediate Lua type (before final conversion to/from Value)
    type LuaType: for<'lua> IntoLua + for<'lua> FromLua;

    /// Convert from Rust to the Lua representation
    fn to_lua(&self) -> Self::LuaType;

    /// Convert from Lua representation to Rust
    fn from_lua_field(value: Self::LuaType) -> LuaResult<Self>;
}

/// Trait for enums that convert to/from Lua strings.
///
/// This is automatically implemented by `#[derive(LuaEnum)]`.
pub trait LuaEnumConvert: Sized {
    /// Convert to a Lua string representation
    fn to_lua_string(&self) -> &'static str;

    /// Convert from a Lua string representation
    fn from_lua_string(s: &str) -> LuaResult<Self>;

    /// Get all valid variant names for error messages
    fn variant_names() -> &'static [&'static str];
}

// ============================================================================
// Primitive Implementations
// ============================================================================

/// Macro to implement LuaFieldConvert for Copy types where LuaType = Self.
macro_rules! impl_lua_field_convert_copy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl LuaFieldConvert for $ty {
                type LuaType = $ty;

                fn to_lua(&self) -> $ty {
                    *self
                }

                fn from_lua_field(value: $ty) -> LuaResult<Self> {
                    Ok(value)
                }
            }
        )+
    };
}

impl_lua_field_convert_copy!(bool, i32, i64, u8, u16, u32, u64, f64);

impl LuaFieldConvert for String {
    type LuaType = String;

    fn to_lua(&self) -> String {
        self.clone()
    }

    fn from_lua_field(value: String) -> LuaResult<Self> {
        Ok(value)
    }
}

// ============================================================================
// Option Implementation
// ============================================================================

// Note: Option<T> cannot implement LuaFieldConvert directly because Lua's nil
// doesn't work well with the trait's associated type pattern. Instead, the
// derive macro generates special handling for Option fields.

// ============================================================================
// Duration Implementation
// ============================================================================

impl LuaFieldConvert for Duration {
    /// Duration is represented as milliseconds in Lua
    type LuaType = u64;

    fn to_lua(&self) -> u64 {
        self.as_millis() as u64
    }

    fn from_lua_field(value: u64) -> LuaResult<Self> {
        Ok(Duration::from_millis(value))
    }
}

// ============================================================================
// Color Implementation
// ============================================================================

use niri_config::Color;

impl LuaFieldConvert for Color {
    /// Color is represented as a hex string "#RRGGBBAA" in Lua
    type LuaType = String;

    fn to_lua(&self) -> String {
        // Color stores f32 values in 0-1 range, convert to 0-255 u8
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        let a = (self.a * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
    }

    fn from_lua_field(value: String) -> LuaResult<Self> {
        parse_color_string(&value)
    }
}

/// Parse a color string in various formats.
///
/// Supported formats:
/// - `#RGB` - 4-bit per channel
/// - `#RGBA` - 4-bit per channel with alpha
/// - `#RRGGBB` - 8-bit per channel
/// - `#RRGGBBAA` - 8-bit per channel with alpha
pub fn parse_color_string(s: &str) -> LuaResult<Color> {
    let s = s.trim();

    if !s.starts_with('#') {
        return Err(LuaError::external(format!(
            "Color must start with '#', got: {}",
            s
        )));
    }

    let hex = &s[1..];

    match hex.len() {
        3 => {
            // #RGB
            let r = parse_hex_digit(hex.chars().next().unwrap())?;
            let g = parse_hex_digit(hex.chars().nth(1).unwrap())?;
            let b = parse_hex_digit(hex.chars().nth(2).unwrap())?;
            // Convert 4-bit to 8-bit by repeating the nibble (0xF -> 0xFF)
            let r = r * 17;
            let g = g * 17;
            let b = b * 17;
            Ok(Color::from_rgba8_unpremul(r, g, b, 255))
        }
        4 => {
            // #RGBA
            let r = parse_hex_digit(hex.chars().next().unwrap())?;
            let g = parse_hex_digit(hex.chars().nth(1).unwrap())?;
            let b = parse_hex_digit(hex.chars().nth(2).unwrap())?;
            let a = parse_hex_digit(hex.chars().nth(3).unwrap())?;
            let r = r * 17;
            let g = g * 17;
            let b = b * 17;
            let a = a * 17;
            Ok(Color::from_rgba8_unpremul(r, g, b, a))
        }
        6 => {
            // #RRGGBB
            let r = parse_hex_byte(&hex[0..2])?;
            let g = parse_hex_byte(&hex[2..4])?;
            let b = parse_hex_byte(&hex[4..6])?;
            Ok(Color::from_rgba8_unpremul(r, g, b, 255))
        }
        8 => {
            // #RRGGBBAA
            let r = parse_hex_byte(&hex[0..2])?;
            let g = parse_hex_byte(&hex[2..4])?;
            let b = parse_hex_byte(&hex[4..6])?;
            let a = parse_hex_byte(&hex[6..8])?;
            Ok(Color::from_rgba8_unpremul(r, g, b, a))
        }
        _ => Err(LuaError::external(format!(
            "Invalid color format. Expected #RGB, #RGBA, #RRGGBB, or #RRGGBBAA, got: {}",
            s
        ))),
    }
}

fn parse_hex_digit(c: char) -> LuaResult<u8> {
    c.to_digit(16)
        .map(|d| d as u8)
        .ok_or_else(|| LuaError::external(format!("Invalid hex digit: {}", c)))
}

fn parse_hex_byte(s: &str) -> LuaResult<u8> {
    u8::from_str_radix(s, 16)
        .map_err(|e| LuaError::external(format!("Invalid hex byte '{}': {}", s, e)))
}

// ============================================================================
// Gradient Implementation
// ============================================================================

use niri_config::Gradient;

impl LuaFieldConvert for Gradient {
    /// Gradient is represented as a table { from = "#color", to = "#color", angle = degrees,
    /// relative_to = "string" }
    type LuaType = GradientTable;

    fn to_lua(&self) -> GradientTable {
        GradientTable {
            from: Color::to_lua(&self.from),
            to: Color::to_lua(&self.to),
            angle: self.angle,
            relative_to: Some(gradient_relative_to_string(self.relative_to)),
        }
    }

    fn from_lua_field(value: GradientTable) -> LuaResult<Self> {
        use niri_config::GradientInterpolation;

        let from = parse_color_string(&value.from)?;
        let to = parse_color_string(&value.to)?;

        // Parse relative_to if provided
        let relative_to = if let Some(ref rt_str) = value.relative_to {
            parse_gradient_relative_to(rt_str)?
        } else {
            // Default to Window
            niri_config::GradientRelativeTo::Window
        };

        Ok(Gradient {
            from,
            to,
            angle: value.angle,
            relative_to,
            in_: GradientInterpolation::default(),
        })
    }
}

/// Intermediate struct for gradient Lua representation
#[derive(Clone)]
pub struct GradientTable {
    pub from: String,
    pub to: String,
    pub angle: i16,
    pub relative_to: Option<String>,
}

impl IntoLua for GradientTable {
    fn into_lua(self, lua: &Lua) -> LuaResult<Value> {
        let table = lua.create_table()?;
        table.set("from", self.from)?;
        table.set("to", self.to)?;
        table.set("angle", self.angle)?;
        if let Some(rt) = self.relative_to {
            table.set("relative_to", rt)?;
        }
        Ok(Value::Table(table))
    }
}

impl FromLua for GradientTable {
    fn from_lua(value: Value, _lua: &Lua) -> LuaResult<Self> {
        match value {
            Value::Table(table) => {
                let from: String = table.get("from")?;
                let to: String = table.get("to")?;
                // Get angle as Option, default to 180
                let angle: i16 = (table.get::<Option<i16>>("angle")?).unwrap_or(180);
                let relative_to: Option<String> = table.get("relative_to")?;

                Ok(GradientTable {
                    from,
                    to,
                    angle,
                    relative_to,
                })
            }
            _ => Err(LuaError::external(
                "Expected a table for gradient with 'from' and 'to' color fields",
            )),
        }
    }
}

use niri_config::GradientRelativeTo;

fn parse_gradient_relative_to(s: &str) -> LuaResult<GradientRelativeTo> {
    match s.to_lowercase().as_str() {
        "window" => Ok(GradientRelativeTo::Window),
        "workspace-view" | "workspace_view" => Ok(GradientRelativeTo::WorkspaceView),
        _ => Err(LuaError::external(format!(
            "Invalid gradient relative_to: '{}'. Expected 'window' or 'workspace-view'",
            s
        ))),
    }
}

fn gradient_relative_to_string(rt: GradientRelativeTo) -> String {
    match rt {
        GradientRelativeTo::Window => "window".to_string(),
        GradientRelativeTo::WorkspaceView => "workspace-view".to_string(),
    }
}

// ============================================================================
// FloatOrInt Implementation
// ============================================================================

use niri_config::FloatOrInt;

impl<const MIN: i32, const MAX: i32> LuaFieldConvert for FloatOrInt<MIN, MAX> {
    /// FloatOrInt is represented as a number in Lua
    type LuaType = f64;

    fn to_lua(&self) -> f64 {
        self.0
    }

    fn from_lua_field(value: f64) -> LuaResult<Self> {
        Ok(FloatOrInt(value))
    }
}

// ============================================================================
// Input Device Enum Implementations
// ============================================================================

use niri_config::input::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap, TrackLayout};
use niri_config::layout::CenterFocusedColumn;
use niri_config::TabIndicatorPosition;
use niri_ipc::ColumnDisplay;

macro_rules! impl_lua_enum {
    ($ty:ty, $name:expr, [$(($variant:ident, $str:expr)),+ $(,)?]) => {
        impl LuaFieldConvert for $ty {
            type LuaType = String;

            fn to_lua(&self) -> String {
                match self {
                    $(Self::$variant => $str,)+
                }
                .to_string()
            }

            fn from_lua_field(value: String) -> LuaResult<Self> {
                match value.as_str() {
                    $($str => Ok(Self::$variant),)+
                    _ => Err(LuaError::external(format!(
                        concat!("Invalid ", $name, " '{}'. Expected: {}"),
                        value,
                        [$($str),+].join(", ")
                    ))),
                }
            }
        }
    };
}

impl_lua_enum!(
    AccelProfile,
    "accel_profile",
    [(Adaptive, "adaptive"), (Flat, "flat"),]
);

impl_lua_enum!(
    ClickMethod,
    "click_method",
    [(ButtonAreas, "button-areas"), (Clickfinger, "clickfinger"),]
);

impl_lua_enum!(
    ScrollMethod,
    "scroll_method",
    [
        (NoScroll, "no-scroll"),
        (TwoFinger, "two-finger"),
        (Edge, "edge"),
        (OnButtonDown, "on-button-down"),
    ]
);

impl_lua_enum!(
    TapButtonMap,
    "tap_button_map",
    [
        (LeftRightMiddle, "left-right-middle"),
        (LeftMiddleRight, "left-middle-right"),
    ]
);

impl_lua_enum!(
    TrackLayout,
    "track_layout",
    [(Global, "global"), (Window, "window"),]
);

impl_lua_enum!(
    TabIndicatorPosition,
    "position",
    [
        (Left, "left"),
        (Right, "right"),
        (Top, "top"),
        (Bottom, "bottom"),
    ]
);

impl_lua_enum!(
    CenterFocusedColumn,
    "center_focused_column",
    [
        (Never, "never"),
        (Always, "always"),
        (OnOverflow, "on-overflow"),
    ]
);

impl_lua_enum!(
    ColumnDisplay,
    "default_column_display",
    [(Normal, "normal"), (Tabbed, "tabbed"),]
);

// ============================================================================
// PresetSize Implementation
// ============================================================================

use niri_config::PresetSize;

/// Lua table representation of PresetSize: { proportion = 0.5 } or { fixed = 1920 }
#[derive(Debug, Clone)]
pub struct PresetSizeTable {
    pub proportion: Option<f64>,
    pub fixed: Option<i32>,
}

impl IntoLua for PresetSizeTable {
    fn into_lua(self, lua: &Lua) -> LuaResult<Value> {
        let tbl = lua.create_table()?;
        if let Some(p) = self.proportion {
            tbl.set("proportion", p)?;
        }
        if let Some(f) = self.fixed {
            tbl.set("fixed", f)?;
        }
        Ok(Value::Table(tbl))
    }
}

impl FromLua for PresetSizeTable {
    fn from_lua(value: Value, _lua: &Lua) -> LuaResult<Self> {
        match value {
            Value::Table(tbl) => Ok(PresetSizeTable {
                proportion: tbl.get("proportion").ok(),
                fixed: tbl.get("fixed").ok(),
            }),
            Value::Number(n) => Ok(PresetSizeTable {
                proportion: Some(n),
                fixed: None,
            }),
            Value::Integer(n) => Ok(PresetSizeTable {
                proportion: None,
                fixed: Some(n as i32),
            }),
            _ => Err(LuaError::external(
                "PresetSize must be a table { proportion = <f64> } or { fixed = <i32> }, or a number",
            )),
        }
    }
}

impl LuaFieldConvert for PresetSize {
    type LuaType = PresetSizeTable;

    fn to_lua(&self) -> PresetSizeTable {
        match self {
            PresetSize::Proportion(p) => PresetSizeTable {
                proportion: Some(*p),
                fixed: None,
            },
            PresetSize::Fixed(f) => PresetSizeTable {
                proportion: None,
                fixed: Some(*f),
            },
        }
    }

    fn from_lua_field(value: PresetSizeTable) -> LuaResult<Self> {
        if let Some(p) = value.proportion {
            Ok(PresetSize::Proportion(p))
        } else if let Some(f) = value.fixed {
            Ok(PresetSize::Fixed(f))
        } else {
            Err(LuaError::external(
                "PresetSize must have either 'proportion' or 'fixed' field",
            ))
        }
    }
}

pub fn preset_sizes_to_lua(lua: &Lua, sizes: &[PresetSize]) -> LuaResult<Value> {
    let arr = lua.create_table()?;
    for (i, size) in sizes.iter().enumerate() {
        let tbl: PresetSizeTable = size.to_lua();
        arr.set(i + 1, tbl)?;
    }
    Ok(Value::Table(arr))
}

pub fn preset_sizes_from_lua(value: Value) -> LuaResult<Vec<PresetSize>> {
    match value {
        Value::Table(tbl) => {
            let mut sizes = Vec::new();
            for pair in tbl.pairs::<i64, PresetSizeTable>() {
                let (_, pst) = pair?;
                sizes.push(PresetSize::from_lua_field(pst)?);
            }
            Ok(sizes)
        }
        Value::Nil => Ok(Vec::new()),
        _ => Err(LuaError::external(
            "preset sizes must be an array of { proportion = <f64> } or { fixed = <i32> }",
        )),
    }
}

// ============================================================================
// SpawnAtStartup Implementation
// ============================================================================

use niri_config::SpawnAtStartup;

impl LuaFieldConvert for SpawnAtStartup {
    type LuaType = SpawnAtStartupTable;

    fn to_lua(&self) -> SpawnAtStartupTable {
        SpawnAtStartupTable {
            command: self.command.clone(),
        }
    }

    fn from_lua_field(value: SpawnAtStartupTable) -> LuaResult<Self> {
        Ok(SpawnAtStartup {
            command: value.command,
        })
    }
}

#[derive(Clone)]
pub struct SpawnAtStartupTable {
    pub command: Vec<String>,
}

impl IntoLua for SpawnAtStartupTable {
    fn into_lua(self, lua: &Lua) -> LuaResult<Value> {
        let table = lua.create_table()?;
        table.set("command", self.command)?;
        Ok(Value::Table(table))
    }
}

impl FromLua for SpawnAtStartupTable {
    fn from_lua(value: Value, _lua: &Lua) -> LuaResult<Self> {
        match value {
            Value::Table(table) => {
                let command: Vec<String> = table.get("command")?;
                Ok(SpawnAtStartupTable { command })
            }
            _ => Err(LuaError::external(
                "Expected a table for spawn_at_startup with 'command' field",
            )),
        }
    }
}

// =============================================================================
// Helper functions for complex field types
// =============================================================================

use niri_config::{GradientColorSpace, GradientInterpolation, HueInterpolation};

/// Convert a Gradient to a Lua table representation.
pub fn gradient_to_table(lua: &mlua::Lua, gradient: &Gradient) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("from", Color::to_lua(&gradient.from))?;
    table.set("to", Color::to_lua(&gradient.to))?;
    table.set("angle", gradient.angle)?;
    table.set(
        "relative_to",
        match gradient.relative_to {
            GradientRelativeTo::Window => "window",
            GradientRelativeTo::WorkspaceView => "workspace-view",
        },
    )?;
    // Include interpolation settings
    let in_table = lua.create_table()?;
    in_table.set(
        "color_space",
        match gradient.in_.color_space {
            GradientColorSpace::Srgb => "srgb",
            GradientColorSpace::SrgbLinear => "srgb-linear",
            GradientColorSpace::Oklab => "oklab",
            GradientColorSpace::Oklch => "oklch",
        },
    )?;
    in_table.set(
        "hue_interpolation",
        match gradient.in_.hue_interpolation {
            HueInterpolation::Shorter => "shorter",
            HueInterpolation::Longer => "longer",
            HueInterpolation::Increasing => "increasing",
            HueInterpolation::Decreasing => "decreasing",
        },
    )?;
    table.set("in", in_table)?;
    Ok(table)
}

/// Parse a Gradient from a Lua table.
pub fn table_to_gradient(table: mlua::Table) -> mlua::Result<Gradient> {
    use std::str::FromStr;
    let from_str: String = table.get("from")?;
    let to_str: String = table.get("to")?;
    let angle: i16 = table.get::<Option<i16>>("angle")?.unwrap_or(180);
    let relative_to_str: Option<String> = table.get("relative_to")?;
    let in_table: Option<mlua::Table> = table.get("in")?;

    let from = Color::from_str(&from_str)
        .map_err(|e| mlua::Error::external(format!("Invalid 'from' color: {}", e)))?;
    let to = Color::from_str(&to_str)
        .map_err(|e| mlua::Error::external(format!("Invalid 'to' color: {}", e)))?;

    let relative_to = match relative_to_str.as_deref() {
        Some("window") | None => GradientRelativeTo::Window,
        Some("workspace-view") => GradientRelativeTo::WorkspaceView,
        Some(other) => {
            return Err(mlua::Error::external(format!(
                "Invalid relative_to: {}. Expected 'window' or 'workspace-view'",
                other
            )));
        }
    };

    let in_ = if let Some(in_tbl) = in_table {
        let color_space_str: Option<String> = in_tbl.get("color_space")?;
        let hue_str: Option<String> = in_tbl.get("hue_interpolation")?;

        let color_space = match color_space_str.as_deref() {
            Some("srgb") | None => GradientColorSpace::Srgb,
            Some("srgb-linear") => GradientColorSpace::SrgbLinear,
            Some("oklab") => GradientColorSpace::Oklab,
            Some("oklch") => GradientColorSpace::Oklch,
            Some(other) => {
                return Err(mlua::Error::external(format!(
                    "Invalid color_space: {}",
                    other
                )));
            }
        };

        let hue_interpolation = match hue_str.as_deref() {
            Some("shorter") | None => HueInterpolation::Shorter,
            Some("longer") => HueInterpolation::Longer,
            Some("increasing") => HueInterpolation::Increasing,
            Some("decreasing") => HueInterpolation::Decreasing,
            Some(other) => {
                return Err(mlua::Error::external(format!(
                    "Invalid hue_interpolation: {}",
                    other
                )));
            }
        };

        GradientInterpolation {
            color_space,
            hue_interpolation,
        }
    } else {
        GradientInterpolation::default()
    };

    Ok(Gradient {
        from,
        to,
        angle,
        relative_to,
        in_,
    })
}

use niri_config::animations::{Curve, EasingParams, Kind, SpringParams};

/// Convert an animation Kind to a Lua table representation.
pub fn anim_kind_to_table(lua: &mlua::Lua, kind: &Kind) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;

    match kind {
        Kind::Easing(params) => {
            table.set("type", "easing")?;
            table.set("duration_ms", params.duration_ms)?;
            let curve_str = match params.curve {
                Curve::Linear => "linear",
                Curve::EaseOutQuad => "ease-out-quad",
                Curve::EaseOutCubic => "ease-out-cubic",
                Curve::EaseOutExpo => "ease-out-expo",
                Curve::CubicBezier(x1, y1, x2, y2) => {
                    // For cubic bezier, include the control points
                    let points = lua.create_table()?;
                    points.set("x1", x1)?;
                    points.set("y1", y1)?;
                    points.set("x2", x2)?;
                    points.set("y2", y2)?;
                    table.set("cubic_bezier", points)?;
                    "cubic-bezier"
                }
            };
            table.set("curve", curve_str)?;
        }
        Kind::Spring(params) => {
            table.set("type", "spring")?;
            table.set("damping_ratio", params.damping_ratio)?;
            table.set("stiffness", params.stiffness)?;
            table.set("epsilon", params.epsilon)?;
        }
    }

    Ok(table)
}

/// Parse an animation Kind from a Lua table.
pub fn table_to_anim_kind(table: mlua::Table) -> mlua::Result<Kind> {
    let kind_type: String = table.get("type")?;

    match kind_type.as_str() {
        "easing" => {
            let duration_ms: u32 = table.get::<Option<u32>>("duration_ms")?.unwrap_or(250);
            let curve_str: Option<String> = table.get("curve")?;
            let cubic_bezier: Option<mlua::Table> = table.get("cubic_bezier")?;

            let curve = if let Some(cb) = cubic_bezier {
                let x1: f64 = cb.get("x1")?;
                let y1: f64 = cb.get("y1")?;
                let x2: f64 = cb.get("x2")?;
                let y2: f64 = cb.get("y2")?;
                Curve::CubicBezier(x1, y1, x2, y2)
            } else {
                match curve_str.as_deref() {
                    Some("linear") => Curve::Linear,
                    Some("ease-out-quad") => Curve::EaseOutQuad,
                    Some("ease-out-cubic") | None => Curve::EaseOutCubic,
                    Some("ease-out-expo") => Curve::EaseOutExpo,
                    Some(other) => {
                        return Err(mlua::Error::external(format!(
                            "Invalid curve: {}. Expected 'linear', 'ease-out-quad', 'ease-out-cubic', 'ease-out-expo', or provide 'cubic_bezier' table",
                            other
                        )));
                    }
                }
            };

            Ok(Kind::Easing(EasingParams { duration_ms, curve }))
        }
        "spring" => {
            let damping_ratio: f64 = table.get::<Option<f64>>("damping_ratio")?.unwrap_or(1.0);
            let stiffness: u32 = table.get::<Option<u32>>("stiffness")?.unwrap_or(800);
            let epsilon: f64 = table.get::<Option<f64>>("epsilon")?.unwrap_or(0.0001);

            Ok(Kind::Spring(SpringParams {
                damping_ratio,
                stiffness,
                epsilon,
            }))
        }
        other => Err(mlua::Error::external(format!(
            "Invalid animation type: {}. Expected 'easing' or 'spring'",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_convert() {
        assert!(<bool as LuaFieldConvert>::to_lua(&true));
        assert!(!<bool as LuaFieldConvert>::from_lua_field(false).unwrap());
    }

    #[test]
    fn test_int_convert() {
        assert_eq!(LuaFieldConvert::to_lua(&42i32), 42);
        assert_eq!(
            <i32 as LuaFieldConvert>::from_lua_field(123).unwrap(),
            123i32
        );
    }

    #[test]
    fn test_float_convert() {
        assert_eq!(LuaFieldConvert::to_lua(&1.5f64), 1.5);
        assert_eq!(
            <f64 as LuaFieldConvert>::from_lua_field(2.5).unwrap(),
            2.5f64
        );
    }

    #[test]
    fn test_string_convert() {
        let s = "hello".to_string();
        assert_eq!(LuaFieldConvert::to_lua(&s), "hello".to_string());
        assert_eq!(
            <String as LuaFieldConvert>::from_lua_field("world".to_string()).unwrap(),
            "world".to_string()
        );
    }

    #[test]
    fn test_duration_convert() {
        let dur = Duration::from_millis(1500);
        assert_eq!(dur.to_lua(), 1500);
        assert_eq!(
            Duration::from_lua_field(2000).unwrap(),
            Duration::from_millis(2000)
        );
    }

    #[test]
    fn test_color_parsing_rgb() {
        let color = parse_color_string("#f0a").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.0).abs() < 0.01);
        assert!((color.b - 0.666).abs() < 0.01);
        assert!((color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_parsing_rgba() {
        let color = parse_color_string("#f0a8").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.0).abs() < 0.01);
        assert!((color.b - 0.666).abs() < 0.01);
        assert!((color.a - 0.533).abs() < 0.01);
    }

    #[test]
    fn test_color_parsing_rrggbb() {
        let color = parse_color_string("#ff0080").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.0).abs() < 0.01);
        assert!((color.b - 0.502).abs() < 0.01);
        assert!((color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_parsing_rrggbbaa() {
        let color = parse_color_string("#ff008080").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.0).abs() < 0.01);
        assert!((color.b - 0.502).abs() < 0.01);
        assert!((color.a - 0.502).abs() < 0.01);
    }

    #[test]
    fn test_color_roundtrip() {
        let original = Color::from_rgba8_unpremul(255, 128, 64, 200);
        let hex = LuaFieldConvert::to_lua(&original);
        let parsed: Color = <Color as LuaFieldConvert>::from_lua_field(hex).unwrap();

        assert!((original.r - parsed.r).abs() < 0.01);
        assert!((original.g - parsed.g).abs() < 0.01);
        assert!((original.b - parsed.b).abs() < 0.01);
        assert!((original.a - parsed.a).abs() < 0.01);
    }

    #[test]
    fn test_color_parsing_error() {
        assert!(parse_color_string("ff0080").is_err());
        assert!(parse_color_string("#ff008").is_err());
        assert!(parse_color_string("#gg0080").is_err());
    }

    #[test]
    fn test_gradient_relative_to_parsing() {
        assert!(matches!(
            parse_gradient_relative_to("window").unwrap(),
            GradientRelativeTo::Window
        ));
        assert!(matches!(
            parse_gradient_relative_to("workspace-view").unwrap(),
            GradientRelativeTo::WorkspaceView
        ));
        assert!(matches!(
            parse_gradient_relative_to("workspace_view").unwrap(),
            GradientRelativeTo::WorkspaceView
        ));
        assert!(parse_gradient_relative_to("invalid").is_err());
    }

    #[test]
    fn test_gradient_relative_to_string() {
        assert_eq!(
            gradient_relative_to_string(GradientRelativeTo::Window),
            "window"
        );
        assert_eq!(
            gradient_relative_to_string(GradientRelativeTo::WorkspaceView),
            "workspace-view"
        );
    }

    #[test]
    fn test_floatorint_convert() {
        let foi: FloatOrInt<0, 100> = FloatOrInt(42.5);
        assert_eq!(LuaFieldConvert::to_lua(&foi), 42.5);
        assert_eq!(
            <FloatOrInt<0, 100> as LuaFieldConvert>::from_lua_field(1.75).unwrap(),
            FloatOrInt::<0, 100>(1.75)
        );
    }

    #[test]
    fn test_accel_profile_convert() {
        use niri_config::input::AccelProfile;

        let adaptive = AccelProfile::Adaptive;
        assert_eq!(LuaFieldConvert::to_lua(&adaptive), "adaptive");
        assert_eq!(
            <AccelProfile as LuaFieldConvert>::from_lua_field("adaptive".to_string()).unwrap(),
            AccelProfile::Adaptive
        );

        let flat = AccelProfile::Flat;
        assert_eq!(LuaFieldConvert::to_lua(&flat), "flat");
        assert_eq!(
            <AccelProfile as LuaFieldConvert>::from_lua_field("flat".to_string()).unwrap(),
            AccelProfile::Flat
        );

        assert!(<AccelProfile as LuaFieldConvert>::from_lua_field("invalid".to_string()).is_err());
    }

    #[test]
    fn test_click_method_convert() {
        use niri_config::input::ClickMethod;

        let button_areas = ClickMethod::ButtonAreas;
        assert_eq!(LuaFieldConvert::to_lua(&button_areas), "button-areas");
        assert_eq!(
            <ClickMethod as LuaFieldConvert>::from_lua_field("button-areas".to_string()).unwrap(),
            ClickMethod::ButtonAreas
        );

        let clickfinger = ClickMethod::Clickfinger;
        assert_eq!(LuaFieldConvert::to_lua(&clickfinger), "clickfinger");
        assert_eq!(
            <ClickMethod as LuaFieldConvert>::from_lua_field("clickfinger".to_string()).unwrap(),
            ClickMethod::Clickfinger
        );

        assert!(<ClickMethod as LuaFieldConvert>::from_lua_field("invalid".to_string()).is_err());
    }

    #[test]
    fn test_scroll_method_convert() {
        use niri_config::input::ScrollMethod;

        assert_eq!(
            LuaFieldConvert::to_lua(&ScrollMethod::NoScroll),
            "no-scroll"
        );
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua_field("no-scroll".to_string()).unwrap(),
            ScrollMethod::NoScroll
        );

        assert_eq!(
            LuaFieldConvert::to_lua(&ScrollMethod::TwoFinger),
            "two-finger"
        );
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua_field("two-finger".to_string()).unwrap(),
            ScrollMethod::TwoFinger
        );

        assert_eq!(LuaFieldConvert::to_lua(&ScrollMethod::Edge), "edge");
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua_field("edge".to_string()).unwrap(),
            ScrollMethod::Edge
        );

        assert_eq!(
            LuaFieldConvert::to_lua(&ScrollMethod::OnButtonDown),
            "on-button-down"
        );
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua_field("on-button-down".to_string())
                .unwrap(),
            ScrollMethod::OnButtonDown
        );

        assert!(<ScrollMethod as LuaFieldConvert>::from_lua_field("invalid".to_string()).is_err());
    }

    #[test]
    fn test_tap_button_map_convert() {
        use niri_config::input::TapButtonMap;

        let lrm = TapButtonMap::LeftRightMiddle;
        assert_eq!(LuaFieldConvert::to_lua(&lrm), "left-right-middle");
        assert_eq!(
            <TapButtonMap as LuaFieldConvert>::from_lua_field("left-right-middle".to_string())
                .unwrap(),
            TapButtonMap::LeftRightMiddle
        );

        let lmr = TapButtonMap::LeftMiddleRight;
        assert_eq!(LuaFieldConvert::to_lua(&lmr), "left-middle-right");
        assert_eq!(
            <TapButtonMap as LuaFieldConvert>::from_lua_field("left-middle-right".to_string())
                .unwrap(),
            TapButtonMap::LeftMiddleRight
        );

        assert!(<TapButtonMap as LuaFieldConvert>::from_lua_field("invalid".to_string()).is_err());
    }

    #[test]
    fn test_track_layout_convert() {
        use niri_config::input::TrackLayout;

        let global = TrackLayout::Global;
        assert_eq!(LuaFieldConvert::to_lua(&global), "global");
        assert_eq!(
            <TrackLayout as LuaFieldConvert>::from_lua_field("global".to_string()).unwrap(),
            TrackLayout::Global
        );

        let window = TrackLayout::Window;
        assert_eq!(LuaFieldConvert::to_lua(&window), "window");
        assert_eq!(
            <TrackLayout as LuaFieldConvert>::from_lua_field("window".to_string()).unwrap(),
            TrackLayout::Window
        );

        assert!(<TrackLayout as LuaFieldConvert>::from_lua_field("invalid".to_string()).is_err());
    }

    #[test]
    fn test_spawn_at_startup_convert() {
        use niri_config::SpawnAtStartup;

        let spawn = SpawnAtStartup {
            command: vec!["kitty".to_string(), "-e".to_string(), "fish".to_string()],
        };

        let table = LuaFieldConvert::to_lua(&spawn);
        assert_eq!(table.command, vec!["kitty", "-e", "fish"]);

        let spawn_table = SpawnAtStartupTable {
            command: vec!["alacritty".to_string()],
        };
        let result: SpawnAtStartup = SpawnAtStartup::from_lua_field(spawn_table).unwrap();
        assert_eq!(result.command, vec!["alacritty"]);
    }
}
