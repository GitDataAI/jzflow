use anyhow::Result;
use mongodb::bson::oid::ObjectId;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Debug)]
pub enum JobState {
    Created,
    Running,
    Error,
    Finish,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    pub id: ObjectId,
    pub graph_json: String,
    pub state: JobState,
    pub retry_number: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

pub trait JobRepo {
    fn insert(&self, job: &Job) -> impl std::future::Future<Output = Result<()>> + Send;

    fn get(&self, id: &ObjectId) -> impl std::future::Future<Output = Result<Option<Job>>> + Send;

    fn delete(&self, id: &ObjectId) -> impl std::future::Future<Output = Result<()>> + Send;

    fn get_job_for_running(&self) -> impl std::future::Future<Output = Result<Option<Job>>> + Send;

    fn update_job(
        &self,
        id: &ObjectId,
        state: &JobState,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn list_jobs(&self) -> impl std::future::Future<Output = Result<Vec<Job>>> + Send;
}

pub trait MainDbRepo = JobRepo + Clone + Send + Sync + 'static;
