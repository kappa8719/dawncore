use std::{collections::HashMap, fs::File, io::Write, sync::Arc};

use lapi::league::{EnumEntrySpec, ObjectFieldSpec, ObjectFieldSpecType, TypeSpec, TypeSpecDetail};
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct RootHelpResponse {
    events: HashMap<String, String>,
    functions: HashMap<String, String>,
    types: HashMap<String, String>,
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

        for (_, resolved) in type_specs.clone().iter_mut() {
            let TypeSpecDetail::Object { fields } = &mut resolved.detail else {
                continue;
            };

            for field in fields.iter_mut() {
                let mut vector_depth = 0;
                let mut ty_current = Arc::new(field.ty.clone());

                while let ObjectFieldSpecType::Vector(inner) = (*ty_current).clone() {
                    vector_depth += 1;
                    ty_current = inner;
                }

                if let ObjectFieldSpecType::Unresolved(name) = (*ty_current).clone() {
                    let inner = ObjectFieldSpecType::Resolved(Arc::new(
                        type_specs
                            .get(&name)
                            .expect("failed to resolve type reference of {name}")
                            .clone(),
                    ));

                    let mut root = inner;
                    for _ in 0..vector_depth {
                        root = ObjectFieldSpecType::Vector(Arc::new(root));
                    }

                    field.ty = root;
                }
            }
        }

        let mut file = File::create("./generated_type_specs.toml").unwrap();
        file.write_all(toml::to_string_pretty(&type_specs).unwrap().as_bytes())
            .unwrap();

        Ok(type_specs)
    }

    async fn resolve_type(&self, name: &str) -> Result<TypeSpec, reqwest::Error> {
        let response: serde_json::Value = self
            .http
            .post(self.host.join("/help").unwrap())
            .query(&[("target", name), ("format", "Full")])
            .send()
            .await?
            .json()
            .await?;

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

    pub async fn resolve(&self) {
        let root_help = self.resolve_root_help().await.unwrap();
        let _ = self.resolve_types(&root_help).await;
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
    let type_resolved = resolve_object_field_spec_type(type_inner, type_element);

    Some(ObjectFieldSpec {
        name: name.to_string(),
        description: description.map(str::to_string),
        offset,
        optional,
        ty: type_resolved,
    })
}

fn resolve_object_field_spec_type(name: &str, element_type: &str) -> ObjectFieldSpecType {
    match name {
        "string" => ObjectFieldSpecType::String,
        "bool" => ObjectFieldSpecType::Boolean,
        "uint8" => ObjectFieldSpecType::Uint8,
        "uint16" => ObjectFieldSpecType::Uint16,
        "uint32" => ObjectFieldSpecType::Uint32,
        "uint64" => ObjectFieldSpecType::Uint64,
        "int8" => ObjectFieldSpecType::Int8,
        "int16" => ObjectFieldSpecType::Int16,
        "int32" => ObjectFieldSpecType::Int32,
        "int64" => ObjectFieldSpecType::Int64,
        "double" => ObjectFieldSpecType::Double,
        "float" => ObjectFieldSpecType::Float,
        "vector" => {
            ObjectFieldSpecType::Vector(Arc::new(resolve_object_field_spec_type(element_type, "")))
        }
        _ => ObjectFieldSpecType::Unresolved(name.to_string()),
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
