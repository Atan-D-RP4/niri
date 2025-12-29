use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExtractKind {
    String,
    Bool,
    Int,
    Float,
    Color,
    Nested,
    Vec,
    Skip,
}

struct FieldInfo {
    name: Ident,
    lua_name: String,
    kind: ExtractKind,
    inner_type: Option<Type>,
}

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("FromLuaTable only supports structs with named fields"),
        },
        _ => panic!("FromLuaTable only supports structs"),
    };

    let field_infos: Vec<FieldInfo> = fields
        .iter()
        .filter_map(|f| {
            let name = f.ident.clone()?;
            let (kind, lua_name, inner_type) = parse_field_attrs(f, &name);
            if kind == ExtractKind::Skip {
                return None;
            }
            Some(FieldInfo {
                name,
                lua_name,
                kind,
                inner_type,
            })
        })
        .collect();

    let extract_vars: Vec<TokenStream2> = field_infos
        .iter()
        .map(|f| {
            let var_name = format_ident!("{}_val", f.name);
            let lua_name = &f.lua_name;
            match f.kind {
                ExtractKind::String => quote! {
                    let #var_name = niri_lua_traits::extract_string_opt(table, #lua_name)?;
                },
                ExtractKind::Bool => quote! {
                    let #var_name = niri_lua_traits::extract_bool_opt(table, #lua_name)?;
                },
                ExtractKind::Int => quote! {
                    let #var_name = niri_lua_traits::extract_int_opt(table, #lua_name)?;
                },
                ExtractKind::Float => quote! {
                    let #var_name = niri_lua_traits::extract_float_opt(table, #lua_name)?;
                },
                ExtractKind::Color => quote! {
                    let #var_name = niri_lua_traits::extract_from_string_opt(table, #lua_name)?;
                },
                ExtractKind::Nested => quote! {
                    let #var_name = niri_lua_traits::extract_table_opt(table, #lua_name)?
                        .map(|t| niri_lua_traits::FromLuaTable::from_lua_table(&t))
                        .transpose()?
                        .flatten();
                },
                ExtractKind::Vec => {
                    let inner = f.inner_type.as_ref().unwrap();
                    quote! {
                        let #var_name = {
                            let seq: Option<mlua::Table> = table.get(#lua_name)?;
                            if let Some(seq) = seq {
                                let mut items = Vec::new();
                                for pair in seq.pairs::<i64, mlua::Table>() {
                                    let (_, t) = pair?;
                                    if let Some(item) = <#inner as niri_lua_traits::FromLuaTable>::from_lua_table(&t)? {
                                        items.push(item);
                                    }
                                }
                                if items.is_empty() { None } else { Some(items) }
                            } else {
                                None
                            }
                        };
                    }
                }
                ExtractKind::Skip => unreachable!(),
            }
        })
        .collect();

    let has_any_checks: Vec<TokenStream2> = field_infos
        .iter()
        .map(|f| {
            let var_name = format_ident!("{}_val", f.name);
            quote! { #var_name.is_some() }
        })
        .collect();

    let field_assignments: Vec<TokenStream2> = field_infos
        .iter()
        .map(|f| {
            let field_name = &f.name;
            let var_name = format_ident!("{}_val", f.name);
            let is_option = is_option_type(
                &fields
                    .iter()
                    .find(|fld| fld.ident.as_ref() == Some(field_name))
                    .unwrap()
                    .ty,
            );
            match f.kind {
                ExtractKind::Int => {
                    if is_option {
                        quote! {
                            if let Some(v) = #var_name {
                                result.#field_name = Some(v as _);
                            }
                        }
                    } else {
                        quote! {
                            if let Some(v) = #var_name {
                                result.#field_name = v as _;
                            }
                        }
                    }
                }
                ExtractKind::Float => {
                    if is_option {
                        quote! {
                            if let Some(v) = #var_name {
                                result.#field_name = Some(v as _);
                            }
                        }
                    } else {
                        quote! {
                            if let Some(v) = #var_name {
                                result.#field_name = v as _;
                            }
                        }
                    }
                }
                _ => quote! {
                    if let Some(v) = #var_name {
                        result.#field_name = v;
                    }
                },
            }
        })
        .collect();

    let expanded = quote! {
        impl niri_lua_traits::FromLuaTable for #name {
            fn from_lua_table(table: &mlua::Table) -> mlua::Result<Option<Self>> {
                #(#extract_vars)*

                let has_any = #(#has_any_checks)||*;
                if !has_any {
                    return Ok(None);
                }

                let mut result = Self::default();
                #(#field_assignments)*

                Ok(Some(result))
            }
        }
    };

    TokenStream::from(expanded)
}

