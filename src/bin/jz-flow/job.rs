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
use jz_flow::{
    api::client::JzFlowClient,
    core::db::Job,
    dag::Dag,
    utils::sizefmt::SmartSize,
};
use mongodb::bson::oid::ObjectId;
use prettytable::{
    Row,
    Table,
};
use serde_variant::to_variant_name;
use tokio::fs;

#[derive(Debug, Parser)]
pub(super) enum JobCommands {
    /// Adds files to myapp
    Create(JobCreateArgs),
    Run(RunJobArgs),
    List(ListJobArgs),
    Detail(JobDetailArgs),
    Clean(CleanJobArgs),
}

pub(super) async fn run_job_subcommand(
    global_opts: GlobalOptions,
    command: JobCommands,
) -> Result<()> {
    match command {
        JobCommands::Create(args) => create_job(global_opts, args).await,
        JobCommands::Run(args) => run_job(global_opts, args).await,
        JobCommands::List(args) => list_job(global_opts, args).await,
        JobCommands::Detail(args) => get_job_details(global_opts, args).await,
        JobCommands::Clean(args) => clean_job(global_opts, args).await,
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

#[derive(Debug, Args)]
pub(super) struct ListJobArgs {
    #[arg(long, default_value = "table", help = "format json/table")]
    pub(super) format: String,
}

pub(super) async fn list_job(global_opts: GlobalOptions, args: ListJobArgs) -> Result<()> {
    let client = JzFlowClient::new(&global_opts.listen)?.job();
    let jobs = client.list().await?;

    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }

    let mut table = Table::new();

    // Add a row per time
    table.add_row(Row::from(vec![
        "ID",
        "Name",
        "State",
        "TryNumber",
        "CreatedAt",
        "UpdatedAt",
    ]));

    jobs.iter().for_each(|job| {
        table.add_row(Row::from(vec![
            cell!(job.id),
            cell!(job.name),
            cell!(to_variant_name(&job.state).unwrap()),
            cell!(job.retry_number),
            cell!(DateTime::from_timestamp(job.created_at, 0).unwrap()),
            cell!(DateTime::from_timestamp(job.updated_at, 0).unwrap()),
        ]));
    });

    table.printstd();
    Ok(())
}

#[derive(Debug, Args)]
pub(super) struct JobDetailArgs {
    #[arg(index = 1, help = "job name, must be unique")]
    pub(super) id: String,

    #[arg(long, default_value = "table", help = "format json/table")]
    pub(super) format: String,
}

pub(super) async fn get_job_details(global_opts: GlobalOptions, args: JobDetailArgs) -> Result<()> {
    let client = JzFlowClient::new(&global_opts.listen)?.job();
    let id: ObjectId = ObjectId::from_str(&args.id)?;
    let job_detail = client.get_job_detail(&id).await?;

    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&job_detail)?);
        return Ok(());
    }

    let mut table = Table::new();
    table.add_row(Row::from(vec![
        "ID",
        "Name",
        "State",
        "TryNumber",
        "CreatedAt",
        "UpdatedAt",
    ]));

    table.add_row(Row::from(vec![
        cell!(job_detail.job.id),
        cell!(job_detail.job.name),
        cell!(to_variant_name(&job_detail.job.state).unwrap()),
        cell!(job_detail.job.retry_number),
        cell!(DateTime::from_timestamp(job_detail.job.created_at, 0).unwrap()),
        cell!(DateTime::from_timestamp(job_detail.job.updated_at, 0).unwrap()),
    ]));
    table.printstd();

    if job_detail.node_status.is_none() {
        return Ok(());
    }

    println!("Nodes:");

    let mut table = Table::new();
    table.add_row(Row::from(vec![
        "NodeName",
        "State",
        "DataBatchCount",
        "Replicas",
        "TmpStorage",
        "Pods",
    ]));
    for status in job_detail.node_status.unwrap() {
        let mut pod_table = Table::new();
        pod_table.add_row(Row::from(vec!["Name", "State", "CPU", "Memory"]));
        for pod in status.pods {
            pod_table.add_row(Row::from(vec![
                cell!(pod.0),
                cell!(pod.1.state),
                cell!(pod.1.cpu_usage),
                cell!(pod.1.memory_usage.to_smart_string()),
            ]));
        }

        table.add_row(Row::from(vec![
            cell!(status.name),
            cell!(to_variant_name(&status.state)?),
            cell!(status.data_count),
            cell!(status.replicas),
            cell!(status.storage),
            cell!(pod_table),
        ]));
    }
    table.printstd();
    Ok(())
}

#[derive(Debug, Args)]
pub(super) struct RunJobArgs {
    #[arg(index = 1, help = "job name, must be unique")]
    pub(super) id: String,
}

pub(super) async fn run_job(global_opts: GlobalOptions, args: RunJobArgs) -> Result<()> {
    let client = JzFlowClient::new(&global_opts.listen)?.job();
    let id: ObjectId = ObjectId::from_str(&args.id)?;
    client.run_job(&id).await?;

    println!("Run job successfully, job ID: {}", args.id);
    Ok(())
}

#[derive(Debug, Args)]
pub(super) struct CleanJobArgs {
    #[arg(index = 1, help = "job name, must be unique")]
    pub(super) id: String,
}

pub(super) async fn clean_job(global_opts: GlobalOptions, args: CleanJobArgs) -> Result<()> {
    let client = JzFlowClient::new(&global_opts.listen)?.job();
    let id: ObjectId = ObjectId::from_str(&args.id)?;
    client.clean_job(&id).await?;

    println!("Clean job successfully, job ID: {}", args.id);
    Ok(())
}
