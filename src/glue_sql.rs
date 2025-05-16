use std::env;
use std::sync::Arc;
use dotenv::dotenv;
use gluesql::{
    gluesql_mongo_storage::MongoStorage,
    prelude::Glue,
};
use tokio::sync::Mutex;

pub type SharedGlue = Arc<Mutex<Glue<MongoStorage>>>;

pub struct GlueSql;

impl GlueSql {
    pub async fn init() -> SharedGlue   {
        dotenv().ok();

        let mongo_uri = env::var("MONGOURI")
            .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());

        let db_name = env::var("MONGO_DB")
            .unwrap_or_else(|_| "sandworm_db".to_string());

        let storage = MongoStorage::new(&mongo_uri, &db_name)
            .await
            .expect("Failed to initialize MongoStorage");

       Arc::new(Mutex::new(Glue::new(storage)))
    }
}
