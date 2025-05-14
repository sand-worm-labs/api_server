use std::any::type_name;

use eql_core::interpreter::Interpreter as EQlInterpreter;
use rocket::http::Status;
use rocket::response::{content::RawJson, content::RawText, status};
use serde_json;
use sui_ql_core::interpreter::Interpreter as SuiQlInterpreter;

use {
    // gluesql::{gluesql_mongo_storage::MongoStorage, gluesql_redis_storage::RedisStorage, prelude::Glue},
    std::fs,
};
mod utils;

#[macro_use]
extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/run?<type_param>&<query>")]
async fn run_query(query: &str, type_param: &str) -> status::Custom<RawJson<String>> {
    if !matches!(type_param, "rpc" | "indexed") {
        return status::Custom(
            Status::BadRequest,
            RawJson(
                r#"{"error": "Invalid type. Supported values are: 'rpc' or 'indexed'."} "#
                    .to_string(),
            ),
        );
    }

    if !utils::is_query_only(query.to_owned()) {
        return status::Custom(
            Status::BadRequest,
            RawJson(r#"{"error": "Only SELECT queries are allowed. CREATE, DROP, INSERT, UPDATE, DELETE, and other write ops are blocked."} "#.to_string()),
        );
    }

    if type_param == "rpc" && utils::is_sui_rpc_query(query) {
        match SuiQlInterpreter::run_program(query).await {
            Ok(data) => {
                let json = serde_json::to_string(&data)
                    .unwrap_or_else(|_| r#"{"error": "Serialization failed."}"#.to_string());
                return status::Custom(Status::Ok, RawJson(json));
            }
            Err(err) => {
                eprintln!("Sui query failed: {:?}", err);
                return status::Custom(
                    Status::InternalServerError,
                    RawJson(r#"{"error": "Query execution failed."}"#.to_string()),
                );
            }
        }
    } else if type_param == "rpc" {
        match EQlInterpreter::run_program(query).await {
            Ok(data) => {
                let json = serde_json::to_string(&data)
                    .unwrap_or_else(|_| r#"{"error": "Serialization failed."}"#.to_string());
                return status::Custom(Status::Ok, RawJson(json));
            }
            Err(err) => {
                eprintln!("Indexed query failed: {:?}", err);
                return status::Custom(
                    Status::InternalServerError,
                    RawJson(r#"{"error": "Query execution failed."}"#.to_string()),
                );
            }
        }
    } else {
        return status::Custom(
            Status::InternalServerError,
            RawJson(r#"{"error": "Query execution failed."}"#.to_string()),
        );
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, run_query])
}
