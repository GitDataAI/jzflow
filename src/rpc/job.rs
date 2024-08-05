use crate::core::db::{
    Job,
    JobRepo,
    MainDbRepo,
};
use actix_web::{
    route,
    web,
    HttpResponse,
};
use mongodb::bson::oid::ObjectId;

//TODO change to use route macro after https://github.com/actix/actix-web/issues/2866  resolved
async fn create<R>(db_repo: web::Data<R>, data: web::Json<Job>) -> HttpResponse
where
    R: JobRepo,
{
    match db_repo.insert(&data.0).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn get<R>(db_repo: web::Data<R>, path: web::Path<(ObjectId)>) -> HttpResponse
where
    R: JobRepo,
{
    match db_repo.get(&path.into_inner()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn delete<R>(db_repo: web::Data<R>, path: web::Path<(ObjectId)>) -> HttpResponse
where
    R: JobRepo,
{
    match db_repo.delete(&path.into_inner()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub(super) fn job_route_config<R>(cfg: &mut web::ServiceConfig)
where
    R: MainDbRepo,
{
    cfg.service(
        web::resource("/job")
            .route(web::post().to(create::<R>))
            .route(web::delete().to(delete::<R>)),
    )
    .service(web::resource("/job/{id}").route(web::get().to(get::<R>)));
}
