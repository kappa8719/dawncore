use std::{fs::File, io::Write};

use lapi::league::{TypeSpec, TypeSpecDetail};
use quote::quote;

pub fn write_types(file: &mut File, types: Vec<TypeSpec>) {
    let mut base = quote! {};

    for spec in types {
        let ident = spec.identifier();

        let generated = match spec.detail {
            TypeSpecDetail::Object { fields } => {
                quote! {
                    pub struct #ident;
                }
            }
            TypeSpecDetail::Enum { entries } => {
                quote! {
                    pub struct #ident;
                }
            }
            TypeSpecDetail::Unit => {
                quote! {
                    pub struct #ident;
                }
            }
        };

        base.extend(Some(generated));
    }

    let parsed = syn::parse_file(base.to_string().as_str()).unwrap();
    let formatted = prettyplease::unparse(&parsed);
    file.write_all(formatted.as_bytes()).unwrap();
}
