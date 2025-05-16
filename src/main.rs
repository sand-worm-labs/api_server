use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Header, Status},
    response::{content::RawJson, status, Response},
    Request, State,
};

use serde::Serialize;

use eql_core::{
    common::query_result::QueryResult as EqlQueryResult, interpreter::Interpreter as EQlInterpreter,
};

use serde_json::{json, Value};
use sui_ql_core::{
    common::query_result::QueryResult as SuiQueryResult,
    interpreter::Interpreter as SuiQlInterpreter,
};

use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Column, PgPool, Row};
use std::env;
use utils::json_error;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Attaching CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
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
async fn run_query(
    query: &str,
    type_param: &str,
    pool: &State<PgPool>,
) -> status::Custom<RawJson<String>> {
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
        let (_label, result): (&str, Result<QueryResult, _>) = if utils::is_sui_rpc_query(query) {
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
                Err(err) => json_error(err),
            },
            Err(err) => json_error(err),
        }
    } else {
        let flattened_query = utils::flatten_known_chain_tables(query);
        match gluesql::prelude::parse(&flattened_query) {
            Ok(p) => p,
            Err(e) => return json_error(e),
        };

        let rows_json: Vec<Value> = match sqlx::query(&flattened_query)
            .persistent(false)
            .fetch_all(&**pool)
            .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|row| {
                    let mut obj = serde_json::Map::new();
                    for (i, column) in row.columns().iter().enumerate() {
                        let column_name = column.name();
                        let value: Result<Value, _> = row.try_get(i); // uses Decode support
                        obj.insert(column_name.to_string(), value.unwrap_or(json!(null)));
                    }
                    Value::Object(obj)
                })
                .collect(),
            Err(e) => return json_error(e),
        };

        let wrapped_data: Vec<Value> = rows_json
            .into_iter()
            .map(|row| json!(row))
            .collect();

        return status::Custom(
            Status::Ok,
            RawJson(
                json!({
                    "type": "Wql",
                    "data": [
                        {"result": wrapped_data}
                    ]
                })
                .to_string(),
            ),
        );
    }
}

#[options("/<_..>")]
fn preflight_handler() -> &'static str {
    ""
}

#[launch]
async fn rocket() -> _ {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Connecting to DB: {}", db_url);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("Could not connect to DB");

    rocket::build()
        .manage(pool)
        .attach(CORS)
        .mount("/", routes![index, run_query, health, preflight_handler])
}
