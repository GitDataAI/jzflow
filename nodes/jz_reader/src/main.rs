use anyhow::{
    anyhow,
    Result,
};
use clap::Parser;
use compute_unit_runner::ipc::{
    self,
    IPCClient,
    SubmitOuputDataReq,
};
use jiaozifs_client_rs::{
    apis::{
        self,
        configuration::Configuration,
    },
    models::RefType,
};
use jz_flow::utils::StdIntoAnyhowResult;
use std::{
    path::Path,
    str::FromStr,
    time::Instant,
};
use tokio::{
    fs,
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    task::JoinSet,
};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{
    info,
    Level,
};

#[derive(Debug, Parser)]
#[command(
    name = "jz_reader",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-flow>",
    about = "embed in k8s images. "
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
    ref_type: String,

    #[arg(long)]
    ref_name: String,

    #[arg(long, default_value = "*")]
    pattern: String,
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
        join_set.spawn(async move { read_jz_fs(args).await });
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

async fn read_jz_fs(args: Args) -> Result<()> {
    let ref_type = match args.ref_type.as_str() {
        "branch" => Ok(RefType::Branch),
        "wip" => Ok(RefType::Wip),
        "tag" => Ok(RefType::Tag),
        "commit" => Ok(RefType::Commit),
        val => Err(anyhow!("ref type not correct {}", val)),
    }?;

    let configuration = Configuration {
        base_path: args.jiaozifs_url,
        basic_auth: Some((args.username, Some(args.password))),
        ..Default::default()
    };

    let file_paths = apis::objects_api::get_files(
        &configuration,
        &args.owner,
        &args.repo,
        &args.ref_name,
        ref_type,
        Some(&args.pattern),
    )
    .await?;

    info!("get files {} in jizozifs", file_paths.len());

    let client = ipc::IPCClientImpl::new(args.unix_socket_addr);
    let tmp_path = Path::new(&args.tmp_path);
    for batch in file_paths.chunks(args.batch_size) {
        //create temp output directory
        let id = uuid::Uuid::new_v4().to_string();
        let output_dir = tmp_path.join(&id);
        fs::create_dir_all(output_dir.clone()).await?;

        let instant = Instant::now();
        //read file from jzfs and write to output directory
        for path in batch {
            let mut byte_stream = apis::objects_api::get_object(
                &configuration,
                &args.owner,
                &args.repo,
                &args.ref_name,
                path.as_str(),
                ref_type,
                None,
            )
            .await?;

            let file_path = output_dir.as_path().join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            let mut tmp_file = fs::File::create(file_path).await?;
            while let Some(item) = byte_stream.next().await {
                tokio::io::copy(&mut item.unwrap().as_ref(), &mut tmp_file)
                    .await
                    .unwrap();
            }
        }
        info!(
            "read a batch files {} spent {:?}",
            batch.len(),
            instant.elapsed()
        );

        //submit directory after completed a batch
        client
            .submit_output(SubmitOuputDataReq::new(&id, batch.len() as u32))
            .await
            .anyhow()?;
        info!(
            "flush batch files {} spent {:?}",
            batch.len(),
            instant.elapsed()
        );
    }
    // read all files
    client.finish().await.unwrap();
    Ok(())
}
