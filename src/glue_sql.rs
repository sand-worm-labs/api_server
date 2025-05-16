use std::env;
use std::sync::Arc;
use dotenv::dotenv;
use gluesql:: prelude::Glue;
use gluesql_redis_storage::RedisStorage;
use tokio::sync::Mutex;

pub type SharedGlue = Glue<RedisStorage>;

pub struct GlueSql;

impl GlueSql {
    pub async fn init() -> SharedGlue   {
        dotenv().ok();

        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let storage = RedisStorage::new("redis_storage_no_primarykey", &redis_url, 1000);

        Glue::new(storage)
    }
}
