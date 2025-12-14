//! LuaConfigProxy derive macro implementation.
//!
//! Generates proxy structs with UserData implementation for config structs.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

use crate::attributes::{to_snake_case, FieldAttrs, FieldKind, StructAttrs};
use crate::collection_proxy::{generate_collection_proxy, CollectionFieldInfo};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let proxy_name = format_ident!("{}Proxy", struct_name);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let struct_attrs = StructAttrs::from_attrs(&input.attrs)?;

    // Get struct fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "LuaConfigProxy only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "LuaConfigProxy can only be applied to structs",
            ));
        }
    };

    // Collect field information
    let mut field_infos = Vec::new();
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let field_attrs = FieldAttrs::from_attrs(&field.attrs)?;

        // Skip fields marked with skip
        if matches!(field_attrs.kind, FieldKind::Skip) {
            continue;
        }

        field_infos.push(FieldInfo {
            name: field_name.clone(),
            ty: field_type.clone(),
            attrs: field_attrs,
        });
    }

    // Generate Rust getter/setter methods
    let getter_methods: Vec<TokenStream2> = field_infos
        .iter()
        .map(|f| generate_getter_method(f, &struct_attrs))
        .collect::<syn::Result<Vec<_>>>()?;

    let setter_methods: Vec<TokenStream2> = field_infos
        .iter()
        .filter(|f| !f.attrs.readonly && matches!(f.attrs.kind, FieldKind::Simple))
        .map(|f| generate_setter_method(f, &struct_attrs))
        .collect::<syn::Result<Vec<_>>>()?;

    // Generate UserData field registrations
    let field_registrations: Vec<TokenStream2> = field_infos
        .iter()
        .map(generate_field_registration)
        .collect();

    // Generate collection proxy structs for collection fields
    let collection_proxies: Vec<TokenStream2> = field_infos
        .iter()
        .filter(|f| matches!(f.attrs.kind, FieldKind::Collection))
        .map(|f| {
            // Determine dirty flag for this field
            let dirty_flag = if let Some(ref flag) = f.attrs.dirty_override {
                format_ident!("{}", flag)
            } else if let Some(ref flag) = struct_attrs.dirty_flag {
                format_ident!("{}", flag)
            } else {
                format_ident!("Misc")
            };

            // Extract item type from Vec<T>
            let item_ty_tokens = extract_vec_inner_type(&f.ty);
            let item_ty: Type =
                syn::parse2(item_ty_tokens).expect("Failed to parse Vec inner type");

            let collection_info = CollectionFieldInfo {
                field_name: f.name.clone(),
                item_type: item_ty,
                dirty_flag,
            };

            generate_collection_proxy(&collection_info, &struct_attrs)
        })
        .collect();

    // Generate the proxy struct and implementations
    let expanded = quote! {
        // Collection proxy structs
        #(#collection_proxies)*

        /// Generated Lua proxy for accessing configuration.
        #[derive(Clone)]
        pub struct #proxy_name {
            state: niri_lua::config_state::ConfigState,
        }

        impl #impl_generics #proxy_name #ty_generics #where_clause {
            /// Create a new proxy with the given config state.
            pub fn new(state: niri_lua::config_state::ConfigState) -> Self {
                Self { state }
            }

            /// Get the underlying config state.
            pub fn state(&self) -> &niri_lua::config_state::ConfigState {
                &self.state
            }

            #(#getter_methods)*

            #(#setter_methods)*
        }

        impl #impl_generics ::mlua::UserData for #proxy_name #ty_generics #where_clause {
            fn add_fields<F: ::mlua::UserDataFields<Self>>(fields: &mut F) {
                #(#field_registrations)*
            }
        }
    };

    Ok(expanded)
}

struct FieldInfo {
    name: syn::Ident,
    ty: Type,
    attrs: FieldAttrs,
}

impl FieldInfo {
    fn lua_name(&self) -> String {
        self.attrs
            .lua_name
            .clone()
            .unwrap_or_else(|| to_snake_case(&self.name.to_string()))
    }

    fn getter_name(&self) -> syn::Ident {
        format_ident!("get_{}", self.name)
    }

