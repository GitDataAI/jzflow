mod job;

use awc::Client;
use anyhow::Result;
use job::JobClient;

#[derive(Clone)]
pub struct JzFlowClient {
    client: Client,
    base_uri: String,
}

impl JzFlowClient {
    pub fn new(base_uri: &str) -> Result<Self> {
        let client = Client::builder()
            .add_default_header(("Content-Type", "application/json"))
            .finish();
        let base_uri = base_uri.to_string() + "/api/v1";
        Ok(JzFlowClient { client, base_uri })
    }

    pub fn job(&self) -> JobClient {
        JobClient {
            client: self.client.clone(),
            base_uri: self.base_uri.clone(),
        }
    }
}