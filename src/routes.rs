use crate::db;
use db::Db;

use crate::actions;
use actions::*;
use warp::{Reply,Filter};
use warp::hyper::StatusCode;


pub fn stats_route(db: &Db) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
// Define /stats route
    let db_stats = db.clone();
    warp::path("stats")
        .and(warp::get())
        .and(warp::any().map(move || db_stats.clone()))
        .and_then(stats)
}

pub fn view_data_route(db: Db) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
// Define /<alias> GET route
    let db_view_data = db.clone();
    warp::path::param::<String>()
        .and(warp::get())
        .and(warp::any().map(move || db_view_data.clone()))
        .and_then(view_data)
        .recover(handle_rejection)
}

pub fn create_alias_route(db: &Db) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
// Define /<alias> POST route
    let db_create_alias = db.clone();
    warp::path::param::<String>()
        .and(warp::post())
        .and(warp::body::json::<CreateAliasRequest>())
        .and(warp::any().map(move || db_create_alias.clone()))
        .and_then(create_alias)
        .map(|_| warp::reply::with_status("Created", StatusCode::CREATED))
}

