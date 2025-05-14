use std::any::type_name;

use eql_core::{
    common::query_result::QueryResult as EqlQueryResult, interpreter::Interpreter as EQlInterpreter,
};
use rocket::http::Status;
use rocket::response::{content::RawJson, content::RawText, status};
use serde::Serialize;
use serde_json::json;
use sui_ql_core::{
    common::query_result::QueryResult as SuiQueryResult,
    interpreter::Interpreter as SuiQlInterpreter,
};

use {
    // gluesql::{gluesql_mongo_storage::MongoStorage, gluesql_redis_storage::RedisStorage, prelude::Glue},
    std::fs,
};
mod utils;

#[macro_use]
extern crate rocket;

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum QueryResult {
    Sui(Vec<SuiQueryResult>),
    Eql(Vec<EqlQueryResult>),
}

#[get("/")]
fn index() -> &'static str {
    "Server is up and running!"
}

#[get("/health")]
fn health_check() -> &'static str {
    "Server is up and running!"
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

    if utils::is_query_only(query.to_owned()) {
        return status::Custom(
            Status::BadRequest,
            RawJson(r#"{"error": "Only SELECT queries are allowed. CREATE, DROP, INSERT, UPDATE, DELETE, and other write ops are blocked."} "#.to_string()),
        );
    }
    if type_param == "rpc" {
        let (label, result): (&str, Result<QueryResult, _>) = if utils::is_sui_rpc_query(query) {
            let res = SuiQlInterpreter::run_program(query)
                .await
                .map(QueryResult::Sui);
            ("SUI_QL", res)
        } else {
            let res = EQlInterpreter::run_program(query)
                .await
                .map(QueryResult::Eql);
            ("EQL", res)
        };

        match result {
            Ok(data) => match serde_json::to_string(&data) {
                Ok(json) => {
                    return status::Custom(Status::Ok, RawJson(json));
                }
                Err(err) => {
                    let error_json = json!({
                        "error": format!("{} serialization failed: {}", label, err.to_string())
                    })
                    .to_string();
                    return status::Custom(Status::InternalServerError, RawJson(error_json));
                }
            },
            Err(err) => {
                let error_json = json!({
                    "error": format!("{} query failed: {}", label, err.to_string())
                })
                .to_string();
                return status::Custom(Status::InternalServerError, RawJson(error_json));
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
    rocket::build().mount("/", routes![index, run_query, health_check])
}
