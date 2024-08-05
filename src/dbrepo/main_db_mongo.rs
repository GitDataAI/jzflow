use crate::{
    core::db::{
        DBConfig,
        Job,
        JobRepo,
        JobState,
    },
    utils::StdIntoAnyhowResult,
};

use anyhow::{
    anyhow,
    Result,
};

use chrono::{
    Duration,
    Utc,
};

use futures::TryStreamExt;
use mongodb::{
    bson::{
        doc,
        oid::ObjectId,
    },
    error::ErrorKind,
    options::IndexOptions,
    Client,
    Collection,
    IndexModel,
};

use serde_variant::to_variant_name;

const JOB_COL_NAME: &'static str = "job";

#[derive(Clone)]
pub struct MongoMainDbRepo {
    job_col: Collection<Job>,
}

impl MongoMainDbRepo {
    pub async fn new<DBC>(config: DBC, db_name: &str) -> Result<Self>
    where
        DBC: DBConfig,
    {
        let client = Client::with_uri_str(config.connection_string()).await?;
        let database = client.database(db_name);
        let job_col: Collection<Job> = database.collection(&JOB_COL_NAME);

        {
            //create index for jobs
            let idx_opts: IndexOptions =
                IndexOptions::builder().name("idx_state".to_owned()).build();

            let index = IndexModel::builder()
                .keys(doc! { "state": 1 })
                .options(idx_opts)
                .build();

            if let Err(e) = job_col.create_index(index).await {
                match *e.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    e => {
                        return Err(anyhow!("create job state index error {}", e));
                    }
                }
            }
        }
        Ok(MongoMainDbRepo { job_col })
    }
}

impl JobRepo for MongoMainDbRepo {
    async fn insert(&self, job: &Job) -> Result<()> {
        self.job_col.insert_one(job).await.map(|_| ()).anyhow()
    }

    async fn get_job_for_running(&self) -> Result<Option<Job>> {
        let update = doc! {
            "$set": {
                "state": to_variant_name(&JobState::Running)?,
                "updated_at":Utc::now().timestamp(),
            },
        };

        self.job_col
            .find_one_and_update(doc! {"state": to_variant_name(&JobState::Created)?}, update)
            .await
            .anyhow()
    }

    async fn update_job(&self, id: &ObjectId, state: &JobState) -> Result<()> {
        let update = doc! {
            "$set": {
                "state": to_variant_name(state)?,
                "updated_at":Utc::now().timestamp(),
            }
        };

        let query = doc! {
            "_id":  id,
        };

        self.job_col
            .update_one(query, update)
            .await
            .map(|_| ())
            .anyhow()
    }

    async fn list_jobs(&self) -> Result<Vec<Job>> {
        self.job_col
            .find(doc! {})
            .await?
            .try_collect()
            .await
            .anyhow()
    }

    async fn get(&self, id: &ObjectId) -> Result<Option<Job>> {
        self.job_col.find_one(doc! {"_id": id}).await.anyhow()
    }

    async fn delete(&self, id: &ObjectId) -> Result<()> {
        self.job_col
            .delete_one(doc! {"_id": id})
            .await
            .map(|_| ())
            .anyhow()
    }
}
