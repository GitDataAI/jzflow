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


pub struct JobClient {
    pub(crate)  client: Client,
    pub(crate) base_uri: String,
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
