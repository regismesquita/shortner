use warp::{Reply};
use warp::{Filter};
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use serde_derive::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use serde_json;
use std::str::FromStr;
use warp::hyper::StatusCode;
use tracing::{info, error};

#[derive(Deserialize)]
#[derive(Serialize)]
struct CreateAliasRequest { url: String }

// The Db has the alias as key and url, count as values.
type Db = Arc<Mutex<HashMap<String, (String, usize)>>>;

#[tokio::main]
async fn main() {
    // Setup subscriber
    tracing_subscriber::fmt::init();

    // Create Database
    let db = load_db().await;

    // Persist Database Every X seconds.
    persist_db(&db);

    // Routes
    let stats = stats_route(&db);
    let create_alias = create_alias_route(&db);
    let view_data = view_data_route(db);

    // Define routes and start server
    let routes = stats.or(create_alias).or(view_data);
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

fn persist_db(db: &Db) {
// Spawn task to persist the DB to the disk periodically
    let cloned_db = db.clone();
    info!("Spawning DB savers");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let save_result = save_db(&cloned_db).await;
            if let Err(e) = save_result {
                error!("Failed to save database: {}", e);
            }
        }
    });
}

fn stats_route(db: &Db) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
// Define /stats route
    let db_stats = db.clone();
    let stats = warp::path("stats")
        .and(warp::get())
        .and(warp::any().map(move || db_stats.clone()))
        .and_then(stats);
    stats
}

fn view_data_route(db: Db) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
// Define /<alias> GET route
    let db_view_data = db.clone();
    let view_data = warp::path::param::<String>()
        .and(warp::get())
        .and(warp::any().map(move || db_view_data.clone()))
        .and_then(view_data)
        .recover(handle_rejection);
    view_data
}

fn create_alias_route(db: &Db) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
// Define /<alias> POST route
    let db_create_alias = db.clone();
    let create_alias = warp::path::param::<String>()
        .and(warp::post())
        .and(warp::body::json::<CreateAliasRequest>())
        .and(warp::any().map(move || db_create_alias.clone()))
        .and_then(create_alias)
        .map(|_| warp::reply::with_status("Created", StatusCode::CREATED));
    create_alias
}

async fn load_db() -> Db {
    let file = File::open("db.json");
    match file {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents).expect("Unable to read file");
            let deserialized: HashMap<String, (String, usize)> = serde_json::from_str(&contents).unwrap();
            Arc::new(Mutex::new(deserialized))
        }
        Err(_e) => {
            Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

async fn save_db(db: &Db) -> Result<(), Box<dyn std::error::Error>>  {
    // let serialized = serde_json::to_string(&*read_db).unwrap();
    // let mut file = File::create("db.json").expect("Unable to create file");
    // file.write_all(serialized.as_bytes()).expect("Unable to write data");
    let read_db = db.lock().map_err(|e| format!("Error locking the database: {}", e))?;
    let serialized = serde_json::to_string(&*read_db)?;
    let mut file = File::create("db.json")?;
    file.write_all(serialized.as_bytes())?;
    drop(file);
    Ok(())
}

#[derive(Serialize)]
struct Stats {
    alias: String,
    count: usize,
}

async fn stats(db: Db) -> Result<impl warp::Reply, warp::Rejection> {
    let lock = db.lock().unwrap();
    let stats: Vec<_> = lock
        .iter()
        .map(|(alias, (_, count))| Stats { alias: alias.clone(), count: *count })
        .collect();

    Ok(warp::reply::json(&stats))
}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
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

async fn create_alias(alias: String, req: CreateAliasRequest, db: Db) -> Result<(), warp::Rejection> {
    let mut lock = db.lock().unwrap();
    if lock.contains_key(&alias) {
        return Err(warp::reject::not_found());
    }
    info!("New alias {} , to {}", alias, req.url);
    lock.insert(alias, (req.url, 0));
    Ok(())
}

async fn view_data(alias: String, db: Db) -> Result<impl warp::Reply, warp::Rejection> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_create_alias() {
        let db = Arc::new(Mutex::new(HashMap::new()));
        let test_request = CreateAliasRequest { url: "https://google.com".to_string() };

        let resp = warp::test::request()
            .method("POST")
            .path("/example")
            .json(&test_request)
            .reply(&create_alias_route(&db))
            .await;

        assert_eq!(resp.status(), 201);
    }

    #[tokio::test]
    async fn test_get_data() {
        //create a db
        let db = Arc::new(Mutex::new(HashMap::new()));
        db.lock().unwrap().insert("test".to_string(), ("https://google.com".to_string(), 0));

        let resp = warp::test::request()
            .method("GET")
            .path("/test")
            .reply(&view_data_route(db))
            .await;

        assert_eq!(resp.status(), 307);
    }
}