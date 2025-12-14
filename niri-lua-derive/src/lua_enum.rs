//! LuaEnum derive macro implementation.
//!
//! Generates `LuaEnumConvert` and `LuaFieldConvert` implementations for enums.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use crate::attributes::{apply_rename, EnumAttrs, VariantAttrs};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let enum_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let enum_attrs = EnumAttrs::from_attrs(&input.attrs)?;

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "LuaEnum can only be applied to enums",
            ));
        }
    };

    let mut to_string_arms = Vec::new();
    let mut from_string_arms = Vec::new();
    let mut variant_name_literals = Vec::new();

    for variant in variants {
        // Only support unit variants for now
        match &variant.fields {
            Fields::Unit => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "LuaEnum only supports unit variants (no fields)",
                ));
            }
        }

        let variant_attrs = VariantAttrs::from_attrs(&variant.attrs)?;
        let variant_ident = &variant.ident;

        // Determine the string representation
        let string_name = variant_attrs
            .rename
            .clone()
            .unwrap_or_else(|| apply_rename(&variant_ident.to_string(), enum_attrs.rename_all));

        variant_name_literals.push(string_name.clone());

        to_string_arms.push(quote! {
            #enum_name::#variant_ident => #string_name,
        });

        from_string_arms.push(quote! {
            #string_name => ::std::result::Result::Ok(#enum_name::#variant_ident),
        });
    }

    // Build error message with all valid variants
    let variant_list = variant_name_literals.join(", ");
    let error_msg = format!("Expected one of: {}", variant_list);

    let expanded = quote! {
        impl #impl_generics niri_lua::traits::LuaEnumConvert for #enum_name #ty_generics #where_clause {
            fn to_lua_string(&self) -> &'static str {
                match self {
                    #(#to_string_arms)*
                }
            }

            fn from_lua_string(s: &str) -> ::std::result::Result<Self, ::mlua::Error> {
                match s {
                    #(#from_string_arms)*
                    _ => ::std::result::Result::Err(::mlua::Error::external(::std::format!(
                        "Invalid value '{}'. {}",
                        s,
                        #error_msg
                    ))),
                }
            }

            fn variant_names() -> &'static [&'static str] {
                &[#(#variant_name_literals),*]
            }
        }

        impl #impl_generics niri_lua::traits::LuaFieldConvert for #enum_name #ty_generics #where_clause {
            type LuaType = ::std::string::String;

            fn to_lua(&self) -> ::std::string::String {
                <Self as niri_lua::traits::LuaEnumConvert>::to_lua_string(self).to_owned()
            }

            fn from_lua(value: ::std::string::String) -> ::std::result::Result<Self, ::mlua::Error> {
                <Self as niri_lua::traits::LuaEnumConvert>::from_lua_string(&value)
            }
        }
    };

    Ok(expanded)
}
