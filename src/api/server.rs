
use crate::{
    core::db::{
        JobDbRepo,
        MainDbRepo,
    },
    driver::Driver,
    job::job_mgr::JobManager,
    utils::IntoAnyhowResult,
};
use actix_web::{
    dev::Server,
    error,
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
use reqwest::Url;

use super::job_api::job_route_config;

fn v1_route<D, MAINR, JOBR>(cfg: &mut web::ServiceConfig)
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    job_route_config::<D, MAINR, JOBR>(cfg);
}

fn config<D, MAINR, JOBR>(cfg: &mut web::ServiceConfig)
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    cfg.service(web::scope("/api/v1").configure(v1_route::<D, MAINR, JOBR>));
}

pub fn start_rpc_server<D, MAINR, JOBR>(
    addr: &str,
    main_db_repo: MAINR,
    job_manager: JobManager<D, MAINR, JOBR>,
) -> Result<Server>
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    let main_db_repo = main_db_repo;
    let uri = Url::parse(addr)?;
    let host_port = format!(
        "{}:{}",
        uri.host().anyhow("host not found")?,
        uri.port().unwrap_or(80)
    );
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
            .app_data(Data::new(main_db_repo.clone()))
            .app_data(Data::new(job_manager.clone()))
            .app_data(web::JsonConfig::default().error_handler(json_error_handler))
            .configure(config::<D, MAINR, JOBR>)
    })
    .disable_signals()
    .bind(host_port)?
    .run();

    Ok(server)
}
