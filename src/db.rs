// A Database that persists the information
// a loop function saves the current app status every 10s
// a load_db loads the previous app status during boot time.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tracing::{error, info};

// The Db has the alias as key and url, count as values.
pub type Db = Arc<Mutex<HashMap<String, (String, usize)>>>;

// Loads the DB from disk during startup time.
pub async fn load_db() -> Db {
    let file = File::open("db.json");
    match file {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Unable to read file");
            let deserialized: HashMap<String, (String, usize)> =
                serde_json::from_str(&contents).unwrap();
            Arc::new(Mutex::new(deserialized))
        }
        Err(_e) => Arc::new(Mutex::new(HashMap::new())),
    }
}

// Loop function to persist the database.
pub fn persist_db(db: &Db) {
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

// Saves the database, this is called by a loop function
pub async fn save_db(db: &Db) -> Result<(), Box<dyn std::error::Error>> {
    let read_db = db
        .lock()
        .map_err(|e| format!("Error locking the database: {}", e))?;
    let serialized = serde_json::to_string(&*read_db)?;
    let mut file = File::create("db.json")?;
    file.write_all(serialized.as_bytes())?;
    drop(file);
    Ok(())
}
