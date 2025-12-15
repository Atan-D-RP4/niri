//! Attribute parsing utilities for derive macros.

use quote::quote;
use syn::{Attribute, LitStr, Type};

/// Rename conventions for enum variants
#[derive(Debug, Clone, Copy, Default)]
#[allow(clippy::enum_variant_names)]
pub enum RenameAll {
    #[default]
    KebabCase,
    SnakeCase,
    ScreamingSnakeCase,
}

/// Field kind in a proxy struct
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FieldKind {
    #[default]
    Simple,
    Nested,
    Collection,
    Skip,
    /// Gradient field - uses table with from/to/angle/relative_to/in
    Gradient,
    /// Offset field - uses table with {x, y}
    Offset,
    /// Animation Kind field - uses table with type/easing/spring params
    AnimKind,
    /// Inverted boolean field - get returns !value, set stores !value
    Inverted,
}

/// Field detection mode for the derive macro
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FieldDetectionMode {
    /// Auto-detect field kinds from types (default when no field annotations present)
    #[default]
    Auto,
    /// Require explicit annotations for all fields
    Explicit,
}

/// Parsed attributes for a struct
#[derive(Debug, Default)]
pub struct StructAttrs {
    pub is_root: bool,
    pub parent_path: Option<String>,
    pub dirty_flag: Option<String>,
    pub generate_dirty_flags: bool,
    /// Custom crate path for niri_lua (e.g., "crate" when used inside niri-lua)
    pub crate_path: Option<String>,
    /// Field detection mode (auto_detect or explicit)
    pub detection_mode: Option<FieldDetectionMode>,
}

/// Parsed attributes for a field
#[derive(Debug, Default)]
pub struct FieldAttrs {
    pub kind: FieldKind,
    pub readonly: bool,
    pub lua_name: Option<String>,
    pub dirty_override: Option<String>,
    /// Custom path relative to parent_path (e.g., "sibling.child" to access parent.sibling.child)
    /// When set, this overrides the field name in the access path.
    /// Use ".." to go up one level, e.g., "../custom_shader" for animations.window_open.anim ->
    /// animations.window_open.custom_shader
    pub custom_path: Option<String>,
    /// Whether the field kind was explicitly set via annotation
    pub has_explicit_kind: bool,
}

/// Parsed attributes for an enum
#[derive(Debug, Default)]
pub struct EnumAttrs {
    pub rename_all: RenameAll,
}

/// Parsed attributes for a variant
#[derive(Debug, Default)]
pub struct VariantAttrs {
    pub rename: Option<String>,
}

impl StructAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("lua_proxy") {
                // Parse the attribute contents
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("root") {
                        result.is_root = true;
                    } else if meta.path.is_ident("generate_dirty_flags") {
                        result.generate_dirty_flags = true;
                    } else if meta.path.is_ident("parent_path") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.parent_path = Some(value.value());
                    } else if meta.path.is_ident("dirty") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.dirty_flag = Some(value.value());
                    } else if meta.path.is_ident("crate") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.crate_path = Some(value.value());
                    } else if meta.path.is_ident("auto_detect") {
                        result.detection_mode = Some(FieldDetectionMode::Auto);
                    } else if meta.path.is_ident("explicit") {
                        result.detection_mode = Some(FieldDetectionMode::Explicit);
                    }
                    Ok(())
                })?;
            }
        }

        Ok(result)
    }

    /// Get the crate path to use for niri_lua types
    pub fn get_crate_path(&self) -> proc_macro2::TokenStream {
        match &self.crate_path {
            Some(path) => {
                let path_ident = syn::parse_str::<syn::Path>(path).expect("Invalid crate path");
                quote::quote! { #path_ident }
            }
            None => quote::quote! { niri_lua },
        }
    }
}

