//! Attribute parsing utilities for derive macros.

use syn::{Attribute, LitStr};

/// Rename conventions for enum variants
#[derive(Debug, Clone, Copy, Default)]
#[allow(clippy::enum_variant_names)]
pub enum RenameAll {
    #[default]
    KebabCase,
    SnakeCase,
    ScreamingSnakeCase,
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
