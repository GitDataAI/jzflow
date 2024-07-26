use anyhow::Ok;
use anyhow::{anyhow, Result};
use clap::Parser;
use compute_unit_runner::ipc::{self, IPCClient};
use jiaozifs_client_rs::apis::{self};
use jiaozifs_client_rs::models::RefType;
use jz_action::utils::IntoAnyhowResult;
use jz_action::utils::StdIntoAnyhowResult;
use std::path::Path;
use std::str::FromStr;
use tokio::fs;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tracing::{info, Level};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(
    name = "jz_writer",
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

    #[arg(long, default_value = "64")]
    batch_size: usize,

    #[arg(long)]
    jiaozifs_url: String,

    #[arg(long)]
    username: String,

    #[arg(long)]
    password: String,

    #[arg(long)]
    owner: String,

    #[arg(long)]
    repo: String,

    #[arg(long)]
    ref_type: String,

    #[arg(long, default_value = "main")]
    source: String,

    #[arg(long)]
    ref_name: String,

    #[arg(long, default_value = "*")]
    pattern: String,
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
            if let Err(e) = write_jz_fs(args).await {
                let _ = shutdown_tx
                    .send(Err(anyhow!("start data controller {e}")))
                    .await;
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

    shutdown_rx.recv().await;
    info!("gracefully shutdown");
    Ok(())
}

async fn write_jz_fs(args: Args) -> Result<()> {
    let mut configuration = apis::configuration::Configuration::default();
    configuration.base_path = args.jiaozifs_url;
    configuration.basic_auth = Some((args.username, Some(args.password)));

    match args.ref_type.as_str() {
        "wip" => {
            //ensure wip exit
            apis::wip_api::get_wip(&configuration, &args.owner, &args.repo, &args.ref_name).await?;
            Ok(RefType::Wip)
        }
        val => Err(anyhow!("ref type not support {}", val)),
    }?;

    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    loop {
        let id = client.request_avaiable_data().await?;
        let path_str = args.tmp_path.clone() + "/" + &id + "-input";
        let root_input_dir = Path::new(&path_str);

        for entry in WalkDir::new(root_input_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                let content = fs::read(path).await?;
                let rel_path = path.strip_prefix(root_input_dir)?;
                apis::objects_api::upload_object(
                    &configuration,
                    &args.owner,
                    &args.repo,
                    &args.ref_name,
                    rel_path.to_str().anyhow("path must be validate")?,
                    Some(true),
                    Some(content),
                )
                .await?;
            }
        }
        client.submit_result(&id).await?;
    }
}
