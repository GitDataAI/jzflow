use crate::media_data_tracker::MediaDataTracker;
use actix_web::{
    dev::{
        Server,
        ServerHandle,
    },
    error,
    http::StatusCode,
    middleware,
    web::{
        self,
        Data,
    },
    App,
    HttpRequest,
    HttpResponse,
    HttpServer,
};
use anyhow::{
    anyhow,
    Result,
};
use core::str;
use http_body_util::Collected;
use jz_action::{
    core::models::{
        DbRepo,
        TrackerState,
    },
    utils::StdIntoAnyhowResult,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    os::unix::net::UnixListener,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{
        oneshot,
        Mutex,
    },
    time::sleep,
};
use tracing::info;

use http_body_util::{
    BodyExt,
    Full,
};
use hyper::{
    body::Bytes,
    Method,
    Request,
};
use hyper_util::client::legacy::Client;
use hyperlocal::{
    UnixClientExt,
    UnixConnector,
    Uri,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AvaiableDataResponse {
    pub id: String,
    pub size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteDataReq {
    pub id: String,
}

impl CompleteDataReq {
    pub fn new(id: &str) -> Self {
        CompleteDataReq { id: id.to_string() }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitOuputDataReq {
    pub id: String,
    pub size: u32,
}

impl SubmitOuputDataReq {
    pub fn new(id: &str, size: u32) -> Self {
        SubmitOuputDataReq {
            id: id.to_string(),
            size,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub state: TrackerState,
}

async fn status<R>(program_mutex: web::Data<Arc<Mutex<MediaDataTracker<R>>>>) -> HttpResponse
where
    R: DbRepo + Clone,
{
    info!("receive status request");
    let status = {
        let program = program_mutex.lock().await;
        Status {
            state: program.local_state.clone(),
        }
    };
    HttpResponse::Ok().json(status)
}

async fn process_data_request<R>(
    program_mutex: web::Data<Arc<Mutex<MediaDataTracker<R>>>>,
) -> HttpResponse
where
    R: DbRepo + Clone,
{
    info!("receive avaiable data reqeust");
    let (tx, rx) = oneshot::channel::<Result<Option<AvaiableDataResponse>>>();

    let sender = loop {
        let program = program_mutex.lock().await;
        if matches!(program.local_state, TrackerState::Ready) {
            break program.ipc_process_data_req_tx.as_ref().cloned();
        }
        drop(program);
        sleep(Duration::from_secs(5)).await;
    };

    //read request
    match sender {
        Some(sender) => {
            if let Err(err) = sender.send(((), tx)).await {
                return HttpResponse::ServiceUnavailable()
                    .body(format!("send to avaiable data channel {err}"));
            }
        }
        None => {
            return HttpResponse::ServiceUnavailable().body("channel is not ready");
        }
    }

    match rx.await {
        Ok(Ok(Some(resp))) => HttpResponse::Ok().json(resp),
        Ok(Ok(None)) => HttpResponse::NotFound().body("no avaiablle data"),
        Ok(Err(err)) => HttpResponse::ServiceUnavailable().body(err.to_string()),
        Err(err) => HttpResponse::ServiceUnavailable().body(err.to_string()),
    }
}

async fn process_completed_request<R>(
    program_mutex: web::Data<Arc<Mutex<MediaDataTracker<R>>>>,
    data: web::Json<CompleteDataReq>,
) -> HttpResponse
where
    R: DbRepo + Clone,
{
    info!("receive data completed request");
    let (tx, rx) = oneshot::channel::<Result<()>>();
    let sender = loop {
        let program = program_mutex.lock().await;
        if matches!(program.local_state, TrackerState::Ready) {
            break program.ipc_process_completed_data_tx.as_ref().cloned();
        }
        drop(program);
        sleep(Duration::from_secs(5)).await;
    };

    //read request
    match sender {
        Some(sender) => {
            if let Err(err) = sender.send((data.0, tx)).await {
                return HttpResponse::ServiceUnavailable()
                    .body(format!("send to avaiable data channel {err}"));
            }
        }
        None => {
            return HttpResponse::ServiceUnavailable().body("channel is not ready");
        }
    }

    match rx.await {
        Ok(Ok(resp)) => HttpResponse::Ok().body(resp),
        Ok(Err(err)) => HttpResponse::ServiceUnavailable().body(err.to_string()),
        Err(err) => HttpResponse::ServiceUnavailable().body(err.to_string()),
    }
}

async fn process_submit_output_request<R>(
    program_mutex: web::Data<Arc<Mutex<MediaDataTracker<R>>>>,
    data: web::Json<SubmitOuputDataReq>,
    //body: web::Bytes,
) -> HttpResponse
where
    R: DbRepo + Clone,
{
    // let data: SubmitOuputDataReq = serde_json::from_slice(&body).unwrap();

    info!("receive submit output request {}", &data.id);
    let (tx, rx) = oneshot::channel::<Result<()>>();
    let sender = loop {
        let program = program_mutex.lock().await;
        if matches!(program.local_state, TrackerState::Ready) {
            break program.ipc_process_submit_output_tx.as_ref().cloned();
        }
        drop(program);
        sleep(Duration::from_secs(5)).await;
    };

    //read request
    match sender {
        Some(sender) => {
            if let Err(err) = sender.send((data.0, tx)).await {
                return HttpResponse::ServiceUnavailable()
                    .body(format!("send to avaiable data channel {err}"));
            }
        }
        None => {
            return HttpResponse::ServiceUnavailable().body("channel is not ready");
        }
    }

    match rx.await {
        Ok(Ok(resp)) => HttpResponse::Ok().body(resp),
        Ok(Err(err)) => HttpResponse::ServiceUnavailable().body(err.to_string()),
        Err(err) => HttpResponse::ServiceUnavailable().body(err.to_string()),
    }
}

pub fn start_ipc_server<R>(
    unix_socket_addr: &str,
    program: Arc<Mutex<MediaDataTracker<R>>>,
) -> Result<Server>
where
    R: DbRepo + Clone + Send + Sync + 'static,
{
    let server = HttpServer::new(move || {
        fn json_error_handler(err: error::JsonPayloadError, _req: &HttpRequest) -> error::Error {
            use actix_web::error::JsonPayloadError;

            let detail = err.to_string();
            let resp = match &err {
                JsonPayloadError::ContentType => HttpResponse::UnsupportedMediaType().body(detail),
                JsonPayloadError::Deserialize(json_err) if json_err.is_data() => {
                    HttpResponse::UnprocessableEntity().body(detail)
                }
                _ => HttpResponse::BadRequest().body(detail),
            };
            error::InternalError::from_response(err, resp).into()
        }

        App::new()
            .wrap(middleware::Logger::default())
            .app_data(Data::new(program.clone()))
            .service(
                web::scope("/api/v1")
                    .service(web::resource("status").get(status::<R>))
                    .service(
                        web::resource("data")
                            .route(web::post().to(process_completed_request::<R>))
                            .route(web::get().to(process_data_request::<R>)),
                    )
                    .service(web::resource("submit").post(process_submit_output_request::<R>)),
            )
            .app_data(web::JsonConfig::default().error_handler(json_error_handler))
    })
    .disable_signals()
    .bind_uds(unix_socket_addr)?
    .run();

    Ok(server)
}

pub trait IPCClient {
    fn status(&self) -> impl std::future::Future<Output = Result<Status>> + Send;
    fn submit_output(
        &self,
        req: SubmitOuputDataReq,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn complete_result(&self, id: &str) -> impl std::future::Future<Output = Result<()>> + Send;
    fn request_avaiable_data(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<AvaiableDataResponse>>> + Send;
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
    async fn status(&self) -> Result<Status> {
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/status").into();

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::default())?;

        let resp = self.client.request(req).await.anyhow()?;
        if !resp.status().is_success() {
            let status_code = resp.status();
            let contents =
                String::from_utf8(resp.collect().await.map(Collected::to_bytes)?.to_vec())?;
            return Err(anyhow!("request status fail {} {}", status_code, contents));
        }

        let contents = resp.collect().await.map(Collected::to_bytes)?;
        let status: Status = serde_json::from_slice(contents.as_ref())?;
        Ok(status)
    }

    async fn submit_output(&self, req: SubmitOuputDataReq) -> Result<()> {
        let json = serde_json::to_string(&req)?;
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/submit").into();

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::from(json))?;
        let resp = self.client.request(req).await.anyhow()?;
        if resp.status().is_success() {
            return Ok(());
        }

        let status_code = resp.status();
        let contents = String::from_utf8(resp.collect().await.map(Collected::to_bytes)?.to_vec())?;
        Err(anyhow!("submit data fail {} {}", status_code, contents))
    }

    async fn request_avaiable_data(&self) -> Result<Option<AvaiableDataResponse>> {
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/data").into();
        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::default())?;

        let resp = self.client.request(req).await.anyhow()?;
        if resp.status().as_u16() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status_code = resp.status();
            let contents =
                String::from_utf8(resp.collect().await.map(Collected::to_bytes)?.to_vec())?;
            return Err(anyhow!(
                "get avaiable data fail {} {}",
                status_code,
                contents
            ));
        }

        let contents = resp.collect().await.map(Collected::to_bytes)?;
        let avaiabel_data: AvaiableDataResponse = serde_json::from_slice(contents.as_ref())?;
        Ok(Some(avaiabel_data))
    }

    async fn complete_result(&self, id: &str) -> Result<()> {
        let req = CompleteDataReq::new(id);
        let json = serde_json::to_string(&req)?;
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/data").into();

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::from(json))?;

        let resp = self.client.request(req).await.anyhow()?;
        if resp.status().is_success() {
            return Ok(());
        }

        let status_code = resp.status();
        let contents = String::from_utf8(resp.collect().await.map(Collected::to_bytes)?.to_vec())?;
        Err(anyhow!("completed data fail {} {}", status_code, contents))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tracing_subscriber;

    #[tokio::test]
    async fn test_render() {
        env::set_var("RUST_LOG", "DEBUG");
        tracing_subscriber::fmt::init();

        let client = IPCClientImpl::new("/home/hunjixin/code/jz-action/test.d".to_string());
        client.request_avaiable_data().await.unwrap();
    }
}
