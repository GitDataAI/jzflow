use crate::{
    core::db::{
        Job,
        JobDbRepo,
        JobRepo,
        JobUpdateInfo,
        MainDbRepo,
    },
    driver::Driver,
};
use actix_web::{
    web,
    HttpResponse,
};
use mongodb::bson::oid::ObjectId;

//TODO change to use route macro after https://github.com/actix/actix-web/issues/2866  resolved
async fn create<D, MAINR, JOBR>(db_repo: web::Data<MAINR>, data: web::Json<Job>) -> HttpResponse
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    match db_repo.insert(&data.0).await {
        Ok(inserted_result) => HttpResponse::Ok().json(&inserted_result),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn get<D, MAINR, JOBR>(db_repo: web::Data<MAINR>, path: web::Path<ObjectId>) -> HttpResponse
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    match db_repo.get(&path.into_inner()).await {
        Ok(Some(inserted_result)) => HttpResponse::Ok().json(&inserted_result),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn delete<D, MAINR, JOBR>(
    db_repo: web::Data<MAINR>,
    path: web::Path<ObjectId>,
) -> HttpResponse
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    match db_repo.delete(&path.into_inner()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn update<D, MAINR, JOBR>(
    db_repo: web::Data<MAINR>,
    path: web::Path<ObjectId>,
    query: web::Query<JobUpdateInfo>,
) -> HttpResponse
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    match db_repo
        .update(&path.into_inner(), &query.into_inner())
        .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub(super) fn job_route_config<D, MAINR, JOBR>(cfg: &mut web::ServiceConfig)
where
    D: Driver,
    MAINR: MainDbRepo,
    JOBR: JobDbRepo,
{
    cfg.service(
        web::resource("/job")
            .route(web::post().to(create::<D, MAINR, JOBR>))
            .route(web::delete().to(delete::<D, MAINR, JOBR>)),
    )
    .service(
        web::resource("/job/{id}")
            .route(web::get().to(get::<D, MAINR, JOBR>))
            .route(web::post().to(update::<D, MAINR, JOBR>)),
    );
}
