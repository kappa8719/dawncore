use std::{fs::File, io::Write};

use lapi::league::{AuthenticatedHttp, FunctionSpec, TypeSpec, TypeSpecDetail};
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
                        let serial_name = v.name.as_str();
                        let ident = v.ident();
                        let name = Ident::new(ident.as_str(), Span::call_site());
                        let ty = type_path_to_type(v.ty.to_rust_type(false), v.optional);

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

                let name_match_arms = entries
                    .iter()
                    .map(|v| {
                        let ident = Ident::new(v.ident().as_str(), Span::call_site());
                        let name = v.name.as_str();

                        quote! {
                            Self::#ident => #name,
                        }
                    })
                    .collect::<Vec<_>>();

                quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize)]
                    pub enum #ident {
                        #(#entry_defs)*
                    }

                    impl #ident {
                        pub fn name(&self) -> &'static str {
                            match self {
                                #(#name_match_arms)*
                            }
                        }
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

pub fn write_functions(mut write: impl Write, functions: &Vec<FunctionSpec>) {
    let mut base = quote! {
        use crate::AuthenticatedHttp as __AuthenticatedHttp;
    };

    for function in functions {
        let arg_defs = function
            .arguments
            .iter()
            .map(|v| {
                let name = Ident::new(v.ident().as_str(), Span::call_site());
                let ty = type_path_to_type(v.ty.to_rust_type(false), v.optional);

                quote! {
                    #name: #ty
                }
            })
            .collect::<Vec<_>>();
        let ident = Ident::new(function.ident().as_str(), Span::call_site());
        let returns_path = match &function.returns {
            Some(returns) => returns.to_rust_type(false),
            None => "()".to_owned(),
        };
        let returns = type_path_to_type(returns_path, false);

        let url = if function.arguments.iter().filter(|v| v.parameter).count() > 0 {
            let mut url = function.url.clone();
            for parameter in function.arguments.iter().filter(|v| v.parameter) {
                url = url.replace(
                    format!("{{{}}}", parameter.name).as_str(),
                    format!("{{{}}}", parameter.ident()).as_str(),
                );
            }
            quote! {
                format!(#url).as_str()
            }
        } else {
            let url = function.url.as_str();

            quote! {
                #url
            }
        };

        let builder_def = match function.method {
            lapi::league::FunctionMethod::Get => {
                quote! { let builder = client.get(client.host.join(#url).unwrap()); }
            }
            lapi::league::FunctionMethod::Post => {
                quote! { let builder = client.post(client.host.join(#url).unwrap()); }
            }
            lapi::league::FunctionMethod::Put => {
                quote! { let builder = client.put(client.host.join(#url).unwrap()); }
            }
            lapi::league::FunctionMethod::Patch => {
                quote! { let builder = client.patch(client.host.join(#url).unwrap()); }
            }
            lapi::league::FunctionMethod::Delete => {
                quote! { let builder = client.delete(client.host.join(#url).unwrap()); }
            }
        };

        let function_def = quote! {
            pub async fn #ident(client: __AuthenticatedHttp, #(#arg_defs),*) -> reqwest::Result<#returns> {
                #builder_def

                todo!()
            }
        };

        base.extend(function_def);
    }

    let parsed = syn::parse_file(base.to_string().as_str()).unwrap();
    let formatted = prettyplease::unparse(&parsed);
    write.write_all(formatted.as_bytes()).unwrap();
}

fn type_path_to_type(path: String, optional: bool) -> syn::Type {
    let path = if optional {
        format!("std::option::Option<{}>", path)
    } else {
        path
    };

    syn::parse_str::<syn::Type>(path.as_str()).unwrap()
}

pub async fn get_client_config_v_1_status_by_type(
    client: AuthenticatedHttp,
    type_: u16,
) -> reqwest::Result<()> {
    todo!()
}
