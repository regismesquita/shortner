use warp::Filter;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::path::PathBuf;
use serde_derive::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use serde_json::{to_string, from_str};

use std::str::FromStr;
use warp::hyper::StatusCode;


#[derive(Deserialize)]
struct CreateAliasRequest { url: String }

type Db = Arc<Mutex<HashMap<String, (String, usize)>>>;

#[tokio::main]
async fn main() {
    let db = load_db().await;
    let cloned_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            save_db(&cloned_db).await;
        }
    });

    let db_stats = db.clone();
    let stats = warp::path("stats")
        .and(warp::get())
        .and(warp::any().map(move || db_stats.clone()))
        .and_then(stats);

    let db_create_alias = db.clone();
    let create_alias = warp::path::param::<String>()
        .and(warp::post())
        .and(warp::body::json::<CreateAliasRequest>())
        .and(warp::any().map(move || db_create_alias.clone()))
        .and_then(create_alias)
        .map(|_| warp::reply::with_status("Created", StatusCode::CREATED));

    let db_view_data = db.clone();
    let view_data = warp::path::param::<String>()
        .and(warp::get())
        .and(warp::any().map(move || db_view_data.clone()))
        .and_then(view_data)
        .recover(handle_rejection);

    let routes = stats.or(create_alias).or(view_data);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[derive(Serialize)]
struct Stats {
    alias: String,
    count: usize,
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

async fn save_db(db: &Db) {
    let read_db = db.lock().unwrap();
    let serialized = serde_json::to_string(&*read_db).unwrap();
    let mut file = File::create("db.json").expect("Unable to create file");
    file.write_all(serialized.as_bytes()).expect("Unable to write data");
    drop(file);
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
    if err.is_not_found() {
        Ok(warp::reply::with_status(
            "Not Found",
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
    else {
        Err(err)
    }
}

async fn create_alias(alias: String, req: CreateAliasRequest, db: Db) -> Result<(), warp::Rejection> {
    let mut lock = db.lock().unwrap();
    if lock.contains_key(&alias) {
        return Err(warp::reject::not_found());
    }
    lock.insert(alias, (req.url, 0));
    Ok(())
}

async fn view_data(alias: String, db: Db) -> Result<impl warp::Reply, warp::Rejection> {
    let mut lock = db.lock().unwrap();
    if let Some((url, count)) = lock.get_mut(&alias) {
        *count += 1;
        return Ok(Box::new(warp::redirect::temporary(warp::http::Uri::from_str(url).unwrap())));
    }
    Err(warp::reject::not_found())
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::test::request;

    #[tokio::test]
    async fn test_create_alias() {
        let db = Arc::new(Mutex::new(HashMap::new()));
        let test_request = CreateAliasRequest { url: "https://google.com".to_string() };

        let resp = request::post()
            .path("/example")
            .json(&test_request)
            .reply(&create_alias_index("/example".to_string(), test_request, db))
            .await;

        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_get_data() {
        let db = Arc::new(Mutex::new(HashMap::new()));
        db.lock().unwrap().insert("test".to_string(), ("https://google.com".to_string(), 0));

        let resp = request::get()
            .path("/test")
            .reply(&data_index(db))
            .await;

        assert_eq!(resp.status(), 302);
    }
}