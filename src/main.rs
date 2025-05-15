use std::error::Error;
use eql_core::{
    common::query_result::QueryResult as EqlQueryResult, interpreter::Interpreter as EQlInterpreter,
};
use rocket::http::{Method, Status};
use rocket::response::{content::RawJson, status};
use serde::Serialize;
use serde_json::json;

use sui_ql_core::{
    common::query_result::QueryResult as SuiQueryResult,
    interpreter::Interpreter as SuiQlInterpreter,
};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::Request;
use rocket::http::Header;
use rocket::response::Response;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Attaching CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

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
    "Sandworm API Server is up and running!"
}

#[get("/health")]
fn health() -> RawJson<String> {
    RawJson("{\"status\":\"healthy\"}".to_string())
}

#[get("/run?<type_param>&<query>")]
async fn run_query(query: &str, type_param: &str) -> status::Custom<RawJson<String>> {
    if !matches!(type_param, "rpc" | "indexed") {
        return status::Custom(
            Status::BadRequest,
            RawJson(
                r#"{"error": "Invalid type. Supported values are: 'rpc' or 'indexed'."} "#.to_string(),
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
                Ok(json) => status::Custom(Status::Ok, RawJson(json)),
                Err(err) => {
                    let error_json = json!({
                        "error": format!("{} serialization failed: {}", label, err)
                    })
                    .to_string();
                    status::Custom(Status::InternalServerError, RawJson(error_json))
                }
            },
            Err(err) => {
                let error_json = json!({
                    "error": format!("{} query failed: {}", label, err)
                })
                .to_string();
                status::Custom(Status::InternalServerError, RawJson(error_json))
            }
        }
    } else {
        status::Custom(
            Status::InternalServerError,
            RawJson(r#"{"error": "Query execution failed."}"#.to_string()),
        )
    }
}

#[options("/<_..>")]
fn preflight_handler() -> &'static str {
    ""
}

#[launch]
fn rocket() -> _ {

    rocket::build()
        .attach(CORS)
        .mount("/", routes![index, run_query, health, preflight_handler])
}
