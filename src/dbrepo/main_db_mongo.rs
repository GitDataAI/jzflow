use crate::{
    core::db::{
        GetJobParams, Job, JobRepo, JobState, JobUpdateInfo, ListJobParams
    },
    utils::{
        IntoAnyhowResult,
        StdIntoAnyhowResult,
    },
};

use anyhow::{
    anyhow,
    Result,
};

use chrono::Utc;

use futures::TryStreamExt;
use mongodb::{
    bson::{
        doc,
        oid::ObjectId,
    },
    error::ErrorKind,
    options::{
        ClientOptions,
        IndexOptions,
    },
    Client,
    Collection,
    IndexModel,
};

use serde_variant::to_variant_name;

const JOB_COL_NAME: &str = "job";

#[derive(Clone)]
pub struct MongoMainDbRepo {
    job_col: Collection<Job>,
}

impl MongoMainDbRepo {
    pub async fn new(connectstring: &str) -> Result<Self> {
        let options = ClientOptions::parse(connectstring).await?;
        let database = options
            .default_database
            .as_ref()
            .expect("set db name in url")
            .clone();
        let client = Client::with_options(options)?;
        let database = client.database(database.as_str());
        let job_col: Collection<Job> = database.collection(JOB_COL_NAME);

        {
            //create index for jobs
            let idx_opts: IndexOptions = IndexOptions::builder()
                .unique(true)
                .name("idx_name".to_owned())
                .build();

            let index = IndexModel::builder()
                .keys(doc! { "name": 1 })
                .options(idx_opts)
                .build();

            if let Err(err) = job_col.create_index(index).await {
                match *err.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    err => {
                        return Err(anyhow!("create job state index error {err}"));
                    }
                }
            }
        }

        {
            //create index for jobs
            let idx_opts: IndexOptions =
                IndexOptions::builder().name("idx_state".to_owned()).build();

            let index = IndexModel::builder()
                .keys(doc! { "state": 1 })
                .options(idx_opts)
                .build();

            if let Err(err) = job_col.create_index(index).await {
                match *err.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    err => {
                        return Err(anyhow!("create job state index error {err}"));
                    }
                }
            }
        }
        Ok(MongoMainDbRepo { job_col })
    }
}

impl JobRepo for MongoMainDbRepo {
    async fn insert(&self, job: &Job) -> Result<Job> {
        let inserted_id = self.job_col.insert_one(job).await?.inserted_id;

        self.job_col
            .find_one(doc! {"_id": inserted_id})
            .await
            .anyhow()
            .and_then(|r| r.anyhow("insert job not found"))
    }

    async fn get_job_for_running(&self) -> Result<Option<Job>> {
        let update = doc! {
            "$set": {
                "state": to_variant_name(&JobState::Selected)?,
                "updated_at":Utc::now().timestamp(),
            },
        };

        self.job_col
            .find_one_and_update(doc! {"state": to_variant_name(&JobState::Created)?}, update)
            .await
            .anyhow()
    }

    async fn update(&self, id: &ObjectId, info: &JobUpdateInfo) -> Result<()> {
        let mut update_fields = doc! {
            "updated_at":Utc::now().timestamp()
        };
        if let Some(state) = info.state.as_ref() {
            update_fields.insert("state", to_variant_name(state)?);
        }

        let update = doc! {"$set": update_fields};
        let query = doc! {
            "_id":  id,
        };

        self.job_col
            .update_one(query, update)
            .await
            .map(|_| ())
            .anyhow()
    }

    async fn list_jobs(&self, list_job_params: &ListJobParams) -> Result<Vec<Job>> {
        let mut query = doc! {};
        if let Some(state) = list_job_params.state.as_ref() {
            query.insert("state", to_variant_name(state)?);
        }

        self.job_col.find(query).await?.try_collect().await.anyhow()
    }

    async fn get(&self, get_params: &GetJobParams) -> Result<Option<Job>> {
        let mut query = doc! {};
        if let Some(id) = get_params.id.as_ref() {
            query.insert("_id", id);
        }
        if let Some(name) = get_params.name.as_ref() {
            query.insert("name", name);
        }

        if query.is_empty() {
            return Ok(None);
        }

        self.job_col.find_one(query).await.anyhow()
    }

    async fn delete(&self, id: &ObjectId) -> Result<()> {
        self.job_col
            .delete_one(doc! {"_id": id})
            .await
            .map(|_| ())
            .anyhow()
    }
}
