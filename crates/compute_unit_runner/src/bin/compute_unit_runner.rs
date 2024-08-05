use compute_unit_runner::{
    fs_cache::{
        FSCache,
        FileCache,
    },
    ipc,
    media_data_tracker,
};
use jz_action::{
    dbrepo::mongo::{
        MongoConfig,
        MongoRepo,
    },
    utils::StdIntoAnyhowResult,
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
    sync::Mutex,
    task::JoinSet,
};

use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    Level,
};

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

    #[arg(short, long)]
    tmp_path: Option<String>,

    #[arg(short, long, default_value = "30")]
    buf_size: usize,

    #[arg(short, long)]
    node_name: String,

    #[arg(short, long)]
    mongo_url: String,

    #[arg(short, long)]
    database: String,

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

    let db_repo = MongoRepo::new(MongoConfig::new(args.mongo_url.clone()), &args.database).await?;

    let program = MediaDataTracker::new(db_repo.clone(), &args.node_name, fs_cache, args.buf_size);

    let program_safe = Arc::new(Mutex::new(program));
    {
        let program_safe = program_safe.clone();
        let node_name = args.node_name.clone();
        let cloned_token = token.clone();
        join_set.spawn(async move {
            MediaDataTracker::<MongoRepo>::apply_db_state(
                cloned_token,
                db_repo,
                &node_name,
                program_safe,
            )
            .await
        });
    }

    {
        //listen unix socket
        let unix_socket_addr = args.unix_socket_addr.clone();
        let program = program_safe.clone();
        join_set.spawn(async move {
            info!("start ipc server {}", &unix_socket_addr);
            ipc::start_ipc_server(unix_socket_addr, program).await
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
            token.cancel();
        });
    }

    while let Some(Err(err)) = join_set.join_next().await {
        error!("exit spawn {err}");
    }

    info!("gracefully shutdown");
    Ok(())
}
