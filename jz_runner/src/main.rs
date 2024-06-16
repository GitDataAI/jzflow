mod unit;

use jz_action::network::datatransfer::data_stream_server::DataStreamServer;
use jz_action::network::nodecontroller::node_controller_server::NodeControllerServer;

use anyhow::{anyhow, Result};
use clap::Parser;
use std::str::FromStr;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, Level};

use unit::{DataNodeControllerServer, UnitDataStrean};
#[derive(Debug, Parser)]
#[command(
    name = "jz_runner",
    version = "0.0.1",
    author = "Author Name <github.com/GitDataAI/jz-action>",
    about = "embed in k8s images"
)]
struct Args {
    #[arg(short, long, default_value = "INFO")]
    log_level: String,

    #[arg(long, default_value = "[::1]:25431")]
    host_port: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(&args.log_level)?)
        .try_init()
        .map_err(|e| anyhow!("{}", e))?;

    let addr = args.host_port.parse()?;
    let node_controller = DataNodeControllerServer::default();
    let unit_data_stream = UnitDataStrean::default();

    Server::builder()
        .add_service(NodeControllerServer::new(node_controller))
        .add_service(DataStreamServer::new(unit_data_stream))
        .serve(addr)
        .await?;
    info!("node listening on {}", addr);
    Ok(())
}
