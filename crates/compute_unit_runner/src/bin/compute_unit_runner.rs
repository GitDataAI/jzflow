use compute_unit_runner::{ipc, media_data_tracker, unit};
use jz_action::dbrepo::mongo::MongoRepo;
use jz_action::network::datatransfer::data_stream_server::DataStreamServer;
use jz_action::utils::StdIntoAnyhowResult;

use anyhow::{anyhow, Result};
use clap::Parser;
use media_data_tracker::MediaDataTracker;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tonic::transport::Server;
use tracing::{info, Level};
use unit::UnitDataStream;

#[derive(Debug, Parser)]
#[command(
    name = "compute_unit_runner",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-action>",
    about = "embed in k8s images. work for process data input and output"
)]
struct Args {
    #[arg(short, long, default_value = "INFO")]
    log_level: String,

    #[arg(short, long, default_value = "/app/tmp")]
    tmp_path: String,

    #[arg(short, long)]
    node_name: String,

    #[arg(short, long)]
    mongo_url: String,

    #[arg(short, long)]
    database: String,

    #[arg(short, long, default_value = "/unix_socket/compute_unit_runner_d")]
    unix_socket_addr: String,

    #[arg(long, default_value = "[::1]:25431")]
    host_port: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(&args.log_level)?)
        .try_init()
        .anyhow()?;

    let db_repo = Arc::new(MongoRepo::new(&args.mongo_url, &args.database).await?);

    let program = MediaDataTracker::new(
        db_repo.clone(),
        &args.node_name,
        PathBuf::from_str(args.tmp_path.as_str())?,
    );

    let program_safe = Arc::new(Mutex::new(program));
    let data_stream = UnitDataStream::<Arc<MongoRepo>>::new(program_safe.clone());

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<Result<()>>(1);
    {
        let shutdown_tx = shutdown_tx.clone();
        let program_safe = program_safe.clone();
        let node_name = args.node_name.clone();
        let _ = tokio::spawn(async move {
            if let Err(err) = MediaDataTracker::<Arc<MongoRepo>>::apply_db_state(
                db_repo,
                &node_name,
                program_safe,
            )
            .await
            {
                let _ = shutdown_tx
                    .send(Err(anyhow!("start data controller {err}")))
                    .await;
            }
        });
    }

    {
        let addr = args.host_port.parse()?;
        //listen port
        let shutdown_tx = shutdown_tx.clone();
        let _ = tokio::spawn(async move {
            if let Err(e) = Server::builder()
                .add_service(DataStreamServer::new(data_stream))
                .serve(addr)
                .await
                .anyhow()
            {
                let _ = shutdown_tx
                    .send(Err(anyhow!("start data controller {e}")))
                    .await;
            }
        });

        info!("node listening on {}", addr);
    }

    {
        //listen unix socket
        let unix_socket_addr = args.unix_socket_addr.clone();
        let program = program_safe.clone();
        let shutdown_tx = shutdown_tx.clone();
        let _ = tokio::spawn(async move {
            if let Err(e) = ipc::start_ipc_server(unix_socket_addr, program) {
                let _ = shutdown_tx
                    .send(Err(anyhow!("start unix socket server {e}")))
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
