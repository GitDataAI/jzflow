use clap::Args;

#[derive(Debug, Args)]
pub(super) struct GlobalOptions {
    #[arg(long, default_value = "INFO", help="set log level(TRACE, DEBUG, INFO, WARN, ERROR)")]
    pub(super) log_level: String,

    #[arg(long, default_value = "http://localhost:45131", help="set api address")]
    pub(super) listen: String,
}
