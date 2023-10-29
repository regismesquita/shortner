use crate::db;
use db::Db;

use std::str::FromStr;
use tracing::{info, error};
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Serialize)]
pub struct Stats {
    alias: String,
    count: usize,
}

pub async fn stats(db: Db) -> Result<impl warp::Reply, warp::Rejection> {
    let lock = db.lock().unwrap();
    let stats: Vec<_> = lock
        .iter()
        .map(|(alias, (_, count))| Stats { alias: alias.clone(), count: *count })
        .collect();

    Ok(warp::reply::json(&stats))
}

pub async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    // Log rejections
    error!("An error occured: {:?}", &err);
    if err.is_not_found() {
        Ok(warp::reply::with_status(
            "Not Found",
            warp::http::StatusCode::NOT_FOUND,
        ))
    } else {
        Err(err)
    }
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct CreateAliasRequest { pub url: String }
pub async fn create_alias(alias: String, req: CreateAliasRequest, db: Db) -> Result<(), warp::Rejection> {
    let mut lock = db.lock().unwrap();
    if lock.contains_key(&alias) {
        return Err(warp::reject::not_found());
    }
    info!("New alias {} , to {}", alias, req.url);
    lock.insert(alias, (req.url, 0));
    Ok(())
}

pub async fn view_data(alias: String, db: Db) -> Result<impl warp::Reply, warp::Rejection> {
    let mut lock = db.lock().unwrap();
    if let Some((url, count)) = lock.get_mut(&alias) {
        *count += 1;
        if *count % 1000 == 0 {
            info!("{} ({}) reached {} views!", alias, url, count);
        }
        return Ok(Box::new(warp::redirect::temporary(warp::http::Uri::from_str(url).unwrap())));
    }
    Err(warp::reject::not_found())
}