    fn setter_name(&self) -> syn::Ident {
        format_ident!("set_{}", self.name)
    }
}

/// Generate the access path for field access based on struct configuration
fn generate_access_path(struct_attrs: &StructAttrs) -> TokenStream2 {
    if struct_attrs.is_root {
        quote! { config }
    } else if let Some(ref parent_path) = struct_attrs.parent_path {
        let path_parts: Vec<_> = parent_path
            .split('.')
            .map(|s| format_ident!("{}", s))
            .collect();
        quote! { config.#(#path_parts).* }
    } else {
        quote! { config }
    }
}

/// Generate a Rust getter method for a field
fn generate_getter_method(
    field: &FieldInfo,
    struct_attrs: &StructAttrs,
) -> syn::Result<TokenStream2> {
    let getter_name = field.getter_name();
    let field_name = &field.name;
    let field_ty = &field.ty;
    let access_path = generate_access_path(struct_attrs);

    match field.attrs.kind {
        FieldKind::Simple => {
            if is_option_type(field_ty) {
                let inner_ty = get_option_inner_type(field_ty);
                Ok(quote! {
                    /// Get the value of this field, returning nil if None.
                    pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Value> {
                        let config = self.state.borrow_config();
                        match &#access_path.#field_name {
                            Some(v) => {
                                let lua_val = <#inner_ty as niri_lua::traits::LuaFieldConvert>::to_lua(v);
                                lua_val.into_lua(lua)
                            }
                            None => Ok(::mlua::Value::Nil),
                        }
                    }
                })
            } else {
                Ok(quote! {
                    /// Get the value of this field.
                    pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Value> {
                        let config = self.state.borrow_config();
                        let lua_val = <#field_ty as niri_lua::traits::LuaFieldConvert>::to_lua(&#access_path.#field_name);
                        lua_val.into_lua(lua)
                    }
                })
            }
        }
        FieldKind::Nested => {
            // Extract the actual type name, handling Option<T>
            let inner_ty = if is_option_type(field_ty) {
                get_option_inner_type(field_ty)
            } else {
                quote! { #field_ty }
            };
            let nested_proxy = format_ident!("{}Proxy", get_type_name_from_tokens(&inner_ty));

            Ok(quote! {
                /// Get a proxy for the nested configuration.
                pub fn #getter_name(&self) -> #nested_proxy {
                    #nested_proxy::new(self.state.clone())
                }
            })
        }
        FieldKind::Collection => {
            // Extract item type from Vec<T>
            let item_ty = extract_vec_inner_type(field_ty);
            let item_type_name = get_type_name_from_tokens(&item_ty);
            let proxy_name = format_ident!("Vec{}Proxy", item_type_name);

            Ok(quote! {
                /// Get a proxy for the collection.
                pub fn #getter_name(&self) -> #proxy_name {
                    #proxy_name::new(self.state.clone())
                }
            })
        }
        FieldKind::Skip => Ok(quote! {}),
    }
}

