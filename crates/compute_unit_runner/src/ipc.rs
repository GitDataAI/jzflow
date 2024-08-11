use crate::media_data_tracker::MediaDataTracker;
use actix_web::{
    dev::Server,
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
use anyhow::Result;
use core::str;
use http_body_util::Collected;
use jz_flow::core::db::{
    JobDbRepo,
    TrackerState,
};
use serde::{
    ser::SerializeStruct,
    Deserialize,
    Serialize,
    Serializer,
};
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{
        oneshot,
        RwLock,
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
use serde::de::{
    self,
    Deserializer,
    MapAccess,
    Visitor,
};
use serde_repr::*;

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

use std::fmt;
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum ErrorNumber {
    NotReady = 1,
    InComingFinish = 2,
    AlreadyFinish = 3,
    DataNotFound = 4,
}

#[derive(Debug)]
pub enum IPCError {
    NodeError { code: ErrorNumber, msg: String },
    UnKnown(String),
}

impl<T> From<T> for IPCError
where
    T: std::error::Error,
{
    fn from(error: T) -> Self {
        IPCError::UnKnown(error.to_string())
    }
}

impl fmt::Display for IPCError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IPCError::NodeError { code, msg } => write!(f, "node error code({:?}): {msg}", code),
            IPCError::UnKnown(err) => write!(f, "unknown error: {}", err),
        }
    }
}

impl Serialize for IPCError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            IPCError::NodeError { code, msg } => {
                let mut my_error_se = serializer.serialize_struct("IPCError", 2)?;
                my_error_se.serialize_field("code", &code)?;
                my_error_se.serialize_field("msg", msg.as_str())?;
                my_error_se.end()
            }
            IPCError::UnKnown(msg) => serializer.serialize_str(msg.as_str()),
        }
    }
}

impl<'de> Deserialize<'de> for IPCError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Code,
            Msg,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`code` or `msg`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "code" => Ok(Field::Code),
                            "msg" => Ok(Field::Msg),
                            _ => Err(de::Error::unknown_field(value, &["code", "msg"])),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct DurationVisitor;

        impl<'de> Visitor<'de> for DurationVisitor {
            type Value = IPCError;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct IPCError")
            }

            fn visit_str<E>(self, value: &str) -> Result<IPCError, E>
            where
                E: de::Error,
            {
                Ok(IPCError::UnKnown(value.to_string()))
            }

            fn visit_map<V>(self, mut map: V) -> Result<IPCError, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut code = None;
                let mut msg = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Code => {
                            if code.is_some() {
                                return Err(de::Error::duplicate_field("code"));
                            }
                            code = Some(map.next_value()?);
                        }
                        Field::Msg => {
                            if msg.is_some() {
                                return Err(de::Error::duplicate_field("msg"));
                            }
                            msg = Some(map.next_value()?);
                        }
                    }
                }
                let code = code.ok_or_else(|| de::Error::missing_field("code"))?;
                let msg = msg.ok_or_else(|| de::Error::missing_field("msg"))?;
                Ok(IPCError::NodeError { code, msg })
            }
        }

        deserializer.deserialize_any(DurationVisitor)
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

#[derive(Serialize, Deserialize)]
pub struct RequetDataReq {
    pub metadata_id: Option<String>,
}

