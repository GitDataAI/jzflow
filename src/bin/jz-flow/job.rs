use crate::global::GlobalOptions;
use anyhow::Result;
use chrono::Utc;
use clap::{
    Args,
    Parser,
    Subcommand,
};
use jz_action::{
    api::client::JzFlowClient,
    core::db::Job,
    dag::Dag,
};
use tokio::fs;

#[derive(Debug, Parser)]
pub(super) enum JobCommands {
    /// Adds files to myapp
    Create(JobCreateArgs),
}

pub(super) async fn run_job_subcommand(
    global_opts: GlobalOptions,
    command: JobCommands,
) -> Result<()> {
    match command {
        JobCommands::Create(args) => create_job(global_opts, args).await,
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

    println!(
        "Create job successfully, job ID: {}",
        created_job.id.to_string()
    );
    Ok(())
}
