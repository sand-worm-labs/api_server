use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use regex::Regex;
use rocket::{
    http::Status,
    response::{content::RawJson, status},
};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use sqlx::Row;
use std::collections::HashSet;

pub fn is_query_only(sql: String) -> bool {
    const BLACKLIST: &[&str] = &[
        "INSERT",
        "UPDATE",
        "DELETE",
        "CREATE",
        "DROP",
        "ALTER",
        "TRUNCATE",
        "REPLACE",
        "GRANT",
        "REVOKE",
        "SHOW",
        "USER",
        "SET",
        "EXECUTE",
        "CALL",
        "COPY",
        "current_database()",
        "current_user()",
        "session_user()",
        "inet_client_addr()",
        "inet_server_addr()",
        "version()",
        "pg_backend_pid()",
        "pg_postmaster_start_time()",
        "pg_current_xact_id()",
        "pg_is_in_recovery()",
        "txid_current()",
        "pg_size_pretty()",
    ];
    let upper = sql.to_uppercase();
    !BLACKLIST.iter().any(|kw| upper.contains(kw))
}

pub fn is_sui_rpc_query(query: &str) -> bool {
    let upper = query.to_uppercase();
    ["SUI", "SUITEST", "SUIDEV"]
        .iter()
        .any(|target| upper.contains(target))
}

pub fn flatten_known_chain_tables(sql: &str) -> String {
    let known_chains: HashSet<&'static str> = [
        "sui", "suidev", "suitest", // Non-EVM
        "eth", "sepolia", "arb", "base", "blast", "op", "poly", "mycelium", "mnt", "zks", "taiko",
        "celo", "avax", "scroll", "bnb", "linea", "zora", "glmr", "movr", "ron", "ftm", "kava",
        "gno", "mekong", "mina",
    ]
    .into_iter()
    .collect();

    let re = Regex::new(r"\b([a-zA-Z0-9_]+)\.([a-zA-Z0-9_]+)\b").unwrap();

    re.replace_all(sql, |caps: &regex::Captures| {
        let chain = &caps[1];
        let table = &caps[2];
        if known_chains.contains(chain) {
            format!("{}_{}", chain, table)
        } else {
            caps[0].to_string() // Leave it untouched
        }
    })
    .to_string()
}

pub fn json_response<T: Serialize>(status: Status, data: T) -> status::Custom<RawJson<String>> {
    let body = serde_json::to_string(&data)
        .unwrap_or_else(|e| json!({ "error": format!("Serialization failed: {}", e) }).to_string());
    status::Custom(status, RawJson(body))
}

pub fn json_error<E: ToString>(err: E) -> status::Custom<RawJson<String>> {
    let err = err.to_string();
    json_response(
        Status::InternalServerError,
        json!({ "error": format!("{}", err.to_string()) }),
    )
}

pub fn decode_column_to_json(row: &sqlx::postgres::PgRow, i: usize, type_name: &str) -> Value {
    match type_name {
        // Numeric types
        "INT2" | "INT4" => json!(row.try_get::<Option<i32>, _>(i).ok().flatten()),
        "INT8" => json!(row.try_get::<Option<i64>, _>(i).ok().flatten()),
        "FLOAT4" => json!(row.try_get::<Option<f32>, _>(i).ok().flatten()),
        "FLOAT8" => json!(row.try_get::<Option<f64>, _>(i).ok().flatten()),
        // .try_get::<Option<Decimal>, _>(i).ok().flatten()),
        "BOOL" => json!(row.try_get::<Option<bool>, _>(i).ok().flatten()),

        "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" | "UUID" => {
            json!(row.try_get::<Option<String>, _>(i).ok().flatten())
        }

        "BYTEA" => row
            .try_get::<Option<Vec<u8>>, _>(i)
            .ok()
            .flatten()
            .map(|b| json!(base64::encode(b)))
            .unwrap_or(json!(null)),

        "JSON" | "JSONB" => row
            .try_get::<Option<Value>, _>(i)
            .ok()
            .flatten()
            .unwrap_or(json!(null)),

        "DATE" => row
            .try_get::<Option<chrono::NaiveDate>, _>(i)
            .map(|opt| opt.map(|d| json!(d.to_string())).unwrap_or(json!(null)))
            .unwrap_or(json!(null)),
        "TIME" => row
            .try_get::<Option<chrono::NaiveTime>, _>(i)
            .map(|v| v.map(|t| json!(t.to_string())).unwrap_or(json!(null)))
            .unwrap_or(json!(null)),
        "TIMESTAMP" => row
            .try_get::<Option<chrono::NaiveDateTime>, _>(i)
            .map(|v| v.map(|ts| json!(ts.to_string())).unwrap_or(json!(null)))
            .unwrap_or(json!(null)),
        "TIMESTAMPTZ" => row
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i)
            .map(|v| v.map(|ts| json!(ts.to_rfc3339())).unwrap_or(json!(null)))
            .unwrap_or(json!(null)),

        _ => {
            let val: Result<Option<String>, _> = row.try_get(i);
            val.map(|v| json!(v)).unwrap_or(json!(null))
        }
    }
}
