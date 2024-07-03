use anyhow::{anyhow, Ok, Result};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use jz_action::network::datatransfer::DataBatchResponse;
use std::process::Command;
use std::{
    os::unix::process::CommandExt,
    sync::{Arc, Mutex},
};
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use tracing::error;

#[derive(Debug)]
pub(crate) enum ProgramState {
    Init,
    Ready,
    Pending,
    Finish,
    Stopped,
}

pub(crate) struct BatchProgram {
    pub(crate) state: ProgramState,

    pub(crate) upstreams: Option<Vec<String>>,

    pub(crate) script: Option<String>,

    pub(crate) tx: broadcast::Sender<DataBatchResponse>, //receive data from upstream and send it to program with this
}

impl BatchProgram {
    pub(crate) fn new() -> Self {
        let tx = broadcast::Sender::new(128);
        BatchProgram {
            state: ProgramState::Init,
            upstreams: None,
            script: None,
            tx: tx,
        }
    }

    pub(crate) async fn fetch_data(&self) -> Result<()> {
        if self.upstreams.is_none() {
            return Err(anyhow!("no upstream"));
        }

        let (tx, mut rx) = mpsc::channel(1024);

        for upstream in self.upstreams.as_ref().unwrap() {
            let upstream_clone = upstream.clone();
            let tx_clone = tx.clone();
            let _ = tokio::spawn(async move {
                let mut client = DataStreamClient::connect(upstream_clone).await?;
                let mut stream = client.subscribe_new_data(Empty {}).await?.into_inner();

                while let Some(item) = stream.next().await {
                    tx_clone.send(item.unwrap()).await.unwrap();
                }

                error!("unable read data from stream");
                Ok(())
            });
        }

        let tx = self.tx.clone();
        let _ = tokio::spawn(async move {
            loop {
                select! {
                 data_batch = rx.recv() => {
                    if let Some(v) = data_batch {
                        if let Err(e) = tx.send(v) {
                            error!("send data {}", e);
                        }
                    }
                 },
                }
            }
        });
        Ok(())
    }

    fn run_one_batch(&self) -> Result<()> {
        if self.script.is_none() {
            return Err(anyhow!("script not found"));
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(self.script.clone().unwrap())
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("{}", String::from_utf8_lossy(&output.stderr)));
        }
        Ok(())
    }
}
