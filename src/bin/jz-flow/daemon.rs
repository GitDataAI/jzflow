use anyhow::Result;
use clap::Args;

use jiaoziflow::{
    api::server::start_rpc_server,
    core::db::MainDbRepo,
    dbrepo::{
        MongoMainDbRepo,
        MongoRunDbRepo,
    },
    driver::kube::KubeDriver,
};
use kube::Client;
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
};

use jiaoziflow::job::job_mgr::JobManager;

use crate::global::GlobalOptions;

#[derive(Debug, Args)]
pub(super) struct DaemonArgs {
    #[arg(
        long,
        default_value = "mongodb://127.0.0.1:27017",
        help = "mongo connection string"
    )]
    mongo_url: String,
}

pub(super) async fn run_daemon(global_opts: GlobalOptions, args: DaemonArgs) -> Result<()> {
    let mut join_set: JoinSet<Result<()>> = JoinSet::new();
    let token = CancellationToken::new();

    let db_url = args.mongo_url.clone() + "/jiaoziflow";
    let db_repo = MongoMainDbRepo::new(db_url.as_str()).await?;
    let client = Client::try_default().await.unwrap();

    let driver = KubeDriver::new(client.clone(), args.mongo_url.as_str()).await?;
    let job_manager =
        JobManager::<KubeDriver<MongoRunDbRepo>, MongoMainDbRepo, MongoRunDbRepo>::new(
            client,
            &args.mongo_url,
            driver,
            db_repo.clone(),
        )
        .await?;

    job_manager.run_backend(&mut join_set, token.clone())?;
    let server = start_rpc_server(&global_opts.listen, db_repo, job_manager)?;
    let handler = server.handle();
    {
        let token = token.clone();
        let handler = handler.clone();
        join_set.spawn(async move {
            info!("start ipc server {}", &global_opts.listen);
            tokio::spawn(server);
            select! {
                _ = token.cancelled() => {
                    handler.stop(true).await;
                   info!("rpc server stopped");
                }
            };
            anyhow::Ok(())
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

    while let Some(Err(err)) = join_set.join_next().await {
        error!("exit spawn {err}");
    }
    info!("gracefully shutdown");
    Ok(())
}