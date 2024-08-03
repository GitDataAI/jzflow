use anyhow::Ok;
use anyhow::{anyhow, Result};
use clap::Parser;
use compute_unit_runner::ipc::{self, IPCClient};
use jz_action::utils::StdIntoAnyhowResult;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{error, info, Level};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(
    name = "dummy_out",
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

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<Result<()>>(1);
    {
        let shutdown_tx = shutdown_tx.clone();
        let _ = tokio::spawn(async move {
            if let Err(e) = write_dummy(args).await {
                let _ = shutdown_tx.send(Err(anyhow!("dummy out {e}"))).await;
            }
        });
    }

    {
        //catch signal
        let _ = tokio::spawn(async move {
            let mut sig_term = signal(SignalKind::terminate()).unwrap();
            let mut sig_int = signal(SignalKind::interrupt()).unwrap();
            select! {
                _ = sig_term.recv() => info!("Recieve SIGTERM"),
                _ = sig_int.recv() => info!("Recieve SIGTINT"),
            };
            let _ = shutdown_tx.send(Err(anyhow!("cancel by signal"))).await;
        });
    }

    if let Some(Err(err)) = shutdown_rx.recv().await {
        error!("program exit with error {:?}", err)
    }
    info!("gracefully shutdown");
    Ok(())
}

async fn write_dummy(args: Args) -> Result<()> {
    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    let tmp_path = Path::new(&args.tmp_path);
    loop {
        let req = client.request_avaiable_data().await?;
        if req.is_none() {
            sleep(Duration::from_secs(2)).await;
            continue;
        }
        let id = req.unwrap().id;
        let path_str = tmp_path.join(&id);
        let root_input_dir = path_str.as_path();

        for entry in WalkDir::new(root_input_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                info!("read path {:?}", path);
            }
        }

        client.complete_result(&id).await?;
    }
}
