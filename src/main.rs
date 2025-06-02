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
use sql_to_json::row_to_json;
use sui_ql_core::{
    common::query_result::QueryResult as SuiQueryResult,
    interpreter::Interpreter as SuiQlInterpreter,
};

use dotenv::dotenv;
use sqlx::any::AnyPool;
use crate::utils::json_error;


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
mod sql_to_json;

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
    pool: &State<AnyPool>,
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

    let query = &utils::remove_sql_comments(query);

    if !utils::is_query_only(query.to_owned()) {
        return status::Custom(
            Status::BadRequest,
            RawJson(
                r#"{"error": "Only SELECT queries are allowed. CREATE, DROP, INSERT, UPDATE, DELETE, and other write ops are blocked."} "#
                    .to_string(),
            ),
        );
    }

    if type_param == "rpc" {
        let (_label, result): (&str, Result<QueryResult, _>) = if utils::is_sui_rpc_query(query) {
            let res = SuiQlInterpreter::run_program(query).await.map(QueryResult::Sui);
            ("SUI_QL", res)
        } else {
            let res = EQlInterpreter::run_program(query).await.map(QueryResult::Eql);
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
        let flattened_query = utils::flatten_known_chain_tables(&query);
        if let Err(e) = gluesql::prelude::parse(&flattened_query) {
            return json_error(e);
        }

        let rows_json: Vec<Value> = match sqlx::query(&flattened_query).fetch_all(&**pool).await {
            Ok(rows) => rows.into_iter().map(|row| row_to_json(&row)).collect(),
            Err(e) => return json_error(e),
        };

        let wrapped_data: Vec<Value> = rows_json.into_iter().map(|row| json!(row)).collect();

        status::Custom(
            Status::Ok,
            RawJson(
                json!({
                    "type": "Wql",
                    "data": [
                        {
                            "result": {
                                "indexed": wrapped_data
                            }
                        }
                    ]
                })
                .to_string(),
            ),
        )
    }
}

#[options("/<_..>")]
fn preflight_handler() -> &'static str {
    ""
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // CryptoProvider::install_default();

    dotenv().ok();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Connecting to DB: {}", db_url);

    let pool = sqlx::AnyPool::connect(&db_url)
        .await
        .expect("Could not connect to DB");

    rocket::build()
        .manage(pool)
        .attach(CORS)
        .mount("/", routes![index, run_query, health, preflight_handler])
        .launch()
        .await?;

    Ok(())
}
