use jz_action::network::datatransfer::{
    data_stream_client::DataStreamClient, MediaDataBatchResponse,
};
use std::collections::{hash_map::Entry, HashMap};
use tokio::time::Instant;
use tonic::transport::Channel;
use tracing::error;

pub struct MultiSender {
    streams: Vec<String>,

    connects: Vec<Option<DataStreamClient<Channel>>>,
}

impl MultiSender {
    pub fn new(streams: Vec<String>) -> Self {
        let connects = streams.iter().map(|_| None).collect();
        MultiSender {
            streams,
            connects: connects,
        }
    }
}

impl MultiSender {
    pub async fn send(&mut self, val: MediaDataBatchResponse) -> Result<(), Vec<String>> {
        let mut sent = vec![];
        for (index, stream) in self.connects.iter_mut().enumerate() {
            let url = self.streams[index].clone();
            if stream.is_none() {
                match DataStreamClient::connect(url.clone()).await {
                    Ok(client) => {
                        *stream = Some(client);
                    }
                    Err(err) => {
                        error!("connect data streams {url} {err}");
                        continue;
                    }
                }
            }

            let client = stream.as_mut().unwrap();
            let now = Instant::now();
            if let Err(err) = client.transfer_media_data(val.clone()).await {
                error!("send reqeust will try next time {url} {err}");
                continue;
            }
            println!("send one success {:?}", now.elapsed());
            sent.push(url);
        }

        if sent.len() == self.streams.len() {
            Ok(())
        } else {
            Err(sent)
        }
    }
}
