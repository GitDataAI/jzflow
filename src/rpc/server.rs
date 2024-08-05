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

use crate::core::db::MainDbRepo;

use super::job::job_route_config;

fn v1_route<R>(cfg: &mut web::ServiceConfig)
where
    R: MainDbRepo,
{
    cfg.service(web::scope("/job").configure(job_route_config::<R>));
}

fn config<R>(cfg: &mut web::ServiceConfig)
where
    R: MainDbRepo,
{
    cfg.service(web::scope("/api/v1").configure(v1_route::<R>));
}

pub fn start_rpc_server<R>(addr: &str, db_repo: R) -> Result<Server>
where
    R: MainDbRepo,
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
            .configure(config::<R>)
            .app_data(web::JsonConfig::default().error_handler(json_error_handler))
    })
    .disable_signals()
    .bind(addr)?
    .run();

    Ok(server)
}
