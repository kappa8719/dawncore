use std::collections::HashMap;

use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct RootHelpResponse {
    events: HashMap<String, String>,
    functions: HashMap<String, String>,
    types: HashMap<String, String>,
}
/// League api generator
pub struct League {
    host: Url,
    http: riot_bridge::league::AuthenticatedHttp,
}

impl League {
    pub fn new(host: Url, http: riot_bridge::league::AuthenticatedHttp) -> League {
        Self { host, http }
    }

    async fn resolve_root_help(&self) -> Result<RootHelpResponse, reqwest::Error> {
        let response = self
            .http
            .get(self.host.join("/help").unwrap())
            .send()
            .await?;

        let json = response.json::<RootHelpResponse>().await?;

        Ok(json)
    }

    async fn resolve_types(&self, help: &RootHelpResponse) -> Result<(), reqwest::Error> {
        for (name, _) in help.types.iter() {
            let resolved = self.resolve_type(name.as_str()).await?;
        }

        Ok(())
    }

    async fn resolve_type(&self, name: &str) -> Result<(), reqwest::Error> {
        let response: serde_json::Value = self
            .http
            .post(self.host.join("/help").unwrap())
            .query(&[("target", name), ("format", "Full")])
            .send()
            .await?
            .json()
            .await?;

        println!("resolved type {name}");
        println!("{response:?}");

        todo!()
    }

    pub async fn resolve(&self) {
        let root_help = self.resolve_root_help().await.unwrap();
        let _ = self.resolve_types(&root_help).await;
    }
}
