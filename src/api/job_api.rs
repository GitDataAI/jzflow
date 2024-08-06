use crate::core::db::{
    DBConfig, Job, JobDbRepo, JobRepo, MainDbRepo
};
use actix_web::{
    web,
    HttpResponse,
};
use mongodb::bson::oid::ObjectId;
use serde::Serialize;

//TODO change to use route macro after https://github.com/actix/actix-web/issues/2866  resolved
async fn create<'reg, MAINR, JOBR, DBC>(db_repo: web::Data<MAINR>, data: web::Json<Job>) -> HttpResponse
where
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    match db_repo.insert(&data.0).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn get<'reg, MAINR, JOBR, DBC>(db_repo: web::Data<MAINR>, path: web::Path<ObjectId>) -> HttpResponse
where
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    match db_repo.get(&path.into_inner()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn delete<'reg, MAINR, JOBR, DBC>(db_repo: web::Data<MAINR>, path: web::Path<ObjectId>) -> HttpResponse
where
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    match db_repo.delete(&path.into_inner()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub(super) fn job_route_config<'reg, MAINR, JOBR, DBC>(cfg: &mut web::ServiceConfig)
where
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    cfg.service(
        web::resource("/job")
            .route(web::post().to(create::<MAINR, JOBR, DBC>))
            .route(web::delete().to(delete::<MAINR, JOBR, DBC>)),
    )
    .service(web::resource("/job/{id}").route(web::get().to(get::<MAINR, JOBR, DBC>)));
}
