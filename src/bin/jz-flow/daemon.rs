use std::str::FromStr;

use anyhow::Result;
use clap::Args;

use jiaoziflow::{
    api::server::start_rpc_server,
    core::AccessMode,
    dbrepo::{
        MongoMainDbRepo,
        MongoRunDbRepo,
    },
    driver::{
        kube_derive::KubeDriver,
        kube_option::KubeOptions,
    },
    job::job_mgr::JobManager,
};
use kube::Client;
#[cfg(target_os = "linux")]
use tokio::signal::unix::{
    signal,
    SignalKind,
};
#[cfg(target_os = "windows")]
use tokio::signal::windows::{
    ctrl_break,
    ctrl_c,
    ctrl_shutdown,
};
use tokio::{
    select,
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
};

use crate::global::GlobalOptions;

#[derive(Debug, Args)]
pub(super) struct DaemonArgs {
    #[arg(
        long,
        default_value = "mongodb://127.0.0.1:27017",
        help = "mongo connection string"
    )]
    mongo_url: String,

    #[arg(
        long,
        default_value = "ReadWriteMany",
        help = "specify storage class type(ReadWriteMany, ReadWriteOnce)"
    )]
    access_mode: String,

    #[arg(
        long,
        default_value = "jz-flow-fs",
        help = "specify storage class name"
    )]
    storage_class_name: String,
}

pub(super) async fn run_daemon(global_opts: GlobalOptions, args: DaemonArgs) -> Result<()> {
    let mut join_set: JoinSet<Result<()>> = JoinSet::new();
    let token = CancellationToken::new();

    let db_url = args.mongo_url.clone() + "/jiaoziflow";
    let db_repo = MongoMainDbRepo::new(db_url.as_str()).await?;
    let client = Client::try_default().await.unwrap();

    let kube_opts = KubeOptions::default()
        .set_db_url(&args.mongo_url)
        .set_storage_class(&args.storage_class_name)
        .set_access_mode(AccessMode::from_str(&args.access_mode)?);

    let driver = KubeDriver::new(client.clone(), kube_opts).await?;
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
        #[cfg(target_os = "linux")]
        tokio::spawn(async move {
            let mut sig_term = signal(SignalKind::terminate()).unwrap();
            let mut sig_int = signal(SignalKind::interrupt()).unwrap();
            #[cfg(target_os = "linux")]
            select! {
                _ = sig_term.recv() => info!("Receive SIGTERM"),
                _ = sig_int.recv() => info!("Receive SIGINT"),
            };
            token.cancel();
        });
        #[cfg(target_os = "windows")]
        tokio::spawn(async move {
            let mut ctrl_c = ctrl_c().unwrap();
            let ctrl_c = ctrl_c.recv();
            let mut ctrl_break = ctrl_break().unwrap();
            let ctrl_break = ctrl_break.recv();
            let mut ctrl_shutdown = ctrl_shutdown().unwrap();
            let ctrl_shutdown = ctrl_shutdown.recv();
            #[cfg(target_os = "windows")]
            select! {
                _ = ctrl_c => info!("Receive Ctrl-C"),
                _ = ctrl_break => info!("Receive Ctrl-Break"),
                _ = ctrl_shutdown => info!("Receive Ctrl-Shutdown"),
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
