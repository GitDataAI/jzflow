use anyhow::Result;
use clap::Parser;
use compute_unit_runner::ipc::{
    self,
    ErrorNumber,
    IPCClient,
    IPCError,
};
use jz_action::utils::StdIntoAnyhowResult;
use std::{
    path::Path,
    str::FromStr,
    time::Duration,
};
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    task::JoinSet,
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    Level,
};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(
    name = "list_files",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-action>",
    about = "embed in k8s images"
)]

struct Args {
    #[arg(short, long, default_value = "INFO")]
    log_level: String,

    #[arg(short, long, default_value = "/unix_socket/compute_unit_runner_d")]
    unix_socket_addr: String,

    #[arg(short, long, default_value = "/app/tmp")]
    tmp_path: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(&args.log_level)?)
        .try_init()
        .anyhow()?;
    let mut join_set = JoinSet::new();
    let token = CancellationToken::new();

    {
        let token = token.clone();
        join_set.spawn(async move { print_files(token, args).await });
    }

    {
        //catch signal
        tokio::spawn(async move {
            let mut sig_term = signal(SignalKind::terminate()).unwrap();
            let mut sig_int = signal(SignalKind::interrupt()).unwrap();
            select! {
                _ = sig_term.recv() => info!("Recieve SIGTERM"),
                _ = sig_int.recv() => info!("Recieve SIGTINT"),
            };
            token.cancel();
        });
    }

    nodes_sdk::monitor_tasks(&mut join_set).await
}

async fn print_files(token: CancellationToken, args: Args) -> Result<()> {
    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    let tmp_path = Path::new(&args.tmp_path);
    loop {
        if token.is_cancelled() {
            return anyhow::Ok(());
        }

        info!("request data");
        match client.request_avaiable_data().await {
            Ok(Some(req)) => {
                let id = req.id;
                let path_str = tmp_path.join(&id);
                let root_input_dir = path_str.as_path();

                for entry in WalkDir::new(root_input_dir) {
                    let entry = entry?;
                    if entry.file_type().is_file() {
                        let path = entry.path();
                        info!("read path {:?}", path);
                    }
                }

                client.complete_result(&id).await.anyhow()?;
            }
            Ok(None) => {
                sleep(Duration::from_secs(2)).await;
                continue;
            }
            Err(IPCError::NodeError { code, msg: _ }) => match code {
                ErrorNumber::AlreadyFinish => {
                    return Ok(());
                }
                ErrorNumber::NotReady => {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
                ErrorNumber::DataNotFound => {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            },
            Err(IPCError::UnKnown(msg)) => {
                error!("got unknow error {msg}");
            }
        }
    }
}
