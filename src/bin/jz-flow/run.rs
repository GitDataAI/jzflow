use anyhow::Result;
use clap::{
    Args,
    Parser,
    Subcommand,
};

use jz_action::{
    api::{
        self,
        server::start_rpc_server,
    },
    core::db::MainDbRepo,
    dbrepo::{
        MongoMainDbRepo,
        MongoRunDbRepo,
    },
    driver::kube::KubeDriver,
    utils::StdIntoAnyhowResult,
};
use kube::Client;
use std::{
    path::Path,
    str::FromStr,
};
use tokio::{
    fs,
    io::AsyncWriteExt,
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    task::JoinSet,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    Level,
};

use jz_action::job::job_mgr::JobManager;

use crate::global::GlobalOptions;

#[derive(Debug, Args)]
pub(super) struct RunArgs {
    #[arg(short, long)]
    mongo_url: String,
}

pub(super) async fn run_backend(global_opts: GlobalOptions, args: RunArgs) -> Result<()> {
    let mut join_set: JoinSet<Result<()>> = JoinSet::new();
    let token = CancellationToken::new();

    let db_url = args.mongo_url.to_string() + "jz_action";
    let db_repo = MongoMainDbRepo::new(db_url.as_str()).await?;
    let client = Client::try_default().await.unwrap();

    let driver = KubeDriver::new(client.clone(), db_url.as_str()).await?;
    let job_manager =
        JobManager::<KubeDriver<MongoRunDbRepo>, MongoMainDbRepo, MongoRunDbRepo>::new(
            client,
            driver,
            db_repo.clone(),
            &args.mongo_url,
        )
        .await?;
    let server = start_rpc_server(&global_opts.listen, db_repo, job_manager).unwrap();
    let handler = server.handle();
    {
        //listen unix socket
        let token = token.clone();
        let handler = handler.clone();
        join_set.spawn(async move {
            info!("start ipc server {}", &global_opts.listen);
            tokio::spawn(server);
            select! {
                _ = token.cancelled() => {
                    handler.stop(true).await;
                   info!("rpc server stopped");
                   return Ok(());
                }
            };
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
