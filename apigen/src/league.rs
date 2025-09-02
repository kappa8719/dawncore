use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct RootHelpResponse {
    events: HashMap<String, String>,
    functions: HashMap<String, String>,
    types: HashMap<String, String>,
}

/// League api generator
pub struct League {
    host: String,
    http: reqwest::Client,
}

impl League {
    pub fn new(host: String) -> League {
        Self {
            host,
            http: reqwest::Client::new(),
        }
    }

    pub fn generate_types(&self) {}
}
