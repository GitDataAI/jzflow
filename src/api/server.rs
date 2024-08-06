use std::sync::Arc;

use actix_web::{
    dev::Server,
    error,
    middleware,
    web::{
        self,
    },
    App,
    HttpRequest,
    HttpResponse,
    HttpServer,
};
use anyhow::Result;
use serde::Serialize;

use crate::{
    core::db::{
        DBConfig, DataRepo, JobDbRepo, MainDbRepo
    },
    job::job_mgr::JobManager,
};

use super::job_api::job_route_config;

fn v1_route<'reg, MAINR, JOBR, DBC>(cfg: &mut web::ServiceConfig)
where
MAINR: MainDbRepo,
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    cfg.service(web::scope("/job").configure(job_route_config::<MAINR, JOBR, DBC>));
}

fn config<'reg, MAINR, JOBR, DBC>(cfg: &mut web::ServiceConfig)
where
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    cfg.service(web::scope("/api/v1").configure(v1_route::<MAINR, JOBR, DBC>));
}

pub fn start_rpc_server<'reg, MAINR, JOBR, DBC>(
    addr: &str,
    db_repo: MAINR,
    job_manager: JobManager<'reg, JOBR, DBC>,
) -> Result<Server>
where
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    let db_repo = db_repo;
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
            .app_data(db_repo.clone())
            .configure(config::<MAINR, JOBR, DBC>)
            .app_data(web::JsonConfig::default().error_handler(json_error_handler))
    })
    .disable_signals()
    .bind(addr)?
    .run();

    Ok(server)
}
