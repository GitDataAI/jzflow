use crate::media_data_tracker::MediaDataTracker;
use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::anyhow;
use anyhow::Result;
use http_body_util::Collected;
use jz_action::core::models::NodeRepo;
use jz_action::utils::StdIntoAnyhowResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Method, Request};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AvaiableDataResponse {
    pub(crate) id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SubmitResultReq {
    pub(crate) id: String,
}

impl SubmitResultReq {
    fn new(id: &str) -> Self {
        SubmitResultReq { id: id.to_string() }
    }
}

async fn process_data_request<R>(
    program_mutex: web::Data<Arc<Mutex<MediaDataTracker<R>>>>,
) -> HttpResponse
where
    R: NodeRepo,
{
    let (tx, mut rx) = oneshot::channel::<AvaiableDataResponse>();
    let program = program_mutex.lock().await;
    program
        .ipc_process_data_req_tx
        .as_ref()
        .unwrap()
        .send(((), tx))
        .await
        .unwrap();

    match rx.try_recv() {
        Ok(resp) => HttpResponse::Ok().json(resp),
        Err(e) => HttpResponse::ServiceUnavailable().body(e.to_string()),
    }
}

async fn process_submit_result_request<R>(
    program_mutex: web::Data<Arc<Mutex<MediaDataTracker<R>>>>,
    data: web::Json<SubmitResultReq>,
) -> HttpResponse
where
    R: NodeRepo,
{
    let (tx, mut rx) = oneshot::channel::<()>();
    let program = program_mutex.lock().await;
    //read request
    program
        .ipc_process_submit_result_tx
        .as_ref()
        .unwrap()
        .send((data.0, tx))
        .await
        .unwrap();
    match rx.try_recv() {
        Ok(resp) => {
            //response result
            HttpResponse::Ok().body(resp)
        }
        Err(e) => HttpResponse::ServiceUnavailable().body(e.to_string()),
    }
}

pub fn start_ipc_server<R>(
    unix_socket_addr: String,
    program: Arc<Mutex<MediaDataTracker<R>>>,
) -> Result<()>
where
    R: NodeRepo + Send + Sync + 'static,
{
    HttpServer::new(move || {
        App::new()
            .app_data(program.clone())
            .service(web::resource("/api/v1/data").get(process_data_request::<R>))
            .service(web::resource("/api/v1/submit").post(process_submit_result_request::<R>))
    })
    .bind_uds(unix_socket_addr)
    .unwrap();

    Ok(())
}

pub trait IPCClient {
    async fn submit_result(&self, id: &str) -> Result<()>;
    async fn request_avaiable_data(&self) -> Result<String>;
}

pub struct IPCClientImpl {
    unix_socket_addr: String,
    client: Client<UnixConnector, Full<Bytes>>,
}

impl IPCClientImpl {
    pub fn new(unix_socket_addr: String) -> Self {
        IPCClientImpl {
            unix_socket_addr,
            client: Client::unix(),
        }
    }
}

impl IPCClient for IPCClientImpl {
    async fn submit_result(&self, id: &str) -> Result<()> {
        let req = SubmitResultReq::new(id);
        let json = serde_json::to_string(&req)?;

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::POST)
            .uri(self.unix_socket_addr.clone() + "/api/v1/submit")
            .body(Full::from(json))?;

        let resp = self.client.request(req).await.anyhow()?;
        if resp.status().is_success() {
            return Ok(());
        }

        Err(anyhow!("submit data fail {}", resp.status()))
    }

    async fn request_avaiable_data(&self) -> Result<String> {
        let url = Uri::new(self.unix_socket_addr.clone(), "/api/v1/data").into();
        let resp = self.client.get(url).await.anyhow()?;

        if !resp.status().is_success() {
            return Err(anyhow!("submit data fail {}", resp.status()));
        }

        let contents = resp.collect().await.map(Collected::to_bytes)?;
        let avaiabel_data: AvaiableDataResponse = serde_json::from_slice(contents.as_ref())?;
        Ok(avaiabel_data.id)
    }
}
