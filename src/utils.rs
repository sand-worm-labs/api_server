use regex::Regex;
use rocket::{http::Status, response::{content::RawJson, status}};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;

pub fn is_query_only(sql: String) -> bool {
    let blacklist = [
        "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER", "TRUNCATE", "REPLACE", "GRANT",
        "REVOKE",
    ];

    let upper = sql.to_uppercase();
    blacklist.iter().any(|kw| upper.contains(kw))
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
    let body = serde_json::to_string(&data).unwrap_or_else(|e| {
        json!({ "error": format!("Serialization failed: {}", e) }).to_string()
    });
    status::Custom(status, RawJson(body))
}

pub fn json_error<E: ToString>(err: E) -> status::Custom<RawJson<String>> {
    json_response(Status::InternalServerError, json!({ "error": format!("{}", err.to_string()) }))
}