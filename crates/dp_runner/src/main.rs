mod channel_tracker;
mod stream;
use jz_action::{
    dbrepo::mongo::{
        MongoConfig,
        MongoRepo,
    },
    network::datatransfer::data_stream_server::DataStreamServer,
    utils::StdIntoAnyhowResult,
};

use anyhow::{
    anyhow,
    Result,
};
use channel_tracker::ChannelTracker;
use clap::Parser;
use compute_unit_runner::fs_cache::*;
use std::{
    str::FromStr,
    sync::Arc,
};
use stream::ChannelDataStream;
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::{
        mpsc,
        Mutex,
    },
};
use tonic::transport::Server;
use tracing::{
    error,
    info,
    Level,
};

#[derive(Debug, Parser)]
#[command(
    name = "dp_runner",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-action>",
    about = "embed in k8s images. make process data input output"
)]
struct Args {
    #[arg(short, long, default_value = "INFO")]
    log_level: String,

    #[arg(short, long)]
    tmp_path: Option<String>,

    #[arg(short, long)]
    node_name: String,

    #[arg(short, long)]
    mongo_url: String,

    #[arg(short, long)]
    database: String,

    #[arg(short, long, default_value = "30")]
    buf_size: usize,

    #[arg(long, default_value = "0.0.0.0:80")]
    host_port: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(&args.log_level)?)
        .try_init()
        .anyhow()?;

    let db_repo = MongoRepo::new(MongoConfig::new(args.mongo_url.clone()), &args.database).await?;

    let fs_cache: Arc<dyn FileCache> = match args.tmp_path {
        Some(path) => Arc::new(FSCache::new(path)),
        None => Arc::new(MemCache::new()),
    };

    let addr = args.host_port.parse()?;
    let program = ChannelTracker::new(db_repo.clone(), fs_cache, &args.node_name, args.buf_size);

    let program_safe = Arc::new(Mutex::new(program));
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<Result<()>>(1);
    {
        let shutdown_tx = shutdown_tx.clone();
        let program_safe = program_safe.clone();
        let node_name = args.node_name.clone();
        let _ = tokio::spawn(async move {
            if let Err(err) =
                ChannelTracker::<MongoRepo>::apply_db_state(db_repo, &node_name, program_safe).await
            {
                let _ = shutdown_tx.send(Err(anyhow!("apply db state {err}"))).await;
            }
        });
    }

    {
        //listen port
        let shutdown_tx_arc = shutdown_tx.clone();
        let _ = tokio::spawn(async move {
            let data_stream = ChannelDataStream {
                program: program_safe,
            };

            if let Err(e) = Server::builder()
                .add_service(DataStreamServer::new(data_stream))
                .serve(addr)
                .await
                .anyhow()
            {
                let _ = shutdown_tx_arc.send(Err(e)).await;
            }
        });

        info!("node listening on {}", addr);
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
