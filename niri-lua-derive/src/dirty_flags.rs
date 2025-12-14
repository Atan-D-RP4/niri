//! DirtyFlags derive macro implementation.
//!
//! Generates a DirtyFlag enum and helper methods from a struct with boolean fields.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Get struct fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "DirtyFlags only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "DirtyFlags can only be applied to structs",
            ));
        }
    };

    // Collect field names and generate enum variants
    let mut field_names = Vec::new();
    let mut enum_variants = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();

        // Verify field is bool type
        if !is_bool_type(&field.ty) {
            return Err(syn::Error::new_spanned(
                field,
                "DirtyFlags struct fields must be of type bool",
            ));
        }

        // Convert snake_case field to PascalCase variant
        let variant_name = to_pascal_case(&field_name.to_string());
        let variant_ident = format_ident!("{}", variant_name);

        field_names.push(field_name.clone());
        enum_variants.push(variant_ident);
    }

    // Generate enum name (remove "Flags" suffix if present, add "Flag")
    let enum_name = generate_enum_name(struct_name);

    // Generate the mark() match arms
    let mark_arms: Vec<_> = field_names
        .iter()
        .zip(enum_variants.iter())
        .map(|(field, variant)| {
            quote! {
                #enum_name::#variant => self.#field = true,
            }
        })
        .collect();

    // Generate the is_dirty() match arms
    let is_dirty_arms: Vec<_> = field_names
        .iter()
        .zip(enum_variants.iter())
        .map(|(field, variant)| {
            quote! {
                #enum_name::#variant => self.#field,
            }
        })
        .collect();

    // Generate the any() check
    let any_checks: Vec<_> = field_names
        .iter()
        .map(|field| quote! { self.#field })
        .collect();

    // Generate the clear() assignments
    let clear_assignments: Vec<_> = field_names
        .iter()
        .map(|field| quote! { self.#field = false; })
        .collect();

    // Generate the clear_flag() match arms
    let clear_flag_arms: Vec<_> = field_names
        .iter()
        .zip(enum_variants.iter())
        .map(|(field, variant)| {
            quote! {
                #enum_name::#variant => self.#field = false,
            }
        })
        .collect();

    // Generate iterator items for dirty_flags()
    let dirty_flags_items: Vec<_> = field_names
        .iter()
        .zip(enum_variants.iter())
        .map(|(field, variant)| {
            quote! {
                if self.#field {
                    result.push(#enum_name::#variant);
                }
            }
        })
        .collect();

    let expanded = quote! {
        /// Enum representing individual dirty flags.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #enum_name {
            #(#enum_variants,)*
        }

        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Create a new instance with all flags cleared.
            pub fn new() -> Self {
                Self::default()
            }

            /// Mark a specific flag as dirty.
            pub fn mark(&mut self, flag: #enum_name) {
                match flag {
                    #(#mark_arms)*
                }
            }

            /// Check if a specific flag is dirty.
            pub fn is_dirty(&self, flag: #enum_name) -> bool {
                match flag {
                    #(#is_dirty_arms)*
                }
            }

            /// Check if any flag is dirty.
            pub fn any(&self) -> bool {
                #(#any_checks)||*
            }

            /// Clear all dirty flags.
            pub fn clear(&mut self) {
                #(#clear_assignments)*
            }

            /// Clear a specific flag.
            pub fn clear_flag(&mut self, flag: #enum_name) {
                match flag {
                    #(#clear_flag_arms)*
                }
            }

            /// Get all currently dirty flags.
            pub fn dirty_flags(&self) -> ::std::vec::Vec<#enum_name> {
                let mut result = ::std::vec::Vec::new();
                #(#dirty_flags_items)*
                result
            }
        }

        impl #impl_generics Default for #struct_name #ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #(#field_names: false,)*
                }
            }
        }
    };

    Ok(expanded)
}

/// Check if a type is bool
fn is_bool_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Generate enum name from struct name
fn generate_enum_name(struct_name: &syn::Ident) -> syn::Ident {
    let name = struct_name.to_string();

    // Remove "Flags" suffix if present, add "Flag"
    let enum_name = if name.ends_with("Flags") {
        format!("{}Flag", &name[..name.len() - 5]) // ConfigDirtyFlags -> ConfigDirtyFlag
    } else if name.ends_with("s") {
        format!("{}Flag", &name[..name.len() - 1]) // ConfigDirtys -> ConfigDirtyFlag
    } else {
        format!("{}Flag", name) // ConfigDirty -> ConfigDirtyFlag
    };

    format_ident!("{}", enum_name)
}
