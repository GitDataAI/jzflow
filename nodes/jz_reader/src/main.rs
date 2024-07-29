use anyhow::{anyhow, Result};
use clap::Parser;
use compute_unit_runner::ipc::{self, IPCClient};
use jiaozifs_client_rs::apis;
use jiaozifs_client_rs::models::RefType;
use jz_action::utils::StdIntoAnyhowResult;
use std::path::Path;
use std::str::FromStr;
use tokio::fs;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::{info, Level};

#[derive(Debug, Parser)]
#[command(
    name = "jz_reader",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-action>",
    about = "embed in k8s images. "
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
            if let Err(e) = read_jz_fs(args).await {
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

async fn read_jz_fs(args: Args) -> Result<()> {
    let ref_type = match args.ref_type.as_str() {
        "branch" => Ok(RefType::Branch),
        "wip" => Ok(RefType::Wip),
        "tag" => Ok(RefType::Tag),
        "commit" => Ok(RefType::Commit),
        val => Err(anyhow!("ref type not correct {}", val)),
    }?;

    let mut configuration = apis::configuration::Configuration::default();
    configuration.base_path = args.jiaozifs_url;
    configuration.basic_auth = Some((args.username, Some(args.password)));
    let file_paths = apis::objects_api::get_files(
        &configuration,
        &args.owner,
        &args.repo,
        &args.ref_name,
        ref_type,
        Some("*"),
    )
    .await?;

    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    let tmp_path = Path::new(&args.tmp_path);
    for batch in file_paths.chunks(args.batch_size) {
        //create temp output directory
        let id = uuid::Uuid::new_v4().to_string();
        let output_dir = tmp_path.join(&id);
        fs::create_dir_all(output_dir.clone()).await?;

        //read file from jzfs and write to output directory
        let mut files = vec![];
        for path in batch {
            let mut byte_stream = apis::objects_api::get_object(
                &configuration,
                &args.owner,
                &args.repo,
                &args.ref_name,
                path.as_str(),
                ref_type,
                None,
            )
            .await?;

            let file_path = output_dir.as_path().join(&path);
            let mut tmp_file = fs::File::create(file_path).await?;
            while let Some(item) = byte_stream.next().await {
                tokio::io::copy(&mut item.unwrap().as_ref(), &mut tmp_file)
                    .await
                    .unwrap();
            }
            files.push(path);
        }

        //submit directory after completed a batch
        client.submit_output(&id).await?;
    }
    Ok(())
}
