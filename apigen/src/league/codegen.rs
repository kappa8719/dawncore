use std::{fs::File, io::Write};

use lapi::league::{FunctionSpec, TypeSpec, TypeSpecDetail};
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

pub fn write_types(file: &mut File, types: &[TypeSpec]) {
    let mut base = quote! {
        use serde::{Serialize, Deserialize};
    };

    let mut types = types.to_vec();
    types.sort_by_key(|v| match &v.detail {
        TypeSpecDetail::Object { .. } => 0,
        TypeSpecDetail::Enum { .. } => 1,
        TypeSpecDetail::Unit => 2,
    });

    for spec in types {
        let ident = spec.identifier();
        let ident = Ident::new(ident.as_str(), Span::call_site());

        let generated = match &spec.detail {
            TypeSpecDetail::Object { fields } => {
                let field_defs = fields
                    .iter()
                    .map(|v| {
                        let type_path = if v.optional {
                            format!("std::option::Option<{}>", v.ty.to_rust_type(false))
                        } else {
                            v.ty.to_rust_type(false)
                        };

                        let serial_name = v.name.as_str();
                        let ident = v.ident();
                        let name = Ident::new(ident.as_str(), Span::call_site());
                        let ty = syn::parse_str::<syn::Type>(type_path.as_str()).unwrap();

                        let attr_serde_rename = if ident.as_str() == serial_name {
                            None
                        } else {
                            Some(quote! {
                                #[serde(rename = #serial_name)]
                            })
                        };

                        quote! {
                            #attr_serde_rename
                            pub #name: #ty,
                        }
                    })
                    .collect::<Vec<_>>();

                quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize)]
                    pub struct #ident {
                        #(#field_defs)*
                    }
                }
            }
            TypeSpecDetail::Enum { entries } => {
                let mut entries = entries.clone();
                entries.sort_by_key(|v| v.value);

                let entry_defs = entries
                    .iter()
                    .map(|v| {
                        let serial_name = v.name.as_str();
                        let name = Ident::new(v.ident().as_str(), Span::call_site());
                        let value = v.value;

                        quote! {
                            #[serde(rename = #serial_name)]
                            #name = #value,
                        }
                    })
                    .collect::<Vec<_>>();

                quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize)]
                    pub enum #ident {
                        #(#entry_defs)*
                    }
                }
            }
            TypeSpecDetail::Unit => {
                quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize)]
                    pub struct #ident;
                }
            }
        };

        base.extend(Some(generated));
    }

    println!("parsing using syn");
    let parsed = syn::parse_file(base.to_string().as_str()).unwrap();
    println!("formatting output");
    let formatted = prettyplease::unparse(&parsed);
    println!("writing to file");
    file.write_all(formatted.as_bytes()).unwrap();
}

pub fn write_functions(write: impl Write, functions: Vec<FunctionSpec>) {
    let mut base = quote! {};

    for function in functions {}
}
