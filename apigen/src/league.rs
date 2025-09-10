use std::{collections::HashMap, str::FromStr, sync::Arc};

use lapi::league::{
    EnumEntrySpec, EventSpec, FunctionArgumentSpec, FunctionMethod, FunctionSpec, ObjectFieldSpec,
    TypeReference, TypeSpec, TypeSpecDetail,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(dead_code)]
#[derive(Deserialize)]
struct RootHelpResponse {
    events: HashMap<String, String>,
    functions: HashMap<String, String>,
    types: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct Resolved {
    pub events: HashMap<String, EventSpec>,
    pub functions: HashMap<String, FunctionSpec>,
    pub types: HashMap<String, TypeSpec>,
}

/// League api generator
pub struct League {
    host: Url,
    http: lapi::league::AuthenticatedHttp,
}

impl League {
    pub fn new(host: Url, http: lapi::league::AuthenticatedHttp) -> League {
        Self { host, http }
    }

    async fn resolve_root_help(&self) -> Result<RootHelpResponse, reqwest::Error> {
        let response = self
            .http
            .post(self.host.join("/help").unwrap())
            .send()
            .await?;

        let json = response.json::<RootHelpResponse>().await?;

        Ok(json)
    }

    async fn resolve_help(&self, target: &str, format: &str) -> reqwest::Result<serde_json::Value> {
        self.http
            .post(self.host.join("/help").unwrap())
            .query(&[("target", target), ("format", format)])
            .send()
            .await?
            .json()
            .await
    }

    async fn resolve_events(
        &self,
        help: &RootHelpResponse,
    ) -> reqwest::Result<HashMap<String, EventSpec>> {
        let mut map = HashMap::new();

        for (name, _) in help.events.iter() {
            let resolved = self.resolve_event(name.as_str()).await?;
            map.insert(name.clone(), resolved);
        }

        Ok(map)
    }

    async fn resolve_event(&self, name: &str) -> reqwest::Result<EventSpec> {
        let response = self.resolve_help(name, "Full").await?;
        let root = response
            .as_array()
            .unwrap()
            .first()
            .unwrap()
            .as_object()
            .unwrap();

        let name = root.get("name").unwrap().as_str().unwrap().to_string();
        let description = root
            .get("description")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let type_object = root.get("type").unwrap().as_object().unwrap();
        let ty = resolve_type_reference(
            type_object.get("type").unwrap().as_str().unwrap(),
            type_object
                .get("elementType")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );

        Ok(EventSpec {
            name,
            description,
            ty,
            tags: vec![],
        })
    }

    async fn resolve_functions(
        &self,
        help: &RootHelpResponse,
    ) -> reqwest::Result<HashMap<String, FunctionSpec>> {
        let mut map = HashMap::new();

        for (name, _) in help.functions.iter() {
            let resolved = self.resolve_function(name.as_str()).await?;
            map.insert(name.clone(), resolved);
        }

        Ok(map)
    }

