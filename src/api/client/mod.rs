mod job;

use anyhow::Result;
use job::JobClient;
use reqwest::{
    header,
    Client,
    Url,
};
#[derive(Clone)]
pub struct JzFlowClient {
    client: Client,
    base_uri: Url,
}

impl JzFlowClient {
    pub fn new(base_uri: &str) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder().default_headers(headers).build()?;
        let base_uri = Url::parse(base_uri)?.join("/api/v1/")?;
        Ok(JzFlowClient { client, base_uri })
    }

    pub fn job(&self) -> JobClient {
        JobClient {
            client: self.client.clone(),
            base_uri: self.base_uri.clone(),
        }
    }
}
