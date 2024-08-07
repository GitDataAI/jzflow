use crate::{
    core::db::{
        Job,
        JobUpdateInfo,
    },
    utils::StdIntoAnyhowResult,
};
use anyhow::{
    anyhow,
    Result,
};
use awc::{
    http::StatusCode,
    Client,
};
use mongodb::bson::oid::ObjectId;
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

    fn job(&self) -> JobClient {
        JobClient {
            client: self.client.clone(),
            base_uri: self.base_uri.clone(),
        }
    }
}

pub struct JobClient {
    client: Client,
    base_uri: String,
}

impl JobClient {
    pub async fn create(&self, job: &Job) -> Result<Job> {
        let mut resp = self
            .client
            .post(self.base_uri.clone() + "/job")
            .send_json(&job)
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let err_msg = resp
                .body()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request job {} reason {err_msg}", resp.status()));
        }

        resp.body()
            .await
            .anyhow()
            .and_then(|body| serde_json::from_slice(&body).anyhow())
            .anyhow()
    }

    pub async fn get(&self, job_id: &ObjectId) -> Result<Option<Job>> {
        let mut resp = self
            .client
            .get(self.base_uri.clone() + "/job/" + job_id.to_hex().as_str())
            .send()
            .await
            .anyhow()?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let err_msg = resp
                .body()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request job {} reason {err_msg}", resp.status()));
        }

        resp.body()
            .await
            .anyhow()
            .and_then(|body| serde_json::from_slice(&body).anyhow())
            .anyhow()
    }

    pub async fn delete(&self, job_id: &ObjectId) -> Result<()> {
        let mut resp = self
            .client
            .delete(self.base_uri.clone() + "/job/" + job_id.to_hex().as_str())
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let err_msg = resp
                .body()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request job {} reason {err_msg}", resp.status()));
        }

        Ok(())
    }

    pub async fn update(&self, job_id: &ObjectId, update_info: &JobUpdateInfo) -> Result<()> {
        let mut resp = self
            .client
            .post(self.base_uri.clone() + "/job/" + job_id.to_hex().as_str())
            .send_json(update_info)
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let err_msg = resp
                .body()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request job {} reason {err_msg}", resp.status()));
        }

        Ok(())
    }
}
