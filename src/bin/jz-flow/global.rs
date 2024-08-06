use clap::Args;

#[derive(Debug, Args)]
pub(super) struct GlobalOptions {
    #[arg(short, long, default_value = "INFO")]
    pub(super) log_level: String,

    #[arg(short, long, default_value = "localhost:45131")]
    pub(super) listen: String,
}