impl RequetDataReq {
    pub fn new(metadata_id: &str) -> Self {
        RequetDataReq {
            metadata_id: Some(metadata_id.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub state: TrackerState,
}

async fn status<R>(program_mutex: web::Data<Arc<RwLock<MediaDataTracker<R>>>>) -> HttpResponse
where
    R: JobDbRepo + Clone,
{
    info!("receive status request");
    let status = {
        let program = program_mutex.read().await;
        let local_state = program.local_state.read().await;
        Status {
            state: local_state.clone(),
        }
    };
    HttpResponse::Ok().json(status)
}

async fn process_data_request<R>(
    program_mutex: web::Data<Arc<RwLock<MediaDataTracker<R>>>>,
    req: web::Query<RequetDataReq>,
) -> HttpResponse
where
    R: JobDbRepo + Clone,
{
    info!("receive avaiable data reqeust");
    let sender = loop {
        let program = program_mutex.read().await;
        let local_state = program.local_state.read().await;
        if *local_state == TrackerState::Finish {
            return HttpResponse::BadRequest().json(IPCError::NodeError {
                code: ErrorNumber::AlreadyFinish,
                msg: "node is already finish".to_string(),
            });
        }

        if *local_state == TrackerState::InComingFinish {
            return HttpResponse::BadRequest().json(IPCError::NodeError {
                code: ErrorNumber::InComingFinish,
                msg: "incoming is already finish".to_string(),
            });
        }

        if *local_state != TrackerState::Init {
            break program.ipc_process_data_req_tx.as_ref().cloned();
        }
        drop(local_state);
        drop(program);
        sleep(Duration::from_secs(5)).await;
    };

    //read request
    let (tx, rx) = oneshot::channel::<Result<Option<AvaiableDataResponse>>>();
    match sender {
        Some(sender) => {
            if let Err(err) = sender.send((req.into_inner(), tx)).await {
                return HttpResponse::InternalServerError()
                    .json(format!("send to avaiable data channel {err}"));
            }
        }
        None => {
            return HttpResponse::InternalServerError()
                .json("channel is not ready maybe not start route data");
        }
    }

    match rx.await {
        Ok(Ok(Some(resp))) => HttpResponse::Ok().json(resp),
        Ok(Ok(None)) => HttpResponse::NotFound().json(IPCError::NodeError {
            code: ErrorNumber::DataNotFound,
            msg: "no avaiable data".to_string(),
        }),
        Ok(Err(err)) => HttpResponse::InternalServerError().json(err.to_string()),
        Err(err) => HttpResponse::ServiceUnavailable().json(err.to_string()),
    }
}

async fn process_completed_request<R>(
    program_mutex: web::Data<Arc<RwLock<MediaDataTracker<R>>>>,
    data: web::Json<CompleteDataReq>,
) -> HttpResponse
where
    R: JobDbRepo + Clone,
{
    info!("receive data completed request");
    let sender = loop {
        let program = program_mutex.read().await;
        let local_state = program.local_state.read().await;

        if *local_state == TrackerState::Finish {
            return HttpResponse::BadRequest().json(IPCError::NodeError {
                code: ErrorNumber::AlreadyFinish,
                msg: "node is already finish".to_string(),
            });
        }

        if *local_state != TrackerState::Init {
            break program.ipc_process_completed_data_tx.as_ref().cloned();
        }
        drop(local_state);
        drop(program);
        sleep(Duration::from_secs(5)).await;
    };

    //read request
    let (tx, rx) = oneshot::channel::<Result<()>>();
    match sender {
        Some(sender) => {
            if let Err(err) = sender.send((data.0, tx)).await {
                return HttpResponse::InternalServerError()
                    .json(format!("send to avaiable data channel {err}"));
            }
        }
        None => {
            return HttpResponse::InternalServerError()
                .json("channel is not ready maybe not start route data");
        }
    }

    match rx.await {
        Ok(Ok(resp)) => HttpResponse::Ok().json(resp),
        Ok(Err(err)) => HttpResponse::InternalServerError().json(err.to_string()),
        Err(err) => HttpResponse::ServiceUnavailable().json(err.to_string()),
    }
}

async fn process_submit_output_request<R>(
    program_mutex: web::Data<Arc<RwLock<MediaDataTracker<R>>>>,
    data: web::Json<SubmitOuputDataReq>,
    //body: web::Bytes,
) -> HttpResponse
where
    R: JobDbRepo + Clone,
{
    // let data: SubmitOuputDataReq = serde_json::from_slice(&body).unwrap();

    info!("receive submit output request {}", &data.id);
    let sender = loop {
        let program = program_mutex.read().await;
        let local_state = program.local_state.read().await;

        if *local_state == TrackerState::Finish {
            return HttpResponse::BadRequest().json(IPCError::NodeError {
                code: ErrorNumber::AlreadyFinish,
                msg: "node is already finish".to_string(),
            });
        }

        if *local_state != TrackerState::Init {
            break program.ipc_process_submit_output_tx.as_ref().cloned();
        }
        drop(local_state);
        drop(program);
        sleep(Duration::from_secs(5)).await;
    };

    //read request
    let (tx, rx) = oneshot::channel::<Result<()>>();
    match sender {
        Some(sender) => {
            if let Err(err) = sender.send((data.0, tx)).await {
                return HttpResponse::InternalServerError()
                    .json(format!("send to avaiable data channel {err}"));
            }
        }
        None => {
            return HttpResponse::InternalServerError()
                .json("channel is not ready maybe not start route data");
        }
    }

    match rx.await {
        Ok(Ok(resp)) => HttpResponse::Ok().json(resp),
        Ok(Err(err)) => HttpResponse::InternalServerError().json(err.to_string()),
        Err(err) => HttpResponse::ServiceUnavailable().json(err.to_string()),
    }
}

async fn process_finish_state_request<R>(
    program_mutex: web::Data<Arc<RwLock<MediaDataTracker<R>>>>,
) -> HttpResponse
where
    R: JobDbRepo + Clone,
{
    info!("receive finish state request");
    let sender = loop {
        let program = program_mutex.read().await;
        let local_state = program.local_state.read().await;
        if *local_state == TrackerState::Finish {
            return HttpResponse::Conflict().json(IPCError::NodeError {
                code: ErrorNumber::AlreadyFinish,
                msg: "node is already finish".to_string(),
            });
        }

        if *local_state != TrackerState::Init {
            break program.ipc_process_finish_state_tx.as_ref().cloned();
        }
        drop(local_state);
        drop(program);
        sleep(Duration::from_secs(5)).await;
    };

    //read request
    let (tx, rx) = oneshot::channel::<Result<()>>();
    match sender {
        Some(sender) => {
            if let Err(err) = sender.send(((), tx)).await {
                return HttpResponse::InternalServerError()
                    .json(format!("send to finish state channel {err}"));
            }
        }
        None => {
            return HttpResponse::InternalServerError()
                .json("channel is not ready maybe not start route data");
        }
    }

    match rx.await {
        Ok(Ok(resp)) => HttpResponse::Ok().json(resp),
        Ok(Err(err)) => HttpResponse::InternalServerError().json(err.to_string()),
        Err(err) => HttpResponse::ServiceUnavailable().json(err.to_string()),
    }
}

pub fn start_ipc_server<R>(
    unix_socket_addr: &str,
    program: Arc<RwLock<MediaDataTracker<R>>>,
) -> Result<Server>
where
    R: JobDbRepo + Clone + Send + Sync + 'static,
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
                    .service(
                        web::resource("status")
                            .get(status::<R>)
                            .post(process_finish_state_request::<R>),
                    )
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
    fn finish(&self) -> impl std::future::Future<Output = Result<(), IPCError>> + Send;
    fn status(&self) -> impl std::future::Future<Output = Result<Status, IPCError>> + Send;
    fn submit_output(
        &self,
        req: SubmitOuputDataReq,
    ) -> impl std::future::Future<Output = Result<(), IPCError>> + Send;
    fn complete_result(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<(), IPCError>> + Send;
    fn request_avaiable_data(
        &self,
        metadata_id: Option<String>,
    ) -> impl std::future::Future<Output = Result<Option<AvaiableDataResponse>, IPCError>> + Send;
}

pub struct IPCClientImpl {
    unix_socket_addr: String,
    //TODO change reqwest https://github.com/seanmonstar/reqwest/pull/1623
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
    async fn finish(&self) -> Result<(), IPCError> {
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/status");

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::default())
            .map_err(IPCError::from)?;

        let resp = self.client.request(req).await.map_err(IPCError::from)?;
        if resp.status().is_success() {
            return Ok(());
        }

        let resp_bytes = resp
            .collect()
            .await
            .map(Collected::to_bytes)
            .map_err(IPCError::from)?;
        Err(serde_json::from_slice(&resp_bytes).map_err(IPCError::from)?)
    }

    async fn status(&self) -> Result<Status, IPCError> {
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/status");

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::default())
            .map_err(IPCError::from)?;

        let resp = self.client.request(req).await.map_err(IPCError::from)?;
        if !resp.status().is_success() {
            let resp_bytes = resp
                .collect()
                .await
                .map(Collected::to_bytes)
                .map_err(IPCError::from)?;
            return Err(serde_json::from_slice(&resp_bytes).map_err(IPCError::from)?);
        }

        let contents = resp.collect().await.map(Collected::to_bytes)?;
        let status: Status = serde_json::from_slice(contents.as_ref())?;
        Ok(status)
    }

    async fn submit_output(&self, req: SubmitOuputDataReq) -> Result<(), IPCError> {
        let json = serde_json::to_string(&req)?;
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/submit");

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::from(json))
            .map_err(IPCError::from)?;

        let resp = self.client.request(req).await.map_err(IPCError::from)?;
        if resp.status().is_success() {
            return Ok(());
        }

        let resp_bytes = resp
            .collect()
            .await
            .map(Collected::to_bytes)
            .map_err(IPCError::from)?;
        Err(serde_json::from_slice(&resp_bytes).map_err(IPCError::from)?)
    }