impl FieldAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("lua_proxy") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("field") {
                        result.kind = FieldKind::Simple;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("nested") {
                        result.kind = FieldKind::Nested;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("collection") {
                        result.kind = FieldKind::Collection;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("skip") {
                        result.kind = FieldKind::Skip;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("gradient") {
                        result.kind = FieldKind::Gradient;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("offset") {
                        result.kind = FieldKind::Offset;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("anim_kind") {
                        result.kind = FieldKind::AnimKind;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("inverted") {
                        result.kind = FieldKind::Inverted;
                        result.has_explicit_kind = true;
                    } else if meta.path.is_ident("readonly") {
                        result.readonly = true;
                    } else if meta.path.is_ident("name") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.lua_name = Some(value.value());
                    } else if meta.path.is_ident("dirty") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.dirty_override = Some(value.value());
                    } else if meta.path.is_ident("path") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.custom_path = Some(value.value());
                    }
                    Ok(())
                })?;
            }
        }

        Ok(result)
    }
}

impl EnumAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("lua_enum") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename_all") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.rename_all = match value.value().as_str() {
                            "kebab-case" => RenameAll::KebabCase,
                            "snake_case" => RenameAll::SnakeCase,
                            "SCREAMING_SNAKE_CASE" => RenameAll::ScreamingSnakeCase,
                            other => {
                                return Err(syn::Error::new_spanned(
                                    value,
                                    format!("Unknown rename_all value: {}", other),
                                ));
                            }
                        };
                    }
                    Ok(())
                })?;
            }
        }

        Ok(result)
    }
}

impl VariantAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("lua_enum") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.rename = Some(value.value());
                    }
                    Ok(())
                })?;
            }
        }

        Ok(result)
    }
}

/// Convert an identifier to kebab-case
pub fn to_kebab_case(ident: &str) -> String {
    use convert_case::{Case, Casing};
    ident.to_case(Case::Kebab)
}

/// Convert an identifier to snake_case
pub fn to_snake_case(ident: &str) -> String {
    use convert_case::{Case, Casing};
    ident.to_case(Case::Snake)
}

/// Convert an identifier to SCREAMING_SNAKE_CASE
pub fn to_screaming_snake_case(ident: &str) -> String {
    use convert_case::{Case, Casing};
    ident.to_case(Case::Upper)
}

/// Apply rename convention to an identifier
pub fn apply_rename(ident: &str, convention: RenameAll) -> String {
    match convention {
        RenameAll::KebabCase => to_kebab_case(ident),
        RenameAll::SnakeCase => to_snake_case(ident),
        RenameAll::ScreamingSnakeCase => to_screaming_snake_case(ident),
    }
}

// ============================================================================
// Field kind inference from types
// ============================================================================

