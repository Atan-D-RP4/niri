use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

struct ConfigStructAttrs {
    prefix: String,
}

struct ConfigFieldAttrs {
    skip: bool,
    no_signal: bool,
    rename: Option<String>,
    enum_type: bool,
    enum_variants: Option<Vec<String>>,
}

fn parse_struct_attrs(attrs: &[syn::Attribute]) -> syn::Result<ConfigStructAttrs> {
    let mut prefix = String::new();

    for attr in attrs {
        if attr.path().is_ident("config") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("prefix") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    prefix = value.value();
                }
                Ok(())
            })?;
        }
    }

    if prefix.is_empty() {
        return Err(syn::Error::new_spanned(
            &attrs[0],
            "ConfigProperties requires non-empty #[config(prefix = \"...\")]",
        ));
    }

    Ok(ConfigStructAttrs { prefix })
}

fn parse_field_attrs(attrs: &[syn::Attribute]) -> syn::Result<ConfigFieldAttrs> {
    let mut skip = false;
    let mut no_signal = false;
    let mut rename = None;
    let mut enum_type = false;
    let mut enum_variants = None;

    for attr in attrs {
        if attr.path().is_ident("config") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skip = true;
                } else if meta.path.is_ident("no_signal") {
                    no_signal = true;
                } else if meta.path.is_ident("rename") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    rename = Some(value.value());
                } else if meta.path.is_ident("enum_type") {
                    enum_type = true;
                } else if meta.path.is_ident("variants") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    enum_variants = Some(
                        value
                            .value()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect(),
                    );
                }
                Ok(())
            })?;
        }
    }

    Ok(ConfigFieldAttrs {
        skip,
        no_signal,
        rename,
        enum_type,
        enum_variants,
    })
}

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let struct_attrs = parse_struct_attrs(&input.attrs)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "ConfigProperties only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ConfigProperties can only be applied to structs",
            ))
        }
    };

    let mut metadata_entries = Vec::new();
    let prefix = &struct_attrs.prefix;

    metadata_entries.push(quote! {
        niri_lua_traits::PropertyMetadata::new(
            #prefix,
            niri_lua_traits::PropertyType::Nested,
        )
    });

    for field in fields {
        let field_attrs = parse_field_attrs(&field.attrs)?;
        if field_attrs.skip {
            continue;
        }

        let field_name = field.ident.as_ref().unwrap();
        let lua_name = field_attrs
            .rename
            .clone()
            .unwrap_or_else(|| field_name.to_string());
        let full_path = format!("{}.{}", prefix, lua_name);
        let field_ty = &field.ty;

        let prop_type = if field_attrs.enum_type {
            let variants = field_attrs.enum_variants.unwrap_or_default();
            let variant_strs: Vec<_> = variants.iter().map(|v| quote! { #v }).collect();
            let type_name = get_type_name(field_ty);
            quote! {
                niri_lua_traits::PropertyType::Enum {
                    name: #type_name,
                    variants: &[#(#variant_strs),*],
                }
            }
        } else if is_option(field_ty) {
            let inner_ty = get_inner_type(field_ty)?;
            get_property_type(&inner_ty)
        } else if is_vec(field_ty) {
            let inner_ty = get_inner_type(field_ty)?;
            let inner_prop_type = get_property_type(&inner_ty);
            quote! { niri_lua_traits::PropertyType::Array(Box::new(#inner_prop_type)) }
        } else if is_struct_type(field_ty) {
            quote! { niri_lua_traits::PropertyType::Nested }
        } else {
            get_property_type(field_ty)
        };

        let metadata = if field_attrs.no_signal {
            quote! {
                niri_lua_traits::PropertyMetadata::new(#full_path, #prop_type).with_no_signal()
            }
        } else {
            quote! {
                niri_lua_traits::PropertyMetadata::new(#full_path, #prop_type)
            }
        };

        metadata_entries.push(metadata);
    }

    Ok(quote! {
        impl niri_lua_traits::ConfigProperties for #struct_name {
            fn property_metadata() -> Vec<niri_lua_traits::PropertyMetadata> {
                vec![
                    #(#metadata_entries),*
                ]
            }
        }
    })
}

fn get_property_type(ty: &Type) -> TokenStream2 {
    match get_type_name(ty).as_str() {
        "bool" => quote! { niri_lua_traits::PropertyType::Bool },
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "usize" | "isize" => {
            quote! { niri_lua_traits::PropertyType::Integer }
        }
        "f32" | "f64" | "FloatOrInt" => quote! { niri_lua_traits::PropertyType::Number },
        "String" => quote! { niri_lua_traits::PropertyType::String },
        _ => quote! { niri_lua_traits::PropertyType::Nested },
    }
}

fn get_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn is_option(ty: &Type) -> bool {
    get_type_name(ty) == "Option"
}

fn is_vec(ty: &Type) -> bool {
    get_type_name(ty) == "Vec"
}

fn is_struct_type(ty: &Type) -> bool {
    let name = get_type_name(ty);
    !matches!(
        name.as_str(),
        "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "usize"
            | "isize"
            | "f32"
            | "f64"
            | "FloatOrInt"
            | "String"
            | "Option"
            | "Vec"
    ) && !name.is_empty()
}

fn get_inner_type(ty: &Type) -> syn::Result<Type> {
    if let Type::Path(path) = ty {
        if let Some(seg) = path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Ok(inner.clone());
                }
            }
        }
    }
    Err(syn::Error::new_spanned(ty, "expected generic type"))
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_simple_struct() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "cursor")]
            struct Cursor {
                pub xcursor_theme: String,
                pub xcursor_size: u8,
                pub hide_when_typing: bool,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("cursor"));
        assert!(tokens.contains("PropertyType :: String"));
        assert!(tokens.contains("PropertyType :: Integer"));
        assert!(tokens.contains("PropertyType :: Bool"));
    }

    #[test]
    fn test_option_field() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "test")]
            struct Test {
                pub value: Option<u32>,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("test.value"));
        assert!(tokens.contains("PropertyType :: Integer"));
    }

    #[test]
    fn test_vec_field() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "test")]
            struct Test {
                pub items: Vec<String>,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("PropertyType :: Array"));
    }

    #[test]
    fn test_skip_field() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "test")]
            struct Test {
                pub visible: bool,
                #[config(skip)]
                pub hidden: String,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("test.visible"));
        assert!(!tokens.contains("test.hidden"));
    }

    #[test]
    fn test_rename_field() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "test")]
            struct Test {
                #[config(rename = "custom_name")]
                pub original: bool,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("test.custom_name"));
        assert!(!tokens.contains("test.original"));
    }

    #[test]
    fn test_no_signal() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "test")]
            struct Test {
                #[config(no_signal)]
                pub quiet: bool,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("with_no_signal"));
    }

    #[test]
    fn test_enum_type() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "test")]
            struct Test {
                #[config(enum_type, variants = "left, center, right")]
                pub alignment: Alignment,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("PropertyType :: Enum"));
    }

    #[test]
    fn test_nested_struct() {
        let input: DeriveInput = parse_quote! {
            #[config(prefix = "layout")]
            struct Layout {
                pub gaps: Gaps,
            }
        };

        let result = derive(input);
        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("layout.gaps"));
        assert!(tokens.contains("PropertyType :: Nested"));
    }
}
