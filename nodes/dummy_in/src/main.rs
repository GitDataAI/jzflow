use anyhow::Result;
use clap::Parser;
use compute_unit_runner::ipc::{
    self,
    IPCClient,
    SubmitOuputDataReq,
};
use jz_action::utils::StdIntoAnyhowResult;
use random_word::Lang;
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

#[derive(Debug, Parser)]
#[command(
    name = "dummyu_in",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-action>",
    about = "embed in k8s images. "
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
        join_set.spawn(async move { dummy_in(token, args).await });
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

async fn dummy_in(token: CancellationToken, args: Args) -> Result<()> {
    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    let tmp_path = Path::new(&args.tmp_path);
    loop {
        if token.is_cancelled() {
            return Ok(());
        }

        let instant = Instant::now();
        let id = uuid::Uuid::new_v4().to_string();
        let output_dir = tmp_path.join(&id);
        fs::create_dir_all(output_dir.clone()).await?;
        for _ in 0..30 {
            let file_name = random_word::gen(Lang::En);
            let file_path = output_dir.as_path().join(file_name.to_string() + ".txt");
            let mut tmp_file = fs::File::create(file_path).await?;
            for _ in 0..100 {
                let word = random_word::gen(Lang::En).to_string() + "\n";
                tmp_file.write(word.as_bytes()).await.unwrap();
            }
        }

        info!("generate new data spent {:?}", instant.elapsed());
        //submit directory after completed a batch
        client
            .submit_output(SubmitOuputDataReq::new(&id, 30))
            .await?;
        info!("submit new data {:?}", instant.elapsed());
    }
}