/// Infer the field kind from a type when not explicitly annotated.
///
/// Returns `Some(FieldKind)` if the type can be auto-detected, or `None` if
/// an explicit annotation is required.
///
/// Auto-detection rules:
/// - Primitives (bool, u8-u64, i8-i64, f32, f64, String) → Simple
/// - Option<primitive> → Simple
/// - Color, FloatOrInt<_, _> → Simple
/// - Gradient, Option<Gradient> → Gradient
/// - ShadowOffset → Offset
/// - Kind (animation) → AnimKind
/// - Vec<T> → Collection
/// - Type ending with "Config" → Nested
/// - Everything else → None (requires explicit annotation)
pub fn infer_field_kind(ty: &Type) -> Option<FieldKind> {
    let type_str = quote!(#ty).to_string();

    // Normalize whitespace for consistent matching
    let type_str = normalize_type_string(&type_str);

    // Check for skip (never auto-detected)
    // Skip must always be explicit

    // Primitives -> Simple
    if is_primitive_type_str(&type_str) {
        return Some(FieldKind::Simple);
    }

    // Option<primitive> -> Simple
    if let Some(inner) = extract_option_inner_str(&type_str) {
        if is_primitive_type_str(inner) {
            return Some(FieldKind::Simple);
        }
        // Option<Color> -> Simple
        if inner == "Color" {
            return Some(FieldKind::Simple);
        }
        // Option<Gradient> -> Gradient
        if inner == "Gradient" {
            return Some(FieldKind::Gradient);
        }
        // Option<*Config> -> Nested
        if inner.ends_with("Config") {
            return Some(FieldKind::Nested);
        }
    }

    // Known config types -> Simple
    if type_str == "Color" || type_str.starts_with("FloatOrInt<") {
        return Some(FieldKind::Simple);
    }

    // Gradient -> Gradient
    if type_str == "Gradient" {
        return Some(FieldKind::Gradient);
    }

    // ShadowOffset -> Offset
    if type_str == "ShadowOffset" {
        return Some(FieldKind::Offset);
    }

    // Animation Kind -> AnimKind
    if type_str == "Kind" {
        return Some(FieldKind::AnimKind);
    }

    // Vec<T> -> Collection
    if type_str.starts_with("Vec<") {
        return Some(FieldKind::Collection);
    }

    // *Config -> Nested
    if type_str.ends_with("Config") {
        return Some(FieldKind::Nested);
    }

    // Known enum types -> Simple
    const KNOWN_ENUMS: &[&str] = &[
        "TrackLayout",
        "ScrollMethod",
        "ClickMethod",
        "TapButtonMap",
        "AccelProfile",
        "TabIndicatorPosition",
        "CenterFocusedColumn",
        "ColumnDisplay",
    ];
    if KNOWN_ENUMS.contains(&type_str.as_str()) {
        return Some(FieldKind::Simple);
    }

    // Also handle Option<enum>
    if let Some(inner) = extract_option_inner_str(&type_str) {
        if KNOWN_ENUMS.contains(&inner) {
            return Some(FieldKind::Simple);
        }
    }

    // Unknown type - require explicit annotation
    None
}

/// Normalize a type string by removing extra whitespace around angle brackets.
/// Converts "Option < T >" to "Option<T>" for consistent matching.
fn normalize_type_string(s: &str) -> String {
    // Remove all spaces around angle brackets and commas
    let mut result = s.to_string();
    // Handle spaces around <
    result = result.replace(" <", "<");
    result = result.replace("< ", "<");
    // Handle spaces around >
    result = result.replace(" >", ">");
    result = result.replace("> ", ">");
    // Handle spaces around commas (normalize to ", ")
    result = result.replace(" ,", ",");
    result = result.replace(", ", ",");
    result = result.replace(",", ", ");
    result
}

/// Check if a type string represents a primitive type
fn is_primitive_type_str(s: &str) -> bool {
    matches!(
        s,
        "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
            | "String"
            | "& str"
            | "&str"
    )
}

/// Extract the inner type from an Option<T> type string
fn extract_option_inner_str(s: &str) -> Option<&str> {
    if s.starts_with("Option<") && s.ends_with('>') {
        Some(&s[7..s.len() - 1])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to parse a type string into a syn::Type for testing
    fn parse_type(s: &str) -> syn::Type {
        syn::parse_str(s).expect("Failed to parse type")
    }

    #[test]
    fn test_infer_primitives() {
        // Boolean
        assert_eq!(
            infer_field_kind(&parse_type("bool")),
            Some(FieldKind::Simple)
        );

        // Unsigned integers
        assert_eq!(infer_field_kind(&parse_type("u8")), Some(FieldKind::Simple));
        assert_eq!(
            infer_field_kind(&parse_type("u16")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("u32")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("u64")),
            Some(FieldKind::Simple)
        );

        // Signed integers
        assert_eq!(infer_field_kind(&parse_type("i8")), Some(FieldKind::Simple));
        assert_eq!(
            infer_field_kind(&parse_type("i32")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("i64")),
            Some(FieldKind::Simple)
        );

        // Floats
        assert_eq!(
            infer_field_kind(&parse_type("f32")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("f64")),
            Some(FieldKind::Simple)
        );

        // String
        assert_eq!(
            infer_field_kind(&parse_type("String")),
            Some(FieldKind::Simple)
        );
    }

    #[test]
    fn test_infer_option_primitives() {
        assert_eq!(
            infer_field_kind(&parse_type("Option<bool>")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("Option<u32>")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("Option<String>")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("Option<f64>")),
            Some(FieldKind::Simple)
        );
    }

    #[test]
    fn test_infer_config_types() {
        // Color type -> Simple
        assert_eq!(
            infer_field_kind(&parse_type("Color")),
            Some(FieldKind::Simple)
        );

        // Option<Color> -> Simple
        assert_eq!(
            infer_field_kind(&parse_type("Option<Color>")),
            Some(FieldKind::Simple)
        );

        // FloatOrInt -> Simple
        assert_eq!(
            infer_field_kind(&parse_type("FloatOrInt<-1, 1>")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("FloatOrInt<-65535, 65535>")),
            Some(FieldKind::Simple)
        );
    }

    #[test]
    fn test_infer_gradient() {
        assert_eq!(
            infer_field_kind(&parse_type("Gradient")),
            Some(FieldKind::Gradient)
        );
        assert_eq!(
            infer_field_kind(&parse_type("Option<Gradient>")),
            Some(FieldKind::Gradient)
        );
    }

    #[test]
    fn test_infer_special_types() {
        // ShadowOffset -> Offset
        assert_eq!(
            infer_field_kind(&parse_type("ShadowOffset")),
            Some(FieldKind::Offset)
        );

        // Kind (animation) -> AnimKind
        assert_eq!(
            infer_field_kind(&parse_type("Kind")),
            Some(FieldKind::AnimKind)
        );
    }

    #[test]
    fn test_infer_collection() {
        assert_eq!(
            infer_field_kind(&parse_type("Vec<String>")),
            Some(FieldKind::Collection)
        );
        assert_eq!(
            infer_field_kind(&parse_type("Vec<u32>")),
            Some(FieldKind::Collection)
        );
        assert_eq!(
            infer_field_kind(&parse_type("Vec<WindowRule>")),
            Some(FieldKind::Collection)
        );
    }

    #[test]
    fn test_infer_nested_config() {
        // Types ending with Config -> Nested
        assert_eq!(
            infer_field_kind(&parse_type("CursorConfig")),
            Some(FieldKind::Nested)
        );
        assert_eq!(
            infer_field_kind(&parse_type("KeyboardConfig")),
            Some(FieldKind::Nested)
        );
        assert_eq!(
            infer_field_kind(&parse_type("FocusRingConfig")),
            Some(FieldKind::Nested)
        );

        // Option<*Config> -> Nested
        assert_eq!(
            infer_field_kind(&parse_type("Option<SomeConfig>")),
            Some(FieldKind::Nested)
        );
    }

    #[test]
    fn test_infer_unknown_returns_none() {
        // Unknown types should return None (require explicit annotation)
        assert_eq!(infer_field_kind(&parse_type("SomeUnknownType")), None);
        assert_eq!(infer_field_kind(&parse_type("CustomEnum")), None);
        assert_eq!(infer_field_kind(&parse_type("Option<UnknownType>")), None);
    }

    #[test]
    fn test_normalize_type_string() {
        assert_eq!(normalize_type_string("Option < T >"), "Option<T>");
        assert_eq!(normalize_type_string("Vec< String >"), "Vec<String>");
        assert_eq!(
            normalize_type_string("FloatOrInt < -1 , 1 >"),
            "FloatOrInt<-1, 1>"
        );
    }

    #[test]
    fn test_is_primitive_type_str() {
        assert!(is_primitive_type_str("bool"));
        assert!(is_primitive_type_str("u8"));
        assert!(is_primitive_type_str("String"));
        assert!(is_primitive_type_str("f64"));

        assert!(!is_primitive_type_str("Color"));
        assert!(!is_primitive_type_str("Vec<u8>"));
        assert!(!is_primitive_type_str("Option<bool>"));
    }

    #[test]
    fn test_extract_option_inner_str() {
        assert_eq!(extract_option_inner_str("Option<bool>"), Some("bool"));
        assert_eq!(extract_option_inner_str("Option<String>"), Some("String"));
        assert_eq!(
            extract_option_inner_str("Option<Gradient>"),
            Some("Gradient")
        );

        assert_eq!(extract_option_inner_str("bool"), None);
        assert_eq!(extract_option_inner_str("Vec<u8>"), None);
    }

    #[test]
    fn test_infer_known_enums() {
        // Known enum types -> Simple
        assert_eq!(
            infer_field_kind(&parse_type("TrackLayout")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("ScrollMethod")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("ClickMethod")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("TapButtonMap")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("AccelProfile")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("TabIndicatorPosition")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("CenterFocusedColumn")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("ColumnDisplay")),
            Some(FieldKind::Simple)
        );

        // Option<enum> -> Simple
        assert_eq!(
            infer_field_kind(&parse_type("Option<ScrollMethod>")),
            Some(FieldKind::Simple)
        );
        assert_eq!(
            infer_field_kind(&parse_type("Option<AccelProfile>")),
            Some(FieldKind::Simple)
        );
    }
}
