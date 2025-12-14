//! Attribute parsing utilities for derive macros.

use syn::{Attribute, LitStr};

/// Rename conventions for enum variants
#[derive(Debug, Clone, Copy, Default)]
pub enum RenameAll {
    #[default]
    KebabCase,
    SnakeCase,
    ScreamingSnakeCase,
}

/// Field kind in a proxy struct
#[derive(Debug, Clone, Copy, Default)]
pub enum FieldKind {
    #[default]
    Simple,
    Nested,
    Collection,
    Skip,
}

/// Parsed attributes for a struct
#[derive(Debug, Default)]
pub struct StructAttrs {
    pub is_root: bool,
    pub parent_path: Option<String>,
    pub dirty_flag: Option<String>,
    pub generate_dirty_flags: bool,
}

/// Parsed attributes for a field
#[derive(Debug, Default)]
pub struct FieldAttrs {
    pub kind: FieldKind,
    pub readonly: bool,
    pub lua_name: Option<String>,
    pub dirty_override: Option<String>,
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
                    }
                    Ok(())
                })?;
            }
        }

        Ok(result)
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
                    } else if meta.path.is_ident("nested") {
                        result.kind = FieldKind::Nested;
                    } else if meta.path.is_ident("collection") {
                        result.kind = FieldKind::Collection;
                    } else if meta.path.is_ident("skip") {
                        result.kind = FieldKind::Skip;
                    } else if meta.path.is_ident("readonly") {
                        result.readonly = true;
                    } else if meta.path.is_ident("name") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.lua_name = Some(value.value());
                    } else if meta.path.is_ident("dirty") {
                        let value: LitStr = meta.value()?.parse()?;
                        result.dirty_override = Some(value.value());
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
