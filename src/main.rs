mod db;
mod actions;
mod routes;

pub use actions::*;
pub use routes::*;
pub use db::{load_db, persist_db, Db};
use warp::{Filter};


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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, Arc};
    use std::collections::HashMap;
    #[tokio::test]
    async fn test_create_alias() {
        let db: Db = Arc::new(Mutex::new(HashMap::new()));
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
        let db: Db = Arc::new(Mutex::new(HashMap::new()));

        db.lock().unwrap().insert(
            "test".to_string(),
            ("https://google.com".to_string(), 4)
        );

        let resp = warp::test::request()
            .method("GET")
            .path("/test")
            .reply(&view_data_route(db))
            .await;

        assert_eq!(resp.status(), 307);
    }


    #[tokio::test]
    async fn test_get_stats() {
        //create a db
        let db: Db = Arc::new(Mutex::new(HashMap::new()));
        db.lock().unwrap().insert(
            "test".to_string(),
            ("https://google.com".to_string(), 4)
        );

        let resp = warp::test::request()
            .method("GET")
            .path("/stats")
            .reply(&stats_route(&db))
            .await;

        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), r#"[{"alias":"test","count":4}]"#);
    }
}