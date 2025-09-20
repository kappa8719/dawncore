use std::{fmt::Display, str::FromStr};

use convert_case::{Case, Casing};
use regex::Regex;
use serde::{Deserialize, Serialize};

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "try", "gen",
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Build {
    pub branch: String,
    pub build_type: String,
    pub patchline: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Uncategorized,
    Builtin,
    Core,
    WebSocket,
    Http,
    Async,
    Logging,
    Tracing,
    Performance,
    Plugin(String),
}

impl Category {
    pub fn from_tags(tags: &[String]) -> Self {
        if tags.contains(&String::from("$builtin")) {
            return Self::Builtin;
        } else if tags.contains(&String::from("websocket")) {
            return Self::WebSocket;
        } else if tags.contains(&String::from("logging")) {
            return Self::Logging;
        } else if tags.contains(&String::from("http")) {
            return Self::Http;
        } else if tags.contains(&String::from("core")) {
            return Self::Core;
        } else if tags.contains(&String::from("Tracing")) {
            return Self::Tracing;
        } else if tags.contains(&String::from("performance")) {
            return Self::Performance;
        } else if tags.contains(&String::from("async")) {
            return Self::Async;
        }

        let regex_plugin = Regex::new(r"(?i)^\s*Plugin\s+(?<name>[^\s]+)\s*$").unwrap();
        for tag in tags.iter() {
            if let Some(captures) = regex_plugin.captures(tag.as_str()) {
                let Some(plugin) = captures.name("name") else {
                    continue;
                };

                return Self::Plugin(plugin.as_str().to_owned());
            }
        }

        Self::Uncategorized
    }
}

impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Uncategorized => f.write_str("uncategorized"),
            Category::Builtin => f.write_str("builtin"),
            Category::Core => f.write_str("core"),
            Category::WebSocket => f.write_str("websocket"),
            Category::Http => f.write_str("http"),
            Category::Async => f.write_str("async"),
            Category::Logging => f.write_str("logging"),
            Category::Tracing => f.write_str("tracing"),
            Category::Performance => f.write_str("performance"),
            Category::Plugin(name) => {
                f.write_str(format!("plugin_{}", name.to_case(Case::Snake)).as_str())
            }
        }
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

impl FunctionSpec {
    pub fn ident(&self) -> String {
        let snake_cased = self.name.to_case(Case::Snake);
        if let Some((method, remaining)) = snake_cased.split_once("_")
            && let Ok(method) = FunctionMethod::from_str(method)
        {
            let method = method.to_string().to_lowercase();
            return format!("{remaining}_{method}");
        }

        snake_cased
    }

    pub fn category(&self) -> Category {
        Category::from_tags(self.tags.as_slice())
    }
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

impl FunctionMethod {
    /// Returns true if this method is updating method and can have body in the request
    pub fn is_update(&self) -> bool {
        matches!(self, Self::Post | Self::Put | Self::Patch)
    }
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
    pub parameter: bool,
    pub query: bool,
    pub ty: TypeReference,
}

impl FunctionArgumentSpec {
    pub fn ident(&self) -> String {
        let mut ident = self.name.to_case(Case::Snake);
        ident = ident.replace("+", "_+");
        if RUST_KEYWORDS.contains(&ident.as_str()) {
            format!("{ident}_")
        } else {
            ident
        }
    }
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

    pub fn category(&self) -> Category {
        Category::from_tags(self.tags.as_slice())
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
            TypeReference::Map | TypeReference::Object => "serde_json::Value".to_owned(),
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
