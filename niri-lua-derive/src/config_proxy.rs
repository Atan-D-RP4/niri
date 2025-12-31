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
        let mut field_attrs = FieldAttrs::from_attrs(&field.attrs)?;
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if matches!(field_attrs.kind, FieldKind::Skip) && field_attrs.has_explicit_kind {
            continue;
        }

        if !field_attrs.has_explicit_kind {
            field_attrs.kind = FieldKind::Simple;
        }

        // Skip fields that were inferred or explicitly set to Skip
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
        .filter(|f| {
            !f.attrs.readonly
                && matches!(
                    f.attrs.kind,
                    FieldKind::Simple
                        | FieldKind::Gradient
                        | FieldKind::Offset
                        | FieldKind::AnimKind
                        | FieldKind::Inverted
                )
        })
        .map(|f| generate_setter_method(f, &struct_attrs))
        .collect::<syn::Result<Vec<_>>>()?;

    let field_registrations: Vec<TokenStream2> = field_infos
        .iter()
        .map(generate_field_registration)
        .collect();

    let tostring_field_names: Vec<String> = field_infos.iter().map(|f| f.lua_name()).collect();

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
    let crate_path = struct_attrs.get_crate_path();
    let expanded = quote! {
        // Collection proxy structs
        #(#collection_proxies)*

        /// Generated Lua proxy for accessing configuration.
        #[derive(Clone)]
        pub struct #proxy_name {
            state: #crate_path::config_state::ConfigState,
        }

        impl #impl_generics #proxy_name #ty_generics #where_clause {
            /// Create a new proxy with the given config state.
            pub fn new(state: #crate_path::config_state::ConfigState) -> Self {
                Self { state }
            }

            /// Get the underlying config state.
            pub fn state(&self) -> &#crate_path::config_state::ConfigState {
                &self.state
            }

            #(#getter_methods)*

            #(#setter_methods)*
        }

        impl #impl_generics ::mlua::UserData for #proxy_name #ty_generics #where_clause {
            fn add_fields<F: ::mlua::UserDataFields<Self>>(fields: &mut F) {
                use ::mlua::IntoLua;
                #(#field_registrations)*
            }

            fn add_methods<M: ::mlua::UserDataMethods<Self>>(methods: &mut M) {
                const FIELD_NAMES: &[&str] = &[#(#tostring_field_names),*];
                methods.add_meta_method(::mlua::MetaMethod::ToString, |lua, this, ()| {
                    use ::mlua::ObjectLike;
                    let table = lua.create_table()?;
                    let ud = lua.create_userdata(this.clone())?;
                    for &name in FIELD_NAMES {
                        if let Ok(val) = ud.get::<::mlua::Value>(name) {
                            table.set(name, val)?;
                        }
                    }
                    let format_fn: ::mlua::Function = lua.globals().get("__niri_format_value")?;
                    format_fn.call::<String>(table)
                });
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
        // Build the path dynamically, handling both identifiers and tuple indices
        let mut path = quote! { config };
        for part in parent_path.split('.') {
            if let Ok(index) = part.parse::<usize>() {
                // Numeric index - use syn::Index for tuple access
                let idx = syn::Index::from(index);
                path = quote! { #path.#idx };
            } else {
                // Regular identifier
                let ident = format_ident!("{}", part);
                path = quote! { #path.#ident };
            }
        }
        path
    } else {
        quote! { config }
    }
}

/// Generate the full field access path, handling custom_path if specified
///
/// If field has custom_path, handles:
/// - ".." to go up one level from parent_path
/// - Regular path segments to descend
///
/// Otherwise uses the default access_path.field_name
fn generate_field_access_path(field: &FieldInfo, struct_attrs: &StructAttrs) -> TokenStream2 {
    let base_path = generate_access_path(struct_attrs);
    let field_name = &field.name;

    if let Some(ref custom_path) = field.attrs.custom_path {
        // Handle ".." prefix to go up one level
        if let Some(rest) = custom_path.strip_prefix("../") {
            // We need to rebuild the parent path without the last segment
            if let Some(ref parent_path) = struct_attrs.parent_path {
                let segments: Vec<&str> = parent_path.split('.').collect();
                if segments.len() > 1 {
                    // Remove last segment and build new path
                    let mut path = quote! { config };
                    for part in &segments[..segments.len() - 1] {
                        if let Ok(index) = part.parse::<usize>() {
                            let idx = syn::Index::from(index);
                            path = quote! { #path.#idx };
                        } else {
                            let ident = format_ident!("{}", part);
                            path = quote! { #path.#ident };
                        }
                    }
                    // Now add the rest of the custom path (after "../")
                    for part in rest.split('.') {
                        if !part.is_empty() {
                            let ident = format_ident!("{}", part);
                            path = quote! { #path.#ident };
                        }
                    }
                    return path;
                }
            }
            // Fallback if parent_path is not set or has only one segment
            let mut path = quote! { config };
            for part in rest.split('.') {
                if !part.is_empty() {
                    let ident = format_ident!("{}", part);
                    path = quote! { #path.#ident };
                }
            }
            path
        } else {
            // Direct custom path from base
            let mut path = base_path;
            for part in custom_path.split('.') {
                if !part.is_empty() {
                    if let Ok(index) = part.parse::<usize>() {
                        let idx = syn::Index::from(index);
                        path = quote! { #path.#idx };
                    } else {
                        let ident = format_ident!("{}", part);
                        path = quote! { #path.#ident };
                    }
                }
            }
            path
        }
    } else {
        // Default: access_path.field_name
        quote! { #base_path.#field_name }
    }
}

/// Generate a Rust getter method for a field
fn generate_getter_method(
    field: &FieldInfo,
    struct_attrs: &StructAttrs,
) -> syn::Result<TokenStream2> {
    let getter_name = field.getter_name();
    let field_ty = &field.ty;
    let field_path = generate_field_access_path(field, struct_attrs);
    let crate_path = struct_attrs.get_crate_path();

    match field.attrs.kind {
        FieldKind::Simple => {
            if is_option_type(field_ty) {
                let inner_ty = get_option_inner_type(field_ty);
                Ok(quote! {
                    pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Value> {
                        use ::mlua::IntoLua;
                        let config = self.state.try_borrow_config()?;
                        match &#field_path {
                            Some(v) => {
                                let lua_val = <#inner_ty as #crate_path::traits::LuaFieldConvert>::to_lua(v);
                                lua_val.into_lua(lua)
                            }
                            None => Ok(::mlua::Value::Nil),
                        }
                    }
                })
            } else {
                Ok(quote! {
                    pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Value> {
                        use ::mlua::IntoLua;
                        let config = self.state.try_borrow_config()?;
                        let lua_val = <#field_ty as #crate_path::traits::LuaFieldConvert>::to_lua(&#field_path);
                        lua_val.into_lua(lua)
                    }
                })
            }
        }
        FieldKind::Nested => {
            let inner_ty = if is_option_type(field_ty) {
                get_option_inner_type(field_ty)
            } else {
                quote! { #field_ty }
            };
            let nested_proxy = format_ident!("{}Proxy", get_type_name_from_tokens(&inner_ty));

            Ok(quote! {
                pub fn #getter_name(&self) -> #nested_proxy {
                    #nested_proxy::new(self.state.clone())
                }
            })
        }
        FieldKind::Collection => {
            let item_ty = extract_vec_inner_type(field_ty);
            let item_type_name = get_type_name_from_tokens(&item_ty);
            let proxy_name = format_ident!("Vec{}Proxy", item_type_name);

            Ok(quote! {
                pub fn #getter_name(&self) -> #proxy_name {
                    #proxy_name::new(self.state.clone())
                }
            })
        }
        FieldKind::Skip => Ok(quote! {}),
        FieldKind::Gradient => {
            if is_option_type(field_ty) {
                Ok(quote! {
                    pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Value> {
                        let config = self.state.try_borrow_config()?;
                        match &#field_path {
                            Some(g) => {
                                let table = #crate_path::traits::gradient_to_table(lua, g)?;
                                Ok(::mlua::Value::Table(table))
                            }
                            None => Ok(::mlua::Value::Nil),
                        }
                    }
                })
            } else {
                Ok(quote! {
                    pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
                        let config = self.state.try_borrow_config()?;
                        #crate_path::traits::gradient_to_table(lua, &#field_path)
                    }
                })
            }
        }
        FieldKind::Offset => Ok(quote! {
            pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
                let config = self.state.try_borrow_config()?;
                #crate_path::traits::offset_to_table(lua, &#field_path)
            }
        }),
        FieldKind::AnimKind => Ok(quote! {
            pub fn #getter_name(&self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
                let config = self.state.try_borrow_config()?;
                #crate_path::traits::anim_kind_to_table(lua, &#field_path)
            }
        }),
        FieldKind::Inverted => Ok(quote! {
            pub fn #getter_name(&self, _lua: &::mlua::Lua) -> ::mlua::Result<bool> {
                let config = self.state.try_borrow_config()?;
                Ok(!#field_path)
            }
        }),
    }
}

/// Generate a Rust setter method for a field
fn generate_setter_method(
    field: &FieldInfo,
    struct_attrs: &StructAttrs,
) -> syn::Result<TokenStream2> {
    let setter_name = field.setter_name();
    let field_ty = &field.ty;
    let field_path = generate_field_access_path(field, struct_attrs);
    let crate_path = struct_attrs.get_crate_path();

    // Determine dirty flag
    let dirty_flag = if let Some(ref flag) = field.attrs.dirty_override {
        format_ident!("{}", flag)
    } else if let Some(ref flag) = struct_attrs.dirty_flag {
        format_ident!("{}", flag)
    } else {
        format_ident!("Misc")
    };

    match field.attrs.kind {
        FieldKind::Simple => {
            if is_option_type(field_ty) {
                let inner_ty = get_option_inner_type(field_ty);
                Ok(quote! {
                    /// Set the value of this field. Pass nil to clear.
                    pub fn #setter_name(&self, lua: &::mlua::Lua, value: ::mlua::Value) -> ::mlua::Result<()> {
                        let new_value = if value.is_nil() {
                            None
                        } else {
                            // First convert Lua Value to the intermediate LuaType
                            let intermediate: <#inner_ty as #crate_path::traits::LuaFieldConvert>::LuaType =
                                ::mlua::FromLua::from_lua(value, lua)?;
                            // Then convert to the actual Rust type
                            Some(<#inner_ty as #crate_path::traits::LuaFieldConvert>::from_lua(intermediate)?)
                        };

                        {
                            let mut config = self.state.try_borrow_config()?;
                            #field_path = new_value;
                        }

                        self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                        Ok(())
                    }
                })
            } else {
                Ok(quote! {
                    pub fn #setter_name(&self, value: <#field_ty as #crate_path::traits::LuaFieldConvert>::LuaType) -> ::mlua::Result<()> {
                        let new_value = <#field_ty as #crate_path::traits::LuaFieldConvert>::from_lua(value)?;

                        {
                            let mut config = self.state.try_borrow_config()?;
                            #field_path = new_value;
                        }

                        self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                        Ok(())
                    }
                })
            }
        }
        FieldKind::Gradient => {
            if is_option_type(field_ty) {
                Ok(quote! {
                    pub fn #setter_name(&self, _lua: &::mlua::Lua, value: ::mlua::Value) -> ::mlua::Result<()> {
                        let new_value = if value.is_nil() {
                            None
                        } else {
                            let table = value.as_table().ok_or_else(|| {
                                ::mlua::Error::external("Expected table or nil for gradient")
                            })?;
                            Some(#crate_path::traits::table_to_gradient(table.clone())?)
                        };

                        {
                            let mut config = self.state.try_borrow_config()?;
                            #field_path = new_value;
                        }

                        self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                        Ok(())
                    }
                })
            } else {
                Ok(quote! {
                    pub fn #setter_name(&self, _lua: &::mlua::Lua, value: ::mlua::Table) -> ::mlua::Result<()> {
                        let new_value = #crate_path::traits::table_to_gradient(value)?;

                        {
                            let mut config = self.state.try_borrow_config()?;
                            #field_path = new_value;
                        }

                        self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                        Ok(())
                    }
                })
            }
        }
        FieldKind::Offset => Ok(quote! {
            pub fn #setter_name(&self, _lua: &::mlua::Lua, value: ::mlua::Table) -> ::mlua::Result<()> {
                let new_value = #crate_path::traits::table_to_offset(value)?;

                {
                    let mut config = self.state.try_borrow_config()?;
                    #field_path = new_value;
                }

                self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }
        }),
        FieldKind::AnimKind => Ok(quote! {
            pub fn #setter_name(&self, _lua: &::mlua::Lua, value: ::mlua::Table) -> ::mlua::Result<()> {
                let new_value = #crate_path::traits::table_to_anim_kind(value)?;

                {
                    let mut config = self.state.try_borrow_config()?;
                    #field_path = new_value;
                }

                self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }
        }),
        FieldKind::Inverted => Ok(quote! {
            pub fn #setter_name(&self, value: bool) -> ::mlua::Result<()> {
                {
                    let mut config = self.state.try_borrow_config()?;
                    #field_path = !value;
                }

                self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }
        }),
        FieldKind::Nested | FieldKind::Collection | FieldKind::Skip => {
            // These don't have setters
            Ok(quote! {})
        }
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
        FieldKind::Gradient => {
            // Gradient fields use table getter/setter
            if field.attrs.readonly {
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
                    });
                }
            } else {
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
                    });
                    fields.add_field_method_set(#lua_name, |lua, this, value| {
                        this.#setter_name(lua, value)
                    });
                }
            }
        }
        FieldKind::Offset => {
            // Offset fields use table getter/setter
            if field.attrs.readonly {
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
                    });
                }
            } else {
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
                    });
                    fields.add_field_method_set(#lua_name, |lua, this, value| {
                        this.#setter_name(lua, value)
                    });
                }
            }
        }
        FieldKind::AnimKind => {
            // Animation kind fields use table getter/setter
            if field.attrs.readonly {
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
                    });
                }
            } else {
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
                    });
                    fields.add_field_method_set(#lua_name, |lua, this, value| {
                        this.#setter_name(lua, value)
                    });
                }
            }
        }
        FieldKind::Inverted => {
            // Inverted boolean fields
            if field.attrs.readonly {
                quote! {
                    fields.add_field_method_get(#lua_name, |lua, this| {
                        this.#getter_name(lua)
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
