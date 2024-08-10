use compute_unit_runner::{
    ipc,
    media_data_tracker,
    state_controller::StateController,
};
use jz_flow::{
    core::db::NodeRepo,
    dbrepo::MongoRunDbRepo,
    utils::StdIntoAnyhowResult,
};
use nodes_sdk::fs_cache::{
    FSCache,
    FileCache,
};

use anyhow::Result;
use clap::Parser;
use media_data_tracker::MediaDataTracker;
use std::{
    str::FromStr,
    sync::Arc,
};
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::RwLock,
    task::JoinSet,
};

use nodes_sdk::monitor_tasks;
use tokio_util::sync::CancellationToken;
use tracing::{
    info,
    Level,
};

#[derive(Debug, Parser)]
#[command(
    name = "compute_unit_runner",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-flow>",
    about = "embed in k8s images. work for process data input and output"
)]
struct Args {
    #[arg(short, long, default_value = "INFO")]
    log_level: String,

    #[arg(short, long)]
    tmp_path: Option<String>,

    #[arg(short, long, default_value = "30")]
    buf_size: usize,

    #[arg(short, long)]
    node_name: String,

    #[arg(short, long)]
    mongo_url: String,

    #[arg(short, long, default_value = "/unix_socket/compute_unit_runner_d")]
    unix_socket_addr: String,
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

    let fs_cache: Arc<dyn FileCache> = Arc::new(FSCache::new(
        args.tmp_path.expect("compute node only support disk cache"),
    ));

    let db_repo = MongoRunDbRepo::new(&args.mongo_url).await?;

    let node = db_repo.get_node_by_name(&args.node_name).await?;

    let mut program = MediaDataTracker::new(
        db_repo.clone(),
        &args.node_name,
        fs_cache,
        args.buf_size,
        node.up_nodes,
        node.incoming_streams,
        node.outgoing_streams,
    );
    program.run_backend(&mut join_set, token.clone())?;

    let program_safe = Arc::new(RwLock::new(program));

    let server = ipc::start_ipc_server(&args.unix_socket_addr, program_safe.clone()).unwrap();
    let handler = server.handle();
    {
        //listen unix socket
        let token = token.clone();
        let handler = handler.clone();
        join_set.spawn(async move {
            info!("start ipc server {}", &args.unix_socket_addr);
            tokio::spawn(server);
            select! {
                _ = token.cancelled() => {
                    handler.stop(true).await;
                   info!("ipc server stopped");
                }
            };
            Ok::<(), anyhow::Error>(())
        });
    }
    {
        let program_safe = program_safe.clone();
        let node_name = args.node_name.clone();
        let cloned_token = token.clone();

        let state_ctl = StateController {
            program: program_safe,
            _handler: handler,
        };
        join_set.spawn(async move {
            state_ctl
                .apply_db_state(cloned_token, db_repo, &node_name)
                .await
        });
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
    monitor_tasks(&mut join_set).await
}
