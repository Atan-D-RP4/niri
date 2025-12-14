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
    fn from_lua(value: Self::LuaType) -> LuaResult<Self>;
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

impl LuaFieldConvert for bool {
    type LuaType = bool;

    fn to_lua(&self) -> bool {
        *self
    }

    fn from_lua(value: bool) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for i32 {
    type LuaType = i32;

    fn to_lua(&self) -> i32 {
        *self
    }

    fn from_lua(value: i32) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for i64 {
    type LuaType = i64;

    fn to_lua(&self) -> i64 {
        *self
    }

    fn from_lua(value: i64) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for u8 {
    type LuaType = u8;

    fn to_lua(&self) -> u8 {
        *self
    }

    fn from_lua(value: u8) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for u16 {
    type LuaType = u16;

    fn to_lua(&self) -> u16 {
        *self
    }

    fn from_lua(value: u16) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for u32 {
    type LuaType = u32;

    fn to_lua(&self) -> u32 {
        *self
    }

    fn from_lua(value: u32) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for u64 {
    type LuaType = u64;

    fn to_lua(&self) -> u64 {
        *self
    }

    fn from_lua(value: u64) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for f64 {
    type LuaType = f64;

    fn to_lua(&self) -> f64 {
        *self
    }

    fn from_lua(value: f64) -> LuaResult<Self> {
        Ok(value)
    }
}

impl LuaFieldConvert for String {
    type LuaType = String;

    fn to_lua(&self) -> String {
        self.clone()
    }

    fn from_lua(value: String) -> LuaResult<Self> {
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

    fn from_lua(value: u64) -> LuaResult<Self> {
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

    fn from_lua(value: String) -> LuaResult<Self> {
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
            let r = parse_hex_digit(hex.chars().nth(0).unwrap())?;
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
            let r = parse_hex_digit(hex.chars().nth(0).unwrap())?;
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

    fn from_lua(value: GradientTable) -> LuaResult<Self> {
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

impl<'lua> IntoLua for GradientTable {
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

impl<'lua> FromLua for GradientTable {
    fn from_lua(value: Value, _lua: &Lua) -> LuaResult<Self> {
        match value {
            Value::Table(table) => {
                let from: String = table.get("from")?;
                let to: String = table.get("to")?;
                // Get angle as Option, default to 180
                let angle: i16 = match table.get::<Option<i16>>("angle")? {
                    Some(a) => a,
                    None => 180,
                };
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

    fn from_lua(value: f64) -> LuaResult<Self> {
        Ok(FloatOrInt(value))
    }
}

// ============================================================================
// Input Device Enum Implementations
// ============================================================================

use niri_config::input::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap, TrackLayout};

impl LuaFieldConvert for AccelProfile {
    type LuaType = String;

    fn to_lua(&self) -> Self::LuaType {
        match self {
            AccelProfile::Adaptive => "adaptive",
            AccelProfile::Flat => "flat",
        }
        .to_string()
    }

    fn from_lua(value: Self::LuaType) -> LuaResult<Self> {
        match value.as_str() {
            "adaptive" => Ok(AccelProfile::Adaptive),
            "flat" => Ok(AccelProfile::Flat),
            _ => Err(LuaError::external(format!(
                "Invalid accel_profile '{}'. Expected: adaptive, flat",
                value
            ))),
        }
    }
}

impl LuaFieldConvert for ClickMethod {
    type LuaType = String;

    fn to_lua(&self) -> Self::LuaType {
        match self {
            ClickMethod::ButtonAreas => "button-areas",
            ClickMethod::Clickfinger => "clickfinger",
        }
        .to_string()
    }

    fn from_lua(value: Self::LuaType) -> LuaResult<Self> {
        match value.as_str() {
            "button-areas" => Ok(ClickMethod::ButtonAreas),
            "clickfinger" => Ok(ClickMethod::Clickfinger),
            _ => Err(LuaError::external(format!(
                "Invalid click_method '{}'. Expected: button-areas, clickfinger",
                value
            ))),
        }
    }
}

impl LuaFieldConvert for ScrollMethod {
    type LuaType = String;

    fn to_lua(&self) -> Self::LuaType {
        match self {
            ScrollMethod::NoScroll => "no-scroll",
            ScrollMethod::TwoFinger => "two-finger",
            ScrollMethod::Edge => "edge",
            ScrollMethod::OnButtonDown => "on-button-down",
        }
        .to_string()
    }

    fn from_lua(value: Self::LuaType) -> LuaResult<Self> {
        match value.as_str() {
            "no-scroll" => Ok(ScrollMethod::NoScroll),
            "two-finger" => Ok(ScrollMethod::TwoFinger),
            "edge" => Ok(ScrollMethod::Edge),
            "on-button-down" => Ok(ScrollMethod::OnButtonDown),
            _ => Err(LuaError::external(format!(
                "Invalid scroll_method '{}'. Expected: no-scroll, two-finger, edge, on-button-down",
                value
            ))),
        }
    }
}

impl LuaFieldConvert for TapButtonMap {
    type LuaType = String;

    fn to_lua(&self) -> Self::LuaType {
        match self {
            TapButtonMap::LeftRightMiddle => "left-right-middle",
            TapButtonMap::LeftMiddleRight => "left-middle-right",
        }
        .to_string()
    }

    fn from_lua(value: Self::LuaType) -> LuaResult<Self> {
        match value.as_str() {
            "left-right-middle" => Ok(TapButtonMap::LeftRightMiddle),
            "left-middle-right" => Ok(TapButtonMap::LeftMiddleRight),
            _ => Err(LuaError::external(format!(
                "Invalid tap_button_map '{}'. Expected: left-right-middle, left-middle-right",
                value
            ))),
        }
    }
}

impl LuaFieldConvert for TrackLayout {
    type LuaType = String;

    fn to_lua(&self) -> Self::LuaType {
        match self {
            TrackLayout::Global => "global",
            TrackLayout::Window => "window",
        }
        .to_string()
    }

    fn from_lua(value: Self::LuaType) -> LuaResult<Self> {
        match value.as_str() {
            "global" => Ok(TrackLayout::Global),
            "window" => Ok(TrackLayout::Window),
            _ => Err(LuaError::external(format!(
                "Invalid track_layout '{}'. Expected: global, window",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_convert() {
        assert_eq!(LuaFieldConvert::to_lua(&true), true);
        assert_eq!(<bool as LuaFieldConvert>::from_lua(false).unwrap(), false);
    }

    #[test]
    fn test_int_convert() {
        assert_eq!(LuaFieldConvert::to_lua(&42i32), 42);
        assert_eq!(<i32 as LuaFieldConvert>::from_lua(123).unwrap(), 123i32);
    }

    #[test]
    fn test_float_convert() {
        assert_eq!(LuaFieldConvert::to_lua(&3.14f64), 3.14);
        assert_eq!(<f64 as LuaFieldConvert>::from_lua(2.718).unwrap(), 2.718f64);
    }

    #[test]
    fn test_string_convert() {
        let s = "hello".to_string();
        assert_eq!(LuaFieldConvert::to_lua(&s), "hello".to_string());
        assert_eq!(
            <String as LuaFieldConvert>::from_lua("world".to_string()).unwrap(),
            "world".to_string()
        );
    }

    #[test]
    fn test_duration_convert() {
        let dur = Duration::from_millis(1500);
        assert_eq!(dur.to_lua(), 1500);
        assert_eq!(
            Duration::from_lua(2000).unwrap(),
            Duration::from_millis(2000)
        );
    }

    #[test]
    fn test_color_parsing_rgb() {
        let color = parse_color_string("#f0a").unwrap();
        // #f0a expands to #ff00aa
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.0).abs() < 0.01);
        assert!((color.b - 0.666).abs() < 0.01);
        assert!((color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_parsing_rgba() {
        let color = parse_color_string("#f0a8").unwrap();
        // #f0a8 expands to #ff00aa88
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
        let parsed: Color = <Color as LuaFieldConvert>::from_lua(hex).unwrap();

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
            <FloatOrInt<0, 100> as LuaFieldConvert>::from_lua(3.14).unwrap(),
            FloatOrInt::<0, 100>(3.14)
        );
    }

    #[test]
    fn test_accel_profile_convert() {
        use niri_config::input::AccelProfile;

        let adaptive = AccelProfile::Adaptive;
        assert_eq!(LuaFieldConvert::to_lua(&adaptive), "adaptive");
        assert_eq!(
            <AccelProfile as LuaFieldConvert>::from_lua("adaptive".to_string()).unwrap(),
            AccelProfile::Adaptive
        );

        let flat = AccelProfile::Flat;
        assert_eq!(LuaFieldConvert::to_lua(&flat), "flat");
        assert_eq!(
            <AccelProfile as LuaFieldConvert>::from_lua("flat".to_string()).unwrap(),
            AccelProfile::Flat
        );

        assert!(<AccelProfile as LuaFieldConvert>::from_lua("invalid".to_string()).is_err());
    }

    #[test]
    fn test_click_method_convert() {
        use niri_config::input::ClickMethod;

        let button_areas = ClickMethod::ButtonAreas;
        assert_eq!(LuaFieldConvert::to_lua(&button_areas), "button-areas");
        assert_eq!(
            <ClickMethod as LuaFieldConvert>::from_lua("button-areas".to_string()).unwrap(),
            ClickMethod::ButtonAreas
        );

        let clickfinger = ClickMethod::Clickfinger;
        assert_eq!(LuaFieldConvert::to_lua(&clickfinger), "clickfinger");
        assert_eq!(
            <ClickMethod as LuaFieldConvert>::from_lua("clickfinger".to_string()).unwrap(),
            ClickMethod::Clickfinger
        );

        assert!(<ClickMethod as LuaFieldConvert>::from_lua("invalid".to_string()).is_err());
    }

    #[test]
    fn test_scroll_method_convert() {
        use niri_config::input::ScrollMethod;

        assert_eq!(
            LuaFieldConvert::to_lua(&ScrollMethod::NoScroll),
            "no-scroll"
        );
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua("no-scroll".to_string()).unwrap(),
            ScrollMethod::NoScroll
        );

        assert_eq!(
            LuaFieldConvert::to_lua(&ScrollMethod::TwoFinger),
            "two-finger"
        );
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua("two-finger".to_string()).unwrap(),
            ScrollMethod::TwoFinger
        );

        assert_eq!(LuaFieldConvert::to_lua(&ScrollMethod::Edge), "edge");
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua("edge".to_string()).unwrap(),
            ScrollMethod::Edge
        );

        assert_eq!(
            LuaFieldConvert::to_lua(&ScrollMethod::OnButtonDown),
            "on-button-down"
        );
        assert_eq!(
            <ScrollMethod as LuaFieldConvert>::from_lua("on-button-down".to_string()).unwrap(),
            ScrollMethod::OnButtonDown
        );

        assert!(<ScrollMethod as LuaFieldConvert>::from_lua("invalid".to_string()).is_err());
    }

    #[test]
    fn test_tap_button_map_convert() {
        use niri_config::input::TapButtonMap;

        let lrm = TapButtonMap::LeftRightMiddle;
        assert_eq!(LuaFieldConvert::to_lua(&lrm), "left-right-middle");
        assert_eq!(
            <TapButtonMap as LuaFieldConvert>::from_lua("left-right-middle".to_string()).unwrap(),
            TapButtonMap::LeftRightMiddle
        );

        let lmr = TapButtonMap::LeftMiddleRight;
        assert_eq!(LuaFieldConvert::to_lua(&lmr), "left-middle-right");
        assert_eq!(
            <TapButtonMap as LuaFieldConvert>::from_lua("left-middle-right".to_string()).unwrap(),
            TapButtonMap::LeftMiddleRight
        );

        assert!(<TapButtonMap as LuaFieldConvert>::from_lua("invalid".to_string()).is_err());
    }

    #[test]
    fn test_track_layout_convert() {
        use niri_config::input::TrackLayout;

        let global = TrackLayout::Global;
        assert_eq!(LuaFieldConvert::to_lua(&global), "global");
        assert_eq!(
            <TrackLayout as LuaFieldConvert>::from_lua("global".to_string()).unwrap(),
            TrackLayout::Global
        );

        let window = TrackLayout::Window;
        assert_eq!(LuaFieldConvert::to_lua(&window), "window");
        assert_eq!(
            <TrackLayout as LuaFieldConvert>::from_lua("window".to_string()).unwrap(),
            TrackLayout::Window
        );

        assert!(<TrackLayout as LuaFieldConvert>::from_lua("invalid".to_string()).is_err());
    }
}
