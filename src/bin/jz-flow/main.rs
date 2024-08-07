mod global;
mod job;
mod run;

use anyhow::Result;
use clap::{
    Parser,
    Subcommand,
};

use global::GlobalOptions;
use jz_action::{
    core::db::MainDbRepo,
    utils::StdIntoAnyhowResult,
};
use run::{
    run_backend,
    RunArgs,
};
use std::str::FromStr;
use tracing::Level;


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