/// Generate a Rust setter method for a field
fn generate_setter_method(
    field: &FieldInfo,
    struct_attrs: &StructAttrs,
) -> syn::Result<TokenStream2> {
    let setter_name = field.setter_name();
    let field_name = &field.name;
    let field_ty = &field.ty;
    let access_path = generate_access_path(struct_attrs);

    // Determine dirty flag
    let dirty_flag = if let Some(ref flag) = field.attrs.dirty_override {
        format_ident!("{}", flag)
    } else if let Some(ref flag) = struct_attrs.dirty_flag {
        format_ident!("{}", flag)
    } else {
        format_ident!("Misc")
    };

    if is_option_type(field_ty) {
        let inner_ty = get_option_inner_type(field_ty);
        Ok(quote! {
            /// Set the value of this field. Pass nil to clear.
            pub fn #setter_name(&self, lua: &::mlua::Lua, value: ::mlua::Value) -> ::mlua::Result<()> {
                let new_value = if value.is_nil() {
                    None
                } else {
                    // First convert Lua Value to the intermediate LuaType
                    let intermediate: <#inner_ty as niri_lua::traits::LuaFieldConvert>::LuaType = 
                        ::mlua::FromLua::from_lua(value, lua)?;
                    // Then convert to the actual Rust type
                    Some(<#inner_ty as niri_lua::traits::LuaFieldConvert>::from_lua(intermediate)?)
                };

                // Explicit scope to release borrow before mark_dirty
                {
                    let mut config = self.state.borrow_config();
                    #access_path.#field_name = new_value;
                }

                self.state.mark_dirty(niri_lua::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }
        })
    } else {
        Ok(quote! {
            /// Set the value of this field.
            pub fn #setter_name(&self, value: <#field_ty as niri_lua::traits::LuaFieldConvert>::LuaType) -> ::mlua::Result<()> {
                let new_value = <#field_ty as niri_lua::traits::LuaFieldConvert>::from_lua(value)?;

                // Explicit scope to release borrow before mark_dirty
                {
                    let mut config = self.state.borrow_config();
                    #access_path.#field_name = new_value;
                }

                self.state.mark_dirty(niri_lua::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }
        })
    }
}

/// Generate UserData field registration for a field
fn generate_field_registration(field: &FieldInfo) -> TokenStream2 {
    let lua_name = field.lua_name();
    let getter_name = field.getter_name();
    let setter_name = field.setter_name();
    let field_ty = &field.ty;

    match field.attrs.kind {
        FieldKind::Simple => {
            if field.attrs.readonly {
                // Read-only field
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
                    });
                }
            } else {
                // Read-write field - handle Option types differently
                // Option setter needs lua context for nil handling
                // Non-Option setter doesn't need lua context
                if is_option_type(field_ty) {
                    quote! {
                        fields.add_field_method_get(#lua_name, |lua, this| {
                            this.#getter_name(lua)
                        });
                        fields.add_field_method_set(#lua_name, |lua, this, value| {
                            this.#setter_name(lua, value)
                        });
                    }
                } else {
                    quote! {
                        fields.add_field_method_get(#lua_name, |lua, this| {
                            this.#getter_name(lua)
                        });
                        fields.add_field_method_set(#lua_name, |_lua, this, value| {
                            this.#setter_name(value)
                        });
                    }
                }
            }
        }
        FieldKind::Nested => {
            // Nested proxies are read-only (return child proxy)
            quote! {
                fields.add_field_method_get(#lua_name, |_lua, this| {
                    Ok(this.#getter_name())
                });
            }
        }
        FieldKind::Collection => {
            // Collection fields return a proxy
            quote! {
                fields.add_field_method_get(#lua_name, |_lua, this| {
                    Ok(this.#getter_name())
                });
            }
        }
        FieldKind::Skip => quote! {},
    }
}

// ============================================================================
// Type analysis utilities
// ============================================================================

/// Check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Extract the inner type T from Option<T> as a TokenStream
fn get_option_inner_type(ty: &Type) -> TokenStream2 {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return quote! { #inner_ty };
                    }
                }
            }
        }
    }
    quote! { () }
}

/// Extract the inner type T from Vec<T> as a TokenStream
fn extract_vec_inner_type(ty: &Type) -> TokenStream2 {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return quote! { #inner_ty };
                    }
                }
            }
        }
    }
    quote! { () }
}

/// Get the type name from a TokenStream (for generating proxy names)
fn get_type_name_from_tokens(tokens: &TokenStream2) -> syn::Ident {
    // Parse as a Type and extract the name
    if let Ok(ty) = syn::parse2::<Type>(tokens.clone()) {
        return get_type_name(&ty);
    }
    format_ident!("Unknown")
}

/// Get the simple type name from a Type
fn get_type_name(ty: &Type) -> syn::Ident {
    // First unwrap Option if present
    let inner = if is_option_type(ty) {
        let inner_tokens = get_option_inner_type(ty);
        if let Ok(inner_ty) = syn::parse2::<Type>(inner_tokens) {
            return get_type_name(&inner_ty);
        }
        ty
    } else {
        ty
    };

    if let Type::Path(type_path) = inner {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident.clone();
        }
    }
    format_ident!("Unknown")
}
