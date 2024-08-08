mod channel_tracker;
mod state_controller;
mod stream;

use jz_action::{
    dbrepo::MongoRunDbRepo,
    network::datatransfer::data_stream_server::DataStreamServer,
    utils::StdIntoAnyhowResult,
};

use anyhow::Result;
use channel_tracker::ChannelTracker;
use clap::Parser;
use jz_action::core::db::NodeRepo;
use nodes_sdk::fs_cache::*;
use state_controller::StateController;
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
    sync::RwLock,
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
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

    let mut join_set = JoinSet::new();
    let token = CancellationToken::new();

    let db_repo = MongoRunDbRepo::new(&args.mongo_url).await?;
    let node = db_repo.get_node_by_name(&args.node_name).await?;

    let fs_cache: Arc<dyn FileCache> = match args.tmp_path {
        Some(path) => Arc::new(FSCache::new(path)),
        None => Arc::new(MemCache::new()),
    };

    let addr = args.host_port.parse()?;
    let mut program = ChannelTracker::new(
        db_repo.clone(),
        fs_cache,
        &args.node_name,
        args.buf_size,
        node.up_nodes,
        node.incoming_streams,
        node.outgoing_streams,
    );
    program.run_backend(&mut join_set, token.clone())?;

    let program_safe = Arc::new(RwLock::new(program));
    {
        let program_safe = program_safe.clone();
        let node_name = args.node_name.clone();
        let cloned_token = token.clone();

        let state_ctl = StateController {
            program: program_safe,
        };
        join_set.spawn(async move {
            state_ctl
                .apply_db_state(cloned_token, db_repo, &node_name)
                .await
        });
    }

    {
        //listen port
        join_set.spawn(async move {
            let data_stream = ChannelDataStream {
                program: program_safe,
            };

            Server::builder()
                .add_service(DataStreamServer::new(data_stream))
                .serve(addr)
                .await
                .anyhow()
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
            token.cancel();
        });
    }

    nodes_sdk::monitor_tasks(&mut join_set).await
}
