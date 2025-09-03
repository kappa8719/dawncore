use std::{ops::Deref, sync::Arc};

use base64::{Engine, prelude::BASE64_STANDARD};

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

pub struct EventSpec;

pub struct FunctionSpec;

pub struct TypeSpec {
    pub name: String,
    pub description: Option<String>,
    pub detail: TypeSpecDetail,
    pub tags: Vec<String>,
}

pub enum TypeSpecDetail {
    Object(Vec<ObjectFieldSpec>),
    Enum(Vec<EnumEntrySpec>),
}

pub struct ObjectFieldSpec {
    pub name: String,
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
    Resolve(Arc<TypeSpec>),
}

pub struct EnumEntrySpec {
    pub name: String,
    pub value: u64,
    pub description: Option<String>,
}
