use anyhow::{
    anyhow,
    Result,
};
use clap::Parser;
use compute_unit_runner::ipc::{
    self,
    ErrorNumber,
    IPCClient,
    IPCError,
};
use jiaozifs_client_rs::{
    apis::{
        self,
        configuration::Configuration,
        ResponseContent,
    },
    models::{
        BranchCreation,
        CreateRepository,
    },
};
use jz_flow::utils::{
    IntoAnyhowResult,
    StdIntoAnyhowResult,
};
use std::{
    path::Path,
    str::FromStr,
    time::Duration,
};
use tokio::{
    fs,
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    task::JoinSet,
    time::{
        sleep,
        Instant,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
    Level,
};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(
    name = "jz_writer",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-flow>",
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
    ref_name: String,

    #[arg(long, default_value = "*", help = "gob format path match string")]
    pattern: String,

    #[arg(long, help = "dont overide file if exit")]
    no_overide: bool,

    #[arg(long, help = "create repo branch if not exit")]
    create_if_not_exit: bool,

    #[arg(
        long,
        default_value = "main",
        help = "create repo base on this branch, must set --create-if-not-exit"
    )]
    source: String,
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

    {
        let token = token.clone();
        join_set.spawn(async move { write_jz_fs(token, args).await });
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

    nodes_sdk::monitor_tasks(&mut join_set).await
}

async fn write_jz_fs(token: CancellationToken, args: Args) -> Result<()> {
    let configuration = &Configuration {
        base_path: args.jiaozifs_url.clone(),
        basic_auth: Some((args.username.clone(), Some(args.password.clone()))),
        ..Default::default()
    };

    let repo_exists =
        match apis::repo_api::get_repository(configuration, &args.owner, &args.repo).await {
            Err(apis::Error::ResponseError(ResponseContent {
                status: reqwest::StatusCode::NOT_FOUND,
                ..
            })) => {
                if args.create_if_not_exit {
                    apis::repo_api::create_repository(
                        configuration,
                        CreateRepository {
                            name: args.repo.clone(),
                            description: Some("created by jz writer".to_string()),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(|err| anyhow!("create repository {err}"))?;
                    false
                } else {
                    return Err(anyhow!("repo not exit"));
                }
            }
            Err(err) => return Err(anyhow!("Failed to query repository: {}", err)),
            Ok(_) => true,
        };

    if repo_exists {
        if let Err(err) =
            apis::branches_api::get_branch(configuration, &args.owner, &args.repo, &args.ref_name)
                .await
        {
            match err {
                apis::Error::ResponseError(ResponseContent {
                    status: reqwest::StatusCode::NOT_FOUND,
                    ..
                }) => {
                    if args.create_if_not_exit {
                        create_branch(configuration, &args)
                            .await
                            .map_err(|err| anyhow!("create branch {err}"))?;
                    } else {
                        return Err(anyhow!("branch not exit"));
                    }
                }
                _ => return Err(anyhow!("Failed to get branch: {}", err)),
            }
        }
    } else {
        create_branch(configuration, &args)
            .await
            .map_err(|err| anyhow!("create branch {err}"))?;
    }

    apis::wip_api::get_wip(configuration, &args.repo, &args.owner, &args.ref_name)
        .await
        .map_err(|err| anyhow!("get wip {err}"))?;

    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    let tmp_path = Path::new(&args.tmp_path);
    loop {
        if token.is_cancelled() {
            return anyhow::Ok(());
        }

        let instant = Instant::now();
        match client.request_avaiable_data(None).await {
            Ok(Some(req)) => {
                let id = req.id;
                let path_str = tmp_path.join(&id);
                let root_input_dir = path_str.as_path();
                for entry in WalkDir::new(root_input_dir) {
                    let entry = entry?;
                    if entry.file_type().is_file() {
                        let path = entry.path();
                        let content = fs::read(path).await?;
                        let rel_path = path.strip_prefix(root_input_dir)?;
                        apis::objects_api::upload_object(
                            configuration,
                            &args.owner,
                            &args.repo,
                            &args.ref_name,
                            rel_path.to_str().anyhow("path must be validate")?,
                            Some(true),
                            Some(content),
                        )
                        .await?;
                        debug!("upload file {path:?} to jiaozifs");
                    }
                }
                client.complete_result(&id).await.anyhow()?;
                info!("process a batch data {:?}", instant.elapsed());
            }
            Ok(None) => {
                sleep(Duration::from_secs(2)).await;
                continue;
            }
            Err(IPCError::NodeError { code, msg: _ }) => match code {
                ErrorNumber::AlreadyFinish => {
                    info!("receive AlreadyFinish");
                    return Ok(());
                }
                ErrorNumber::NotReady => {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
                ErrorNumber::DataNotFound => {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
                ErrorNumber::InComingFinish => {
                    apis::wip_api::commit_wip(
                        configuration,
                        &args.owner,
                        &args.repo,
                        &args.ref_name,
                        "created by jz action",
                    )
                    .await?;
                    client.finish().await.anyhow()?;
                    info!("all data finish");
                    return Ok(());
                }
            },
            Err(IPCError::UnKnown(msg)) => {
                error!("got unknow error {msg}");
            }
        }
    }
}

async fn create_branch(configuration: &Configuration, args: &Args) -> Result<()> {
    if let Err(err) = apis::branches_api::create_branch(
        configuration,
        &args.owner,
        &args.repo,
        BranchCreation {
            name: args.ref_name.clone(),
            source: args.source.clone(),
        },
    )
    .await
    {
        match err {
            apis::Error::ResponseError(ResponseContent {
                status: reqwest::StatusCode::CONFLICT,
                ..
            }) => Ok(()),
            _ => Err(anyhow!("Failed to create branch: {}", err)),
        }
    } else {
        Ok(())
    }
}
