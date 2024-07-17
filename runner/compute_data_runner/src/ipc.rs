use anyhow::Result;

use actix_web::{web, App, HttpResponse, HttpServer};

use crate::program::BatchProgram;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DataResponse {
    pub(crate) path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SubmitResultReq {
    pub(crate) path: String,
}

async fn process_data_request(program_mutex: web::Data<Arc<Mutex<BatchProgram>>>) -> HttpResponse {
    let (tx, mut rx) = oneshot::channel::<DataResponse>();
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

async fn process_submit_result_request(
    program_mutex: web::Data<Arc<Mutex<BatchProgram>>>,
    data: web::Json<SubmitResultReq>,
) -> HttpResponse {
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

pub(crate) fn start_ipc_server(
    unix_socket_addr: String,
    program: Arc<Mutex<BatchProgram>>,
) -> Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(program.clone())
            .service(web::resource("/api/v1/data").get(process_data_request))
            .service(web::resource("/api/v1/submit").post(process_submit_result_request))
    })
    .bind_uds(unix_socket_addr)
    .unwrap();

    Ok(())
}
