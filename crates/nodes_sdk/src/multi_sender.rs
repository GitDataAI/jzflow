use jz_action::network::datatransfer::{
    data_stream_client::DataStreamClient,
    MediaDataBatchResponse,
};
use tokio::time::Instant;
use tonic::transport::Channel;
use tracing::{
    debug,
    error,
};

pub struct MultiSender {
    streams: Vec<String>,

    connects: Vec<Option<DataStreamClient<Channel>>>,
}

impl MultiSender {
    pub fn new(streams: Vec<String>) -> Self {
        let connects = streams.iter().map(|_| None).collect();
        MultiSender { streams, connects }
    }
}

impl MultiSender {
    pub async fn send(
        &mut self,
        val: MediaDataBatchResponse,
        sent_nodes: &[&str],
    ) -> Result<(), Vec<String>> {
        let mut sent = vec![];
        for (index, stream) in self.connects.iter_mut().enumerate() {
            let url = &self.streams[index];
            if sent_nodes.contains(&url.as_str()) {
                debug!("{} has sent before, skip it", url);
                sent.push(url.to_string());
                continue;
            }

            if stream.is_none() {
                match DataStreamClient::connect(url.to_string()).await {
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
            sent.push(url.to_string());
        }

        if sent.len() == self.streams.len() {
            Ok(())
        } else {
            Err(sent)
        }
    }
}
