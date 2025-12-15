//! Collection proxy generation for Vec<T> fields.
//!
//! Generates proxy structs that provide Lua table-like access to Vec<T> config fields.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Ident, Type};

use crate::attributes::StructAttrs;

/// Information about a collection field
pub struct CollectionFieldInfo {
    pub field_name: Ident,
    pub item_type: Type,
    pub dirty_flag: Ident,
}

/// Generate a collection proxy struct and its implementation
pub fn generate_collection_proxy(
    field: &CollectionFieldInfo,
    struct_attrs: &StructAttrs,
) -> TokenStream2 {
    let field_name = &field.field_name;
    let item_type = &field.item_type;
    let dirty_flag = &field.dirty_flag;

    // Generate proxy name: Vec{ItemType}Proxy
    let item_type_name = get_type_name(item_type);
    let proxy_name = format_ident!("Vec{}Proxy", item_type_name);

    // Generate access path
    let access_path = generate_access_path(struct_attrs, field_name);

    // Get the crate path (either niri_lua or a custom path like crate)
    let crate_path = struct_attrs.get_crate_path();

    quote! {
        /// Collection proxy for accessing Vec<#item_type> configuration.
        ///
        /// This proxy provides table-like access to a Vec field with 1-based indexing
        /// (following Lua conventions). All index parameters from Lua start at 1, not 0.
        #[derive(Clone)]
        pub struct #proxy_name {
            state: #crate_path::config_state::ConfigState,
        }

        impl #proxy_name {
            /// Create a new collection proxy.
            pub fn new(state: #crate_path::config_state::ConfigState) -> Self {
                Self { state }
            }

            /// Get the length of the collection.
            pub fn len(&self) -> usize {
                let config = self.state.borrow_config();
                #access_path.len()
            }

            /// Check if the collection is empty.
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            /// Get an item by index (0-based internally, 1-based from Lua).
            pub fn get(&self, index: usize, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Value> {
                use ::mlua::IntoLua;
                let config = self.state.borrow_config();
                if index >= #access_path.len() {
                    return Err(::mlua::Error::external(format!(
                        "Index {} out of bounds (length: {})",
                        index + 1,  // Show 1-based index in error
                        #access_path.len()
                    )));
                }

                // Convert the item to Lua value
                let item = &#access_path[index];
                let lua_val = <#item_type as #crate_path::traits::LuaFieldConvert>::to_lua(item);
                lua_val.into_lua(lua)
            }

            /// Append an item to the collection.
            pub fn append(&self, value: ::mlua::Value, lua: &::mlua::Lua) -> ::mlua::Result<()> {
                // First convert Lua Value to the intermediate LuaType
                let intermediate: <#item_type as #crate_path::traits::LuaFieldConvert>::LuaType =
                    ::mlua::FromLua::from_lua(value, lua)?;
                // Then convert to the actual Rust type
                let new_item = <#item_type as #crate_path::traits::LuaFieldConvert>::from_lua(intermediate)?;

                {
                    let mut config = self.state.borrow_config();
                    #access_path.push(new_item);
                }

                self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }

            /// Remove an item at the given index (1-based from Lua).
            pub fn remove(&self, index: usize) -> ::mlua::Result<()> {
                {
                    let mut config = self.state.borrow_config();
                    if index == 0 || index > #access_path.len() {
                        return Err(::mlua::Error::external(format!(
                            "Index {} out of bounds (length: {})",
                            index,
                            #access_path.len()
                        )));
                    }
                    #access_path.remove(index - 1);  // Convert to 0-based
                }

                self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }

            /// Clear all items from the collection.
            pub fn clear(&self) -> ::mlua::Result<()> {
                {
                    let mut config = self.state.borrow_config();
                    #access_path.clear();
                }

                self.state.mark_dirty(#crate_path::config_state::DirtyFlag::#dirty_flag);
                Ok(())
            }
        }

        impl ::mlua::UserData for #proxy_name {
            fn add_fields<F: ::mlua::UserDataFields<Self>>(_fields: &mut F) {
                // No direct fields - use metamethods for table-like access
            }

            fn add_methods<M: ::mlua::UserDataMethods<Self>>(methods: &mut M) {
                // __len metamethod
                methods.add_meta_method(::mlua::MetaMethod::Len, |_lua, this, ()| {
                    Ok(this.len())
                });

                // __index metamethod (1-based indexing)
                methods.add_meta_method(::mlua::MetaMethod::Index, |lua, this, index: usize| {
                    if index == 0 {
                        return Err(::mlua::Error::external("Lua indices start at 1, not 0"));
                    }
                    this.get(index - 1, lua)  // Convert to 0-based
                });

                // Collection methods
                methods.add_method("append", |lua, this, value: ::mlua::Value| {
                    this.append(value, lua)
                });

                methods.add_method("remove", |_lua, this, index: usize| {
                    this.remove(index)
                });

                methods.add_method("clear", |_lua, this, ()| {
                    this.clear()
                });

                // __iter metamethod for iteration support
                // Luau uses __iter instead of __pairs for iteration
                // The iterator returns (index, value) pairs with 1-based indices
                methods.add_meta_method(::mlua::MetaMethod::Iter, |lua, this, ()| {
                    use ::mlua::IntoLua;
                    let len = this.len();
                    let state = this.state.clone();

                    // Create iterator function that returns next (key, value) pair
                    let iter_fn = lua.create_function(move |lua, (_, prev_idx): (::mlua::Value, Option<i64>)| {
                        use ::mlua::IntoLua;
                        let next_idx = prev_idx.map(|i| i + 1).unwrap_or(1);

                        if next_idx as usize > len || next_idx < 1 {
                            // End of iteration - return nil
                            return Ok((::mlua::Value::Nil, ::mlua::Value::Nil));
                        }

                        // Get item at 0-based index
                        let config = state.borrow_config();
                        let item = &#access_path[(next_idx as usize) - 1];
                        let lua_val = <#item_type as #crate_path::traits::LuaFieldConvert>::to_lua(item);
                        drop(config);

                        Ok((::mlua::Value::Integer(next_idx), lua_val.into_lua(lua)?))
                    })?;

                    // Return (iterator_fn, nil, nil) - standard Lua iteration protocol
                    Ok((iter_fn, ::mlua::Value::Nil, ::mlua::Value::Nil))
                });
            }
        }
    }
}

/// Generate the access path for a collection field
fn generate_access_path(struct_attrs: &StructAttrs, field_name: &Ident) -> TokenStream2 {
    if struct_attrs.is_root {
        quote! { config.#field_name }
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
        quote! { #path.#field_name }
    } else {
        quote! { config.#field_name }
    }
}

/// Get the type name from a Type
fn get_type_name(ty: &Type) -> Ident {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident.clone();
        }
    }
    format_ident!("Unknown")
}
