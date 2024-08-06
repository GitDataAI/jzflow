mod global;
mod job;
mod run;

use anyhow::Result;
use clap::{
    Args,
    Parser,
    Subcommand,
};

use global::GlobalOptions;
use jz_action::{
    api::{
        self,
        server::start_rpc_server,
    },
    core::db::MainDbRepo,
    dbrepo::{
        MongoMainDbRepo,
        MongoRunDbRepo,
    },
    driver::kube::KubeDriver,
    utils::StdIntoAnyhowResult,
};
use kube::Client;
use run::{
    run_backend,
    RunArgs,
};
use std::{
    path::Path,
    str::FromStr,
};
use tokio::{
    fs,
    io::AsyncWriteExt,
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    task::JoinSet,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    Level,
};

use jz_action::job::job_mgr::JobManager;

#[derive(Debug, Parser)]
#[command(name = "jz-action-backend", author = "Author Name <github.com/GitDataAI/jz-action>", version, about= "jz-action backend", long_about = None, disable_version_flag = true)]
struct Cli {
    #[clap(flatten)]
    global_opts: GlobalOptions,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Adds files to myapp
    Run(RunArgs),
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(&args.global_opts.log_level)?)
        .try_init()
        .anyhow()?;

    match args.command {
        Commands::Run(run_args) => run_backend(args.global_opts, run_args).await,
    }
}