    async fn request_avaiable_data(
        &self,
        metadata_id: Option<String>,
    ) -> Result<Option<AvaiableDataResponse>, IPCError> {
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/data");
        let json = serde_json::to_string(&RequetDataReq { metadata_id })?;
        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::from(json))
            .map_err(IPCError::from)?;

        let resp = self.client.request(req).await.map_err(IPCError::from)?;
        if resp.status().as_u16() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let resp_bytes = resp
                .collect()
                .await
                .map(Collected::to_bytes)
                .map_err(IPCError::from)?;
            return Err(serde_json::from_slice(&resp_bytes).map_err(IPCError::from)?);
        }

        let contents = resp.collect().await.map(Collected::to_bytes)?;
        let avaiabel_data: AvaiableDataResponse = serde_json::from_slice(contents.as_ref())?;
        Ok(Some(avaiabel_data))
    }

    async fn complete_result(&self, id: &str) -> Result<(), IPCError> {
        let req = CompleteDataReq::new(id);
        let json = serde_json::to_string(&req)?;
        let url: Uri = Uri::new(self.unix_socket_addr.clone(), "/api/v1/data");

        let req: Request<Full<Bytes>> = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Full::from(json))
            .map_err(IPCError::from)?;

        let resp = self.client.request(req).await.map_err(IPCError::from)?;
        if resp.status().is_success() {
            return Ok(());
        }

        let resp_bytes = resp
            .collect()
            .await
            .map(Collected::to_bytes)
            .map_err(IPCError::from)?;
        Err(serde_json::from_slice(&resp_bytes).map_err(IPCError::from)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_error() {
        {
            let my_err = IPCError::NodeError {
                code: ErrorNumber::AlreadyFinish,
                msg: "aaaa".to_string(),
            };
            let result = serde_json::to_string(&my_err).unwrap();
            assert_eq!(result, r#"{"code":2,"msg":"aaaa"}"#);

            let de_my_err = serde_json::from_str(&result).unwrap();
            match de_my_err {
                IPCError::NodeError { code, msg } => {
                    assert_eq!(code, ErrorNumber::AlreadyFinish);
                    assert_eq!(msg, "aaaa");
                }
                _ => panic!("unexpected error type"),
            }
        }

        {
            let my_err = IPCError::UnKnown("bbbbbbbbbbbbbb".to_string());
            let result = serde_json::to_string(&my_err).unwrap();
            assert_eq!(result, r#""bbbbbbbbbbbbbb""#);

            let de_my_err = serde_json::from_str(&result).unwrap();
            match de_my_err {
                IPCError::UnKnown(msg) => {
                    assert_eq!(msg, "bbbbbbbbbbbbbb");
                }
                _ => panic!("unexpected error type"),
            }
        }
    }
}
