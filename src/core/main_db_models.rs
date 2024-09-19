use anyhow::Result;
use mongodb::bson::oid::ObjectId;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub enum JobState {
    #[default]
    Created,
    Selected,
    Deployed,
    Running,
    Error,
    Finish,
    Clean,
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Job {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub name: String,
    pub graph_json: String,
    pub state: JobState,
    pub manual_run: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JobUpdateInfo {
    pub state: Option<JobState>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListJobParams {
    pub state: Option<JobState>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetJobParams {
    pub name: Option<String>,
    pub id: Option<ObjectId>,
}

impl Default for GetJobParams {
    fn default() -> Self {
        Self::new()
    }
}

impl GetJobParams {
    pub fn new() -> Self {
        GetJobParams {
            name: None,
            id: None,
        }
    }

    pub fn set_id(mut self, id: ObjectId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn set_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

pub trait JobRepo: Clone + Send + Sync + 'static {
    fn insert(&self, job: &Job) -> impl std::future::Future<Output = Result<Job>> + Send;

    fn get(
        &self,
        id: &GetJobParams,
    ) -> impl std::future::Future<Output = Result<Option<Job>>> + Send;

    fn delete(&self, id: &ObjectId) -> impl std::future::Future<Output = Result<()>> + Send;

    fn get_job_for_running(&self) -> impl std::future::Future<Output = Result<Option<Job>>> + Send;

    fn update(
        &self,
        id: &ObjectId,
        info: &JobUpdateInfo,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn list_jobs(
        &self,
        list_job_params: &ListJobParams,
    ) -> impl std::future::Future<Output = Result<Vec<Job>>> + Send;
}

// pub trait JobRepo = JobRepo + Clone + Send + Sync + 'static;
