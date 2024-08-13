use crate::{
    core::db::{
        GetJobParams, Job, JobUpdateInfo
    },
    job::job_mgr::JobDetails,
    utils::StdIntoAnyhowResult,
};
use anyhow::{
    anyhow,
    Result,
};

use mongodb::bson::oid::ObjectId;
use reqwest::{
    Client,
    StatusCode,
    Url,
};

pub struct JobClient {
    pub(crate) client: Client,
    pub(crate) base_uri: Url,
}

impl JobClient {
    pub async fn create(&self, job: &Job) -> Result<Job> {
        let resp = self
            .client
            .post(self.base_uri.clone().join("job")?)
            .json(&job)
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("create job {code} reason {err_msg}"));
        }

        resp.bytes()
            .await
            .anyhow()
            .and_then(|body| serde_json::from_slice(&body).anyhow())
            .anyhow()
    }

    pub async fn get_by_id(&self, job_id: &ObjectId) -> Result<Option<Job>> {
        let resp = self
            .client
            .get(
                self.base_uri
                    .clone()
                    .join("job/")?
                    .join(job_id.to_hex().as_str())?,
            )
            .send()
            .await
            .anyhow()?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("get job {code} reason {err_msg}"));
        }

        resp.bytes()
            .await
            .anyhow()
            .and_then(|body| serde_json::from_slice(&body).anyhow())
            .anyhow()
    }

    pub async fn get(&self, get_job_params: &GetJobParams) -> Result<Option<Job>> {
        let mut uri = self.base_uri
        .clone()
        .join("job")?;
        
        if let Some(id) = get_job_params.id.as_ref() {
            uri.query_pairs_mut().append_pair("id", id.to_string().as_str());
        }

        if let Some(name) = get_job_params.name.as_ref() {
            uri.query_pairs_mut().append_pair("name", name.as_str());
        }

        let resp = self
            .client
            .get(uri)
            .send()
            .await
            .anyhow()?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("get job {code} reason {err_msg}"));
        }

        resp.bytes()
            .await
            .anyhow()
            .and_then(|body| serde_json::from_slice(&body).anyhow())
            .anyhow()
    }

    pub async fn list(&self) -> Result<Vec<Job>> {
        let resp = self
            .client
            .get(self.base_uri.clone().join("jobs")?)
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("list job {code} reason {err_msg}"));
        }

        resp.bytes()
            .await
            .anyhow()
            .and_then(|body| serde_json::from_slice(&body).anyhow())
            .anyhow()
    }

    pub async fn delete(&self, job_id: &ObjectId) -> Result<()> {
        let resp = self
            .client
            .delete(
                self.base_uri
                    .clone()
                    .join("job/")?
                    .join(job_id.to_hex().as_str())?,
            )
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("delete job {code} reason {err_msg}"));
        }

        Ok(())
    }

    pub async fn update(&self, job_id: &ObjectId, update_info: &JobUpdateInfo) -> Result<()> {
        let resp = self
            .client
            .post(
                self.base_uri
                    .clone()
                    .join("job/")?
                    .join(job_id.to_hex().as_str())?,
            )
            .json(update_info)
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request update job {code} reason {err_msg}"));
        }

        Ok(())
    }

    pub async fn get_job_detail(&self, job_id: &ObjectId) -> Result<JobDetails> {
        let resp = self
            .client
            .get(
                self.base_uri
                    .clone()
                    .join("job/")?
                    .join("detail/")?
                    .join(job_id.to_hex().as_str())?,
            )
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request job detail {code} reason {err_msg}"));
        }

        resp.bytes()
            .await
            .anyhow()
            .and_then(|body| serde_json::from_slice(&body).anyhow())
            .anyhow()
    }

    pub async fn run_job(&self, job_id: &ObjectId) -> Result<()> {
        let resp = self
            .client
            .post(
                self.base_uri
                    .clone()
                    .join("job/")?
                    .join("run/")?
                    .join(job_id.to_hex().as_str())?,
            )
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request start job {code} reason {err_msg}"));
        }

        Ok(())
    }

    pub async fn clean_job(&self, job_id: &ObjectId) -> Result<()> {
        let resp = self
            .client
            .delete(
                self.base_uri
                    .clone()
                    .join("job/")?
                    .join(job_id.to_hex().as_str())?,
            )
            .send()
            .await
            .anyhow()?;

        if !resp.status().is_success() {
            let code = resp.status();
            let err_msg = resp
                .bytes()
                .await
                .anyhow()
                .and_then(|body| String::from_utf8(body.into()).anyhow())?;
            return Err(anyhow!("request start job {code} reason {err_msg}"));
        }

        Ok(())
    }
}
