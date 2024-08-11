#![feature(iter_repeat_n)]
#[macro_use]
extern crate prettytable;

mod daemon;
mod global;
mod job;

use anyhow::Result;
use clap::{
    Parser,
    Subcommand,
};

use daemon::{
    run_daemon,
    DaemonArgs,
};
use global::GlobalOptions;
use job::{
    run_job_subcommand,
    JobCommands,
};

use jz_flow::{
    core::db::MainDbRepo,
    utils::StdIntoAnyhowResult,
};

use std::str::FromStr;
use tracing::Level;

#[derive(Debug, Parser)]
#[command(name = "jz-flow-daemon", author = "Author Name <github.com/GitDataAI/jz-flow>", version, about= "jz-flow daemon", long_about = None, disable_version_flag = true)]
struct Cli {
    #[clap(flatten)]
    global_opts: GlobalOptions,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Adds files to myapp
    Daemon(DaemonArgs),

    #[command(subcommand)]
    Job(JobCommands),
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(&args.global_opts.log_level)?)
        .try_init()
        .anyhow()?;

    match args.command {
        Commands::Daemon(run_args) => run_daemon(args.global_opts, run_args).await,
        Commands::Job(job_commands) => run_job_subcommand(args.global_opts, job_commands).await,
    }
}
