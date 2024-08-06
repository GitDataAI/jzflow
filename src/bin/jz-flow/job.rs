use crate::global::GlobalOptions;
use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub(super) struct JobCreateArgs {
    #[arg(long)]
    pub(super) name: String,

    #[arg(long, help = "dag pipline definition")]
    pub(super) path: String,
}

pub(super) async fn run_backend(global_opts: GlobalOptions, args: JobCreateArgs) -> Result<()> {
    Ok(())
}
