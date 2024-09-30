use std::str::FromStr;

use crate::{
    core::db::{
        GetJobParams,
        Job,
        JobRepo,
        JobUpdateInfo,
        ListJobParams,
        Repo,
    },
    driver::Driver,
    job::job_mgr::JobManager,
};
use actix_web::{
    web,
    HttpResponse,
};
use mongodb::bson::oid::ObjectId;

//TODO change to use route macro after https://github.com/actix/actix-web/issues/2866  resolved
async fn create<MAINR>(db_repo: web::Data<MAINR>, data: web::Json<Job>) -> HttpResponse
where
    MAINR: JobRepo,
{
    match db_repo.insert(&data.0).await {
        Ok(inserted_result) => HttpResponse::Ok().json(&inserted_result),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn get_by_id<MAINR>(db_repo: web::Data<MAINR>, path: web::Path<ObjectId>) -> HttpResponse
where
    MAINR: JobRepo,
{
    match db_repo
        .get(&GetJobParams::new().set_id(path.into_inner()))
        .await
    {
        Ok(Some(inserted_result)) => HttpResponse::Ok().json(&inserted_result),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn get<MAINR>(db_repo: web::Data<MAINR>, query: web::Query<GetJobParams>) -> HttpResponse
where
    MAINR: JobRepo,
{
    match db_repo.get(&query.into_inner()).await {
        Ok(Some(inserted_result)) => HttpResponse::Ok().json(&inserted_result),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn list<MAINR>(db_repo: web::Data<MAINR>) -> HttpResponse
where
    MAINR: JobRepo,
{
    let list_job_params = &ListJobParams { state: None };
    match db_repo.list_jobs(list_job_params).await {
        Ok(jobs) => HttpResponse::Ok().json(&jobs),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn clean_job<D, MAINR, JOBR>(
    job_manager: web::Data<JobManager<D, MAINR, JOBR>>,
    path: web::Path<String>,
) -> HttpResponse
where
    D: Driver,
    MAINR: JobRepo,
    JOBR: Repo,
{
    let id = ObjectId::from_str(&path.into_inner()).unwrap();
    match job_manager.clean_job(&GetJobParams::new().set_id(id)).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn update<MAINR>(
    db_repo: web::Data<MAINR>,
    path: web::Path<ObjectId>,
    query: web::Query<JobUpdateInfo>,
) -> HttpResponse
where
    MAINR: JobRepo,
{
    match db_repo
        .update(&path.into_inner(), &query.into_inner())
        .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn job_details<D, MAINR, JOBR>(
    job_manager: web::Data<JobManager<D, MAINR, JOBR>>,
    path: web::Path<String>,
) -> HttpResponse
where
    D: Driver,
    MAINR: JobRepo,
    JOBR: Repo,
{
    let id = ObjectId::from_str(&path.into_inner()).unwrap();
    match job_manager
        .get_job_details(&GetJobParams::new().set_id(id))
        .await
    {
        Ok(detail) => HttpResponse::Ok().json(detail),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn run_job<D, MAINR, JOBR>(
    job_manager: web::Data<JobManager<D, MAINR, JOBR>>,
    path: web::Path<String>,
) -> HttpResponse
where
    D: Driver,
    MAINR: JobRepo,
    JOBR: Repo,
{
    let id = ObjectId::from_str(&path.into_inner()).unwrap();
    match job_manager.start_job(&GetJobParams::new().set_id(id)).await {
        Ok(detail) => HttpResponse::Ok().json(detail),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub(super) fn job_route_config<D, MAINR, JOBR>(cfg: &mut web::ServiceConfig)
where
    D: Driver,
    MAINR: JobRepo,
    JOBR: Repo,
{
    cfg.service(
        web::resource("/job")
            .route(web::post().to(create::<MAINR>))
            .route(web::get().to(get::<MAINR>)),
    )
    .service(
        web::resource("/job/{id}")
            .route(web::get().to(get_by_id::<MAINR>))
            .route(web::post().to(update::<MAINR>))
            .route(web::delete().to(clean_job::<D, MAINR, JOBR>)),
    )
    .service(web::resource("/jobs").route(web::get().to(list::<MAINR>)))
    .service(web::resource("/job/detail/{id}").route(web::get().to(job_details::<D, MAINR, JOBR>)))
    .service(web::resource("/job/run/{id}").route(web::post().to(run_job::<D, MAINR, JOBR>)));
}
