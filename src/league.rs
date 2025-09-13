use std::{fmt::Display, ops::Deref, str::FromStr};

use base64::{Engine, prelude::BASE64_STANDARD};
use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};

const RIOT_ROOT_CERTIFICATE_URL: &str =
    "https://static.developer.riotgames.com/docs/lol/riotgames.pem";

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "try", "gen",
];

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventSpec {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub ty: TypeReference,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionSpec {
    pub arguments: Vec<FunctionArgumentSpec>,
    pub name: String,
    pub description: Option<String>,
    pub method: FunctionMethod,
    pub url: String,
    pub privilege: Option<u64>,
    pub returns: Option<TypeReference>,
    pub thread_safe: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum FunctionMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl FromStr for FunctionMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "get" => Ok(Self::Get),
            "post" => Ok(Self::Post),
            "put" => Ok(Self::Put),
            "patch" => Ok(Self::Patch),
            "delete" => Ok(Self::Delete),
            &_ => Err(()),
        }
    }
}

impl Display for FunctionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionMethod::Get => f.write_str("GET"),
            FunctionMethod::Post => f.write_str("POST"),
            FunctionMethod::Put => f.write_str("PUT"),
            FunctionMethod::Patch => f.write_str("PATCH"),
            FunctionMethod::Delete => f.write_str("DELETE"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionArgumentSpec {
    pub name: String,
    pub description: Option<String>,
    pub optional: bool,
    pub ty: TypeReference,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeSpec {
    pub name: String,
    pub description: Option<String>,
    pub detail: TypeSpecDetail,
    pub tags: Vec<String>,
    pub size: Option<u64>,
}

impl TypeSpec {
    pub fn identifier(&self) -> String {
        self.name.replace("-", "_").to_case(Case::Pascal)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "repr")]
pub enum TypeSpecDetail {
    Object { fields: Vec<ObjectFieldSpec> },
    Enum { entries: Vec<EnumEntrySpec> },
    Unit,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectFieldSpec {
    pub name: String,
    pub description: Option<String>,
    pub offset: u64,
    pub optional: bool,
    pub ty: TypeReference,
}

impl ObjectFieldSpec {
    pub fn ident(&self) -> String {
        let ident = self.name.to_case(Case::Snake);
        if RUST_KEYWORDS.contains(&ident.as_str()) {
            format!("_{ident}")
        } else {
            ident
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeReference {
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
    Map,
    Object,
    Vector(Box<TypeReference>),
    Reference(String),
}

impl TypeReference {
    pub fn to_rust_type(&self, boxed: bool) -> String {
        match self {
            TypeReference::String => "std::string::String".to_owned(),
            TypeReference::Boolean => "bool".to_owned(),
            TypeReference::Uint8 => "u8".to_owned(),
            TypeReference::Uint16 => "u16".to_owned(),
            TypeReference::Uint32 => "u32".to_owned(),
            TypeReference::Uint64 => "u64".to_owned(),
            TypeReference::Int8 => "i8".to_owned(),
            TypeReference::Int16 => "i16".to_owned(),
            TypeReference::Int32 => "i32".to_owned(),
            TypeReference::Int64 => "i64".to_owned(),
            TypeReference::Double => "f64".to_owned(),
            TypeReference::Float => "f32".to_owned(),
            TypeReference::Map => {
                "std::collections::HashMap<std::string::String, std::string::String>".to_owned()
            }
            TypeReference::Object => {
                "std::collections::HashMap<std::any::Any, std::any::Any>".to_owned()
            }
            TypeReference::Vector(type_reference) => {
                format!("std::vec::Vec<{}>", type_reference.to_rust_type(boxed))
            }
            TypeReference::Reference(name) => {
                let ident = name.replace("-", "_").to_case(Case::Pascal);
                if boxed {
                    format!("std::boxed::Box<{}>", ident)
                } else {
                    ident
                }
            }
        }
    }
}

// impl Serialize for TypeReference {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let name = "ObjectFieldSpecType";
//         match self {
//             TypeReference::String => serializer.serialize_unit_variant(name, 0, "String"),
//             TypeReference::Boolean => serializer.serialize_unit_variant(name, 1, "Boolean"),
//             TypeReference::Uint8 => serializer.serialize_unit_variant(name, 2, "Uint8"),
//             TypeReference::Uint16 => serializer.serialize_unit_variant(name, 3, "Uint16"),
//             TypeReference::Uint32 => serializer.serialize_unit_variant(name, 4, "Uint32"),
//             TypeReference::Uint64 => serializer.serialize_unit_variant(name, 5, "Uint64"),
//             TypeReference::Int8 => serializer.serialize_unit_variant(name, 6, "Int8"),
//             TypeReference::Int16 => serializer.serialize_unit_variant(name, 7, "Int16"),
//             TypeReference::Int32 => serializer.serialize_unit_variant(name, 8, "Int32"),
//             TypeReference::Int64 => serializer.serialize_unit_variant(name, 9, "Int64"),
//             TypeReference::Double => serializer.serialize_unit_variant(name, 10, "Double"),
//             TypeReference::Float => serializer.serialize_unit_variant(name, 11, "Float"),
//             TypeReference::Map => serializer.serialize_unit_variant(name, 12, "Map"),
//             TypeReference::Object => serializer.serialize_unit_variant(name, 13, "Object"),
//             TypeReference::Vector(field_spec_type) => serializer.serialize_newtype_variant(
//                 name,
//                 14,
//                 "Vector",
//                 &*(field_spec_type.clone()),
//             ),
//             TypeReference::Resolved(type_spec) => {
//                 serializer.serialize_newtype_variant(name, 15, "Resolved", type_spec.name.as_str())
//             }
//             TypeReference::Unresolved(type_name) => {
//                 serializer.serialize_newtype_variant(name, 16, "Unresolved", type_name.as_str())
//             }
//         }
//     }
// }
//
// struct ObjectFieldSpecTypeVisitor;
//
// impl<'de> Visitor<'de> for ObjectFieldSpecTypeVisitor {
//     type Value = TypeReference;
//
//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("a variant of ObjectFieldSpecType")
//     }
//
//     fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
//     where
//         A: serde::de::EnumAccess<'de>,
//     {
//         let (variant, variant_access) = data.variant::<String>()?;
//
//         fn visit_unit<'de, A>(
//             variant_access: A::Variant,
//             variant: TypeReference,
//         ) -> Result<TypeReference, A::Error>
//         where
//             A: serde::de::EnumAccess<'de>,
//         {
//             variant_access.unit_variant()?;
//             Ok(variant)
//         }
//
//         fn visit_vector<'de, A>(variant_access: A::Variant) -> Result<TypeReference, A::Error>
//         where
//             A: serde::de::EnumAccess<'de>,
//         {
//             let name = variant_access.newtype_variant::<String>()?;
//
//             Ok(TypeReference::Vector(Arc::new(TypeReference::Unresolved(
//                 name,
//             ))))
//         }
//
//         fn visit_unresolved<'de, A>(variant_access: A::Variant) -> Result<TypeReference, A::Error>
//         where
//             A: serde::de::EnumAccess<'de>,
//         {
//             let name = variant_access.newtype_variant::<String>()?;
//
//             Ok(TypeReference::Unresolved(name))
//         }
//
//         match variant.as_str() {
//             "String" => visit_unit::<'de, A>(variant_access, TypeReference::String),
//             "Boolean" => visit_unit::<'de, A>(variant_access, TypeReference::Boolean),
//             "Uint8" => visit_unit::<'de, A>(variant_access, TypeReference::Uint8),
//             "Uint16" => visit_unit::<'de, A>(variant_access, TypeReference::Uint16),
//             "Uint32" => visit_unit::<'de, A>(variant_access, TypeReference::Uint32),
//             "Uint64" => visit_unit::<'de, A>(variant_access, TypeReference::Uint64),
//             "Int8" => visit_unit::<'de, A>(variant_access, TypeReference::Int8),
//             "Int16" => visit_unit::<'de, A>(variant_access, TypeReference::Int16),
//             "Int32" => visit_unit::<'de, A>(variant_access, TypeReference::Int32),
//             "Int64" => visit_unit::<'de, A>(variant_access, TypeReference::Int64),
//             "Double" => visit_unit::<'de, A>(variant_access, TypeReference::Double),
//             "Float" => visit_unit::<'de, A>(variant_access, TypeReference::Float),
//             "Map" => visit_unit::<'de, A>(variant_access, TypeReference::Map),
//             "Object" => visit_unit::<'de, A>(variant_access, TypeReference::Object),
//             "Vector" => visit_vector::<'de, A>(variant_access),
//             "Resolved" | "Unresolved" => visit_unresolved::<'de, A>(variant_access),
//             _ => unreachable!(),
//         }
//     }
// }
//
// impl<'de> Deserialize<'de> for TypeReference {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let variants = &[
//             "String",
//             "Boolean",
//             "Uint8",
//             "Uint16",
//             "Uint32",
//             "Uint64",
//             "Int8",
//             "Int16",
//             "Int32",
//             "Int64",
//             "Double",
//             "Float",
//             "Map",
//             "Object",
//             "Vector",
//             "Resolved",
//             "Unresolved",
//         ];
//         deserializer.deserialize_enum("ObjectFieldSpecType", variants, ObjectFieldSpecTypeVisitor)
//     }
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnumEntrySpec {
    pub name: String,
    pub value: u64,
    pub description: Option<String>,
}

impl EnumEntrySpec {
    pub fn ident(&self) -> String {
        self.name.to_case(Case::Pascal)
    }
}