    async fn resolve_function(&self, name: &str) -> reqwest::Result<FunctionSpec> {
        let response_full = self.resolve_help(name, "Full").await?;
        let response_console = self.resolve_help(name, "Console").await?;

        let console_root = response_console.get(name).unwrap().as_object().unwrap();
        let full_root = response_full.get(0).unwrap().as_object().unwrap();
        let name = name.to_string();
        let description = full_root
            .get("description")
            .unwrap()
            .as_str()
            .and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                }
            });
        let method = console_root
            .get("http_method")
            .and_then(|v| v.as_str())
            .and_then(|v| FunctionMethod::from_str(v).ok())
            .unwrap_or(FunctionMethod::Post);
        let url = console_root
            .get("url")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
            .unwrap_or(format!("/{name}"));
        let privilege = console_root.get("privilege").and_then(|v| v.as_u64());
        let returns = console_root
            .get("returns")
            .and_then(|v| v.as_object())
            .and_then(|v| v.keys().next_back())
            .map(|v| resolve_type_reference(v.as_str(), ""));
        let thread_safe = full_root
            .get("threadSafe")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let arguments = full_root
            .get("arguments")
            .and_then(|v| v.as_array())
            .map(|v| {
                v.iter()
                    .filter_map(|v| v.as_object())
                    .map(|v| {
                        let type_object = v.get("type").unwrap().as_object().unwrap();
                        FunctionArgumentSpec {
                            name: v.get("name").unwrap().as_str().unwrap().to_string(),
                            description: v
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(|v| v.to_string()),
                            optional: v.get("optional").and_then(|v| v.as_bool()).unwrap_or(false),
                            ty: resolve_type_reference(
                                type_object.get("type").unwrap().as_str().unwrap(),
                                type_object
                                    .get("elementType")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(""),
                            ),
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or(vec![]);

        Ok(FunctionSpec {
            arguments,
            name,
            description,
            method,
            url,
            privilege,
            returns,
            thread_safe,
            tags: vec![],
        })
    }

    async fn resolve_types(
        &self,
        help: &RootHelpResponse,
    ) -> Result<HashMap<String, TypeSpec>, reqwest::Error> {
        let mut type_specs = HashMap::new();

        for (name, _) in help.types.iter() {
            let name = name.clone();
            let resolved = self.resolve_type(name.as_str()).await?;
            type_specs.insert(name, resolved);
        }

        Ok(type_specs)
    }

    async fn resolve_type(&self, name: &str) -> Result<TypeSpec, reqwest::Error> {
        let response = self.resolve_help(name, "Full").await?;
        let map = response
            .as_array()
            .unwrap()
            .first()
            .unwrap()
            .as_object()
            .unwrap();

        let name = map
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let description = map
            .get("description")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        let fields = map.get("fields").and_then(|v| v.as_array());
        let values = map.get("values").and_then(|v| v.as_array());
        let size = map.get("size").and_then(|v| v.as_u64());

        let spec_detail = if let Some(fields) = fields
            && !fields.is_empty()
        {
            let field_specs = fields
                .iter()
                .filter_map(object_field_spec_from_value)
                .collect::<Vec<_>>();

            TypeSpecDetail::Object {
                fields: field_specs,
            }
        } else if let Some(values) = values
            && !values.is_empty()
        {
            let entry_specs = values
                .iter()
                .filter_map(enum_entry_spec_from_value)
                .collect::<Vec<_>>();

            TypeSpecDetail::Enum {
                entries: entry_specs,
            }
        } else {
            TypeSpecDetail::Unit
        };

        let spec = TypeSpec {
            name,
            description,
            size,
            detail: spec_detail,
            tags: vec![],
        };

        Ok(spec)
    }

    pub async fn resolve(&self) -> reqwest::Result<Resolved> {
        let root_help = self.resolve_root_help().await.unwrap();
        let mut types = self.resolve_types(&root_help).await?;
        let mut functions = self.resolve_functions(&root_help).await?;
        let mut events = self.resolve_events(&root_help).await?;

        let types_map = types.clone();

        for (_, resolved) in types.iter_mut() {
            let TypeSpecDetail::Object { fields } = &mut resolved.detail else {
                continue;
            };

            for field in fields.iter_mut() {
                resolve_unresolved_types(&types_map, &mut field.ty);
            }
        }

        for (_, resolved) in functions.iter_mut() {
            for argument in resolved.arguments.iter_mut() {
                resolve_unresolved_types(&types_map, &mut argument.ty);
            }

            if let Some(returns) = &mut resolved.returns {
                resolve_unresolved_types(&types_map, returns);
            }
        }

        for (_, resolved) in events.iter_mut() {
            resolve_unresolved_types(&types_map, &mut resolved.ty);
        }

        Ok(Resolved {
            events,
            types,
            functions,
        })
    }
}

fn object_field_spec_from_value(value: &Value) -> Option<ObjectFieldSpec> {
    let object = value.as_object()?;

    let name = object.get("name")?.as_str()?;
    let description = object.get("description").and_then(|v| v.as_str());
    let offset = object.get("offset")?.as_u64()?;
    let optional = object
        .get("optional")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let type_object = object.get("type")?.as_object()?;
    let type_element = type_object.get("elementType")?.as_str().unwrap_or("");
    let type_inner = type_object.get("type")?.as_str()?;
    let type_resolved = resolve_type_reference(type_inner, type_element);

    Some(ObjectFieldSpec {
        name: name.to_string(),
        description: description.map(str::to_string),
        offset,
        optional,
        ty: type_resolved,
    })
}

fn resolve_type_reference(name: &str, element_type: &str) -> TypeReference {
    match name {
        "string" => TypeReference::String,
        "bool" => TypeReference::Boolean,
        "uint8" => TypeReference::Uint8,
        "uint16" => TypeReference::Uint16,
        "uint32" => TypeReference::Uint32,
        "uint64" => TypeReference::Uint64,
        "int8" => TypeReference::Int8,
        "int16" => TypeReference::Int16,
        "int32" => TypeReference::Int32,
        "int64" => TypeReference::Int64,
        "double" => TypeReference::Double,
        "float" => TypeReference::Float,
        "map" => TypeReference::Map,
        "object" => TypeReference::Object,
        "vector" => TypeReference::Vector(Arc::new(resolve_type_reference(element_type, ""))),
        _ => TypeReference::Unresolved(name.to_string()),
    }
}

fn enum_entry_spec_from_value(value: &Value) -> Option<EnumEntrySpec> {
    let object = value.as_object()?;
    let name = object.get("name").and_then(|v| v.as_str())?.to_string();
    let value = object.get("value").and_then(|v| v.as_u64())?;
    let description = object
        .get("description")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    Some(EnumEntrySpec {
        name,
        value,
        description,
    })
}

fn resolve_unresolved_types(map: &HashMap<String, TypeSpec>, target: &mut TypeReference) {
    let mut vector_depth = 0;
    let mut ty_current = Arc::new(target.clone());

    while let TypeReference::Vector(inner) = (*ty_current).clone() {
        vector_depth += 1;
        ty_current = inner;
    }

    if let TypeReference::Unresolved(name) = (*ty_current).clone() {
        let inner = TypeReference::Resolved(Arc::new(
            map.get(&name)
                .unwrap_or_else(|| panic!("failed to resolve type reference of {name}"))
                .clone(),
        ));

        let mut root = inner;
        for _ in 0..vector_depth {
            root = TypeReference::Vector(Arc::new(root));
        }

        *target = root;
    }
}
