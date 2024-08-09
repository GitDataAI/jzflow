use std::str::FromStr;

use crate::global::GlobalOptions;
use anyhow::Result;
use chrono::{
    DateTime,
    Utc,
};
use clap::{
    Args,
    Parser,
};
use comfy_table::Table;
use jz_action::{
    api::client::JzFlowClient,
    core::db::Job,
    dag::Dag,
};
use mongodb::bson::oid::ObjectId;
use serde_variant::to_variant_name;
use tokio::fs;

#[derive(Debug, Parser)]
pub(super) enum JobCommands {
    /// Adds files to myapp
    Create(JobCreateArgs),
    List,
    Detail(JobDetailArgs),
}

pub(super) async fn run_job_subcommand(
    global_opts: GlobalOptions,
    command: JobCommands,
) -> Result<()> {
    match command {
        JobCommands::Create(args) => create_job(global_opts, args).await,
        JobCommands::List => list_job(global_opts).await,
        JobCommands::Detail(args) => get_job_details(global_opts, args).await,
    }
}

#[derive(Debug, Args)]
pub(super) struct JobCreateArgs {
    #[arg(long, help = "job name, must be unique")]
    pub(super) name: String,

    #[arg(long, help = "dag pipline definition")]
    pub(super) path: String,
}

pub(super) async fn create_job(global_opts: GlobalOptions, args: JobCreateArgs) -> Result<()> {
    let client = JzFlowClient::new(&global_opts.listen)?.job();
    let dag_config = fs::read_to_string(&args.path).await?;
    let _ = Dag::from_json(dag_config.as_str())?;
    let tm = Utc::now().timestamp();
    let job = Job {
        name: args.name.clone(),
        graph_json: dag_config,
        created_at: tm,
        updated_at: tm,
        ..Default::default()
    };

    let created_job = client.create(&job).await?;

    println!("Create job successfully, job ID: {}", created_job.id);
    Ok(())
}

pub(super) async fn list_job(global_opts: GlobalOptions) -> Result<()> {
    let client = JzFlowClient::new(&global_opts.listen)?.job();
    let jobs = client.list().await?;

    let mut table = Table::new();
    table.set_header(vec![
        "ID",
        "Name",
        "State",
        "TryNumber",
        "CreatedAt",
        "UpdatedAt",
    ]);

    jobs.iter().for_each(|job| {
        table.add_row(vec![
            job.id.to_string(),
            job.name.to_string(),
            to_variant_name(&job.state).unwrap().to_string(),
            job.retry_number.to_string(),
            DateTime::from_timestamp(job.created_at, 0)
                .unwrap()
                .to_string(),
            DateTime::from_timestamp(job.updated_at, 0)
                .unwrap()
                .to_string(),
        ]);
    });
    println!("{table}");
    Ok(())
}

#[derive(Debug, Args)]
pub(super) struct JobDetailArgs {
    #[arg(long, help = "job name, must be unique")]
    pub(super) id: String,
}

pub(super) async fn get_job_details(global_opts: GlobalOptions, args: JobDetailArgs) -> Result<()> {
    let client = JzFlowClient::new(&global_opts.listen)?.job();
    let id: ObjectId = ObjectId::from_str(&args.id)?;
    let job_detail = client.get_job_detail(&id).await?;

    println!("{}", serde_json::to_string_pretty(&job_detail)?);
    Ok(())
}
