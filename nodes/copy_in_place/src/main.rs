use anyhow::Result;
use clap::Parser;
use compute_unit_runner::ipc::{
    self,
    IPCClient,
    SubmitOuputDataReq,
};
use jz_action::utils::StdIntoAnyhowResult;
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
    error,
    info,
    Level,
};

#[derive(Debug, Parser)]
#[command(
    name = "copy_in_place",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-action>",
    about = "embed in k8s images. move input directory to dest directory."
)]

struct Args {
    #[arg(short, long, default_value = "INFO")]
    log_level: String,

    #[arg(short, long, default_value = "/unix_socket/compute_unit_runner_d")]
    unix_socket_addr: String,

    #[arg(short, long, default_value = "/app/tmp")]
    tmp_path: String,
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
        join_set.spawn(async move { copy_in_place(token, args).await });
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

async fn copy_in_place(token: CancellationToken, args: Args) -> Result<()> {
    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    let tmp_path = Path::new(&args.tmp_path);
    loop {
        select! {
            _ = token.cancelled() => {
                return Ok(());
             }
            else => {
                let instant = Instant::now();

                let req = client.request_avaiable_data().await?;
                if req.is_none() {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }

                let id = req.unwrap().id;
                let path_str = tmp_path.join(&id);
                let root_input_dir = path_str.as_path();

                let new_id = uuid::Uuid::new_v4().to_string();
                let output_dir = tmp_path.join(&new_id);

                fs::rename(root_input_dir, output_dir).await?;

                info!("move data {:?}", instant.elapsed());

                client.complete_result(&id).await?;

                //submit directory after completed a batch
                client
                    .submit_output(SubmitOuputDataReq::new(&new_id, 30))
                    .await?;
                info!("submit new data {:?}", instant.elapsed());

            }
        }
    }
}