fn parse_field_attrs(field: &syn::Field, name: &Ident) -> (ExtractKind, String, Option<Type>) {
    let mut kind = None;
    let mut lua_name = to_kebab_case(&name.to_string());
    let mut inner_type = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("from_lua") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                kind = Some(ExtractKind::Skip);
            } else if meta.path.is_ident("string") {
                kind = Some(ExtractKind::String);
            } else if meta.path.is_ident("bool") {
                kind = Some(ExtractKind::Bool);
            } else if meta.path.is_ident("int") {
                kind = Some(ExtractKind::Int);
            } else if meta.path.is_ident("float") {
                kind = Some(ExtractKind::Float);
            } else if meta.path.is_ident("color") {
                kind = Some(ExtractKind::Color);
            } else if meta.path.is_ident("nested") {
                kind = Some(ExtractKind::Nested);
            } else if meta.path.is_ident("vec") {
                kind = Some(ExtractKind::Vec);
            } else if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                lua_name = lit.value();
            }
            Ok(())
        })
        .ok();
    }

    let kind = kind.unwrap_or_else(|| infer_kind(&field.ty, &mut inner_type));
    (kind, lua_name, inner_type)
}

fn infer_kind(ty: &Type, inner_type: &mut Option<Type>) -> ExtractKind {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    if type_str == "bool" {
        return ExtractKind::Bool;
    }
    if type_str == "String" || type_str == "Option<String>" {
        return ExtractKind::String;
    }
    if type_str.contains("i8")
        || type_str.contains("i16")
        || type_str.contains("i32")
        || type_str.contains("i64")
        || type_str.contains("u8")
        || type_str.contains("u16")
        || type_str.contains("u32")
        || type_str.contains("u64")
        || type_str.contains("usize")
        || type_str.contains("isize")
    {
        return ExtractKind::Int;
    }
    if type_str.contains("f32") || type_str.contains("f64") {
        return ExtractKind::Float;
    }
    if type_str.contains("Color") {
        return ExtractKind::Color;
    }
    if type_str.starts_with("Vec<") {
        if let Type::Path(type_path) = ty {
            if let Some(seg) = type_path.path.segments.last() {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        *inner_type = Some(inner.clone());
                    }
                }
            }
        }
        return ExtractKind::Vec;
    }
    if type_str.starts_with("Option<") {
        let inner_str = &type_str[7..type_str.len() - 1];
        if inner_str == "bool" {
            return ExtractKind::Bool;
        }
        if inner_str == "String" {
            return ExtractKind::String;
        }
        if inner_str.contains("i8")
            || inner_str.contains("i16")
            || inner_str.contains("i32")
            || inner_str.contains("i64")
            || inner_str.contains("u8")
            || inner_str.contains("u16")
            || inner_str.contains("u32")
            || inner_str.contains("u64")
        {
            return ExtractKind::Int;
        }
        if inner_str.contains("f32") || inner_str.contains("f64") {
            return ExtractKind::Float;
        }
        if inner_str.contains("Color") {
            return ExtractKind::Color;
        }
        return ExtractKind::Nested;
    }

    ExtractKind::Nested
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else if c == '_' {
            result.push('-');
        } else {
            result.push(c);
        }
    }
    result
}

fn is_option_type(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string().replace(' ', "");
    type_str.starts_with("Option<")
}
