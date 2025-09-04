use std::{ops::Deref, sync::Arc};

use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};

const RIOT_ROOT_CERTIFICATE_URL: &str =
    "https://static.developer.riotgames.com/docs/lol/riotgames.pem";

pub struct AuthenticatedHttp {
    http: reqwest::Client,
}

impl AuthenticatedHttp {
    /// Creates a reqwest http client with Riot root certificate and given credentials.
    ///
    /// This method is async because it needs to fetch certificate from static server.
    pub async fn new(token: &str) -> Result<Self, reqwest::Error> {
        let root_certificate = reqwest::get(RIOT_ROOT_CERTIFICATE_URL)
            .await?
            .bytes()
            .await?;
        let root_certificate = reqwest::Certificate::from_pem(root_certificate.as_ref())?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(
                format!(
                    "Basic {}",
                    BASE64_STANDARD.encode(format!("riot:{}", token))
                )
                .as_str(),
            )
            .unwrap(),
        );

        let http = reqwest::Client::builder()
            .add_root_certificate(root_certificate)
            .default_headers(headers)
            .build()?;

        Ok(Self { http })
    }
}

impl Deref for AuthenticatedHttp {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.http
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventSpec;

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionSpec;

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeSpec {
    pub name: String,
    pub description: Option<String>,
    pub detail: TypeSpecDetail,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TypeSpecDetail {
    Object(Vec<ObjectFieldSpec>),
    Enum(Vec<EnumEntrySpec>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectFieldSpec {
    pub name: String,
    pub description: Option<String>,
    pub offset: u64,
    pub optional: bool,
    pub ty: ObjectFieldSpecType,
}

pub enum ObjectFieldSpecType {
    String,
    Boolean,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Int8,
    Int16,
    Int32,
    Int64,
    Double,
    Float,
    Vector(Arc<ObjectFieldSpecType>),
    /// The type that has been resolved
    Resolved(Arc<TypeSpec>),
    /// The type that has to be resolved but can not be resolved immediately
    Unresolved(String),
}

impl Serialize for ObjectFieldSpecType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = "ObjectFieldSpecType";
        match self {
            ObjectFieldSpecType::String => serializer.serialize_unit_variant(name, 0, "String"),
            ObjectFieldSpecType::Boolean => serializer.serialize_unit_variant(name, 1, "Boolean"),
            ObjectFieldSpecType::Uint8 => serializer.serialize_unit_variant(name, 2, "Uint8"),
            ObjectFieldSpecType::Uint16 => serializer.serialize_unit_variant(name, 3, "Uint16"),
            ObjectFieldSpecType::Uint32 => serializer.serialize_unit_variant(name, 4, "Uint32"),
            ObjectFieldSpecType::Uint64 => serializer.serialize_unit_variant(name, 5, "Uint64"),
            ObjectFieldSpecType::Int8 => serializer.serialize_unit_variant(name, 6, "Int8"),
            ObjectFieldSpecType::Int16 => serializer.serialize_unit_variant(name, 7, "Int16"),
            ObjectFieldSpecType::Int32 => serializer.serialize_unit_variant(name, 8, "Int32"),
            ObjectFieldSpecType::Int64 => serializer.serialize_unit_variant(name, 9, "Int64"),
            ObjectFieldSpecType::Double => serializer.serialize_unit_variant(name, 10, "Double"),
            ObjectFieldSpecType::Float => serializer.serialize_unit_variant(name, 11, "Float"),
            ObjectFieldSpecType::Vector(field_spec_type) => serializer.serialize_newtype_variant(
                name,
                12,
                "Vector",
                &*(field_spec_type.clone()),
            ),
            ObjectFieldSpecType::Resolved(type_spec) => {
                serializer.serialize_newtype_variant(name, 13, "Resolved", type_spec.name.as_str())
            }
            ObjectFieldSpecType::Unresolved(type_name) => {
                serializer.serialize_newtype_variant(name, 14, "Unresolved", type_name.as_str())
            }
        }
    }
}

fn serialize_vector_field() {}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnumEntrySpec {
    pub name: String,
    pub value: u64,
    pub description: Option<String>,
}
