use regex::Regex;
use rocket::{
    http::Status,
    response::{content::RawJson, status},
};

use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use sqlx::Row;
use std::collections::HashSet;

pub fn remove_sql_comments(sql: &str) -> String {
    // Regexes for each comment style
    // 1. Block comments: /* ... */
    let re_block = Regex::new(r"/\*[\s\S]*?\*/").unwrap();
    // 2. -- single-line comments
    let re_line1 = Regex::new(r"--[^\r\n]*").unwrap();
    // 3. // single-line comments
    let re_line2 = Regex::new(r"//[^\r\n]*").unwrap();

    // Apply in order: block first, then single-line
    let no_block = re_block.replace_all(sql, "");
    let no_line1 = re_line1.replace_all(&no_block, "");
    let no_line2 = re_line2.replace_all(&no_line1, "");

    no_line2.into_owned()
}

 const BLACKLIST: &[&str] = &[
        // DML
        "INSERT", "UPDATE", "DELETE", "MERGE", "UPSERT", "TRUNCATE", "RETURNING", "OVERRIDING SYSTEM VALUE",

        // DDL
        "CREATE", "ALTER", "DROP", "RENAME", "COMMENT", "REINDEX", "CLUSTER", "DISCARD",

        // Transactions
        "BEGIN", "COMMIT", "ROLLBACK", "SAVEPOINT", "RELEASE", "PREPARE", "DEALLOCATE",

        // Roles & Users
        "GRANT", "REVOKE", "CREATE USER", "DROP USER", "CREATE ROLE", "DROP ROLE", "ALTER USER", "ALTER ROLE",
        "SET ROLE", "RESET ROLE", "SESSION AUTHORIZATION", "SET SESSION AUTHORIZATION", "LOGIN", "PASSWORD",

        // Schema & Object Management
        "CREATE TABLE", "DROP TABLE", "ALTER TABLE", "UNLOGGED", "TEMP TABLE", "TEMPORARY",
        "CREATE SEQUENCE", "ALTER SEQUENCE", "DROP SEQUENCE",
        "CREATE VIEW", "DROP VIEW", "ALTER VIEW", "MATERIALIZED", "REFRESH MATERIALIZED VIEW",
        "CREATE FUNCTION", "ALTER FUNCTION", "DROP FUNCTION",
        "CREATE PROCEDURE", "DROP PROCEDURE", "CALL",
        "CREATE TRIGGER", "DROP TRIGGER",
        "CREATE RULE", "DROP RULE",
        "CREATE INDEX", "DROP INDEX", "USING BTREE", "USING GIN", "USING HASH",
        "CREATE EXTENSION", "ALTER EXTENSION", "DROP EXTENSION",
        "CREATE SCHEMA", "DROP SCHEMA", "ALTER SCHEMA",

        // Copy & External
        "COPY", "DO", "LISTEN", "NOTIFY", "UNLISTEN", "EXPLAIN", "ANALYZE",

        // Runtime and system information
        "SHOW", "SET", "RESET", "CONFIG", "LOAD", "VACUUM", "ANALYZE", "CHECKPOINT", "REASSIGN OWNED",
        "pg_sleep", "pg_cancel_backend", "pg_terminate_backend", "pg_reload_conf", "pg_rotate_logfile",
        "pg_stat_reset", "pg_logical_emit_message",

        // Low-level system functions
        "pg_backend_pid", "pg_postmaster_start_time", "pg_current_xact_id", "txid_current",
        "pg_is_in_recovery", "pg_last_xact_replay_timestamp", "pg_switch_wal", "pg_create_physical_replication_slot",
        "pg_drop_replication_slot", "pg_create_logical_replication_slot", "pg_drop_logical_replication_slot",

        // WAL, Replication
        "pg_current_wal_lsn", "pg_wal_lsn_diff", "pg_replication_origin", "pg_create_restore_point",
        "pg_start_backup", "pg_stop_backup", "pg_promote",

        // System views/tables
        "pg_stat_", "pg_replication_", "pg_settings", "pg_file_", "pg_ls_", "pg_log_", "pg_read_file",
        "pg_read_binary_file", "pg_stat_file", "pg_tablespace", "pg_database", "pg_user", "pg_roles",
        "pg_shadow", "pg_authid", "pg_auth_members", "pg_group",

        // Size & config introspection
        "pg_size_pretty", "pg_table_size", "pg_database_size", "pg_indexes_size",
        "pg_total_relation_size", "pg_column_size", "pg_relation_size",

        // Network/system
        "inet_client_addr", "inet_client_port", "inet_server_addr", "inet_server_port",
        "pg_hba_file_rules", "pg_ident_file_mappings",

        // Injection & abuse patterns
        "--", "/*", "*/", "#", ";", ";--", "OR 1=1", "' OR '1'='1", "\" OR \"1\"=\"1", "UNION SELECT",
        "INFORMATION_SCHEMA", "SYSTEM_USER", "CURRENT_CATALOG", "CURRENT_SCHEMA",

        // Encoding & internal config
        "client_encoding", "application_name", "standard_conforming_strings",
        "statement_timeout", "idle_in_transaction_session_timeout", "log_min_duration_statement",
        "work_mem", "maintenance_work_mem", "shared_buffers", "effective_cache_size",

        // User identity
        "user", "current_user", "session_user", "system_user", "is_superuser",

        // Time/locale
        "datestyle", "timezone",

        // Extensions or plugins
        "plpgsql", "pgcrypto", "postgis", "pgstattuple", "snowball", "tsearch2", "uuid-ossp", "xml2","size"
    ];
    
pub fn is_query_only(sql: String) -> bool { 
     !is_blacklisted_query(&sql)
}

fn is_blacklisted_query(sql: &str) -> bool {
    const BLACKLIST_REGEX: &str = r###"(?i)\b(INSERT|UPDATE|DELETE|MERGE|UPSERT|TRUNCATE|RETURNING|OVERRIDING\s+SYSTEM\s+VALUE|CREATE|ALTER|DROP|RENAME|COMMENT|REINDEX|CLUSTER|DISCARD|BEGIN|COMMIT|ROLLBACK|SAVEPOINT|RELEASE|PREPARE|DEALLOCATE|GRANT|REVOKE|CREATE\s+USER|DROP\s+USER|CREATE\s+ROLE|DROP\s+ROLE|ALTER\s+USER|ALTER\s+ROLE|SET\s+ROLE|RESET\s+ROLE|SESSION\s+AUTHORIZATION|SET\s+SESSION\s+AUTHORIZATION|LOGIN|PASSWORD|CREATE\s+TABLE|DROP\s+TABLE|ALTER\s+TABLE|UNLOGGED|TEMP\s+TABLE|TEMPORARY|CREATE\s+SEQUENCE|ALTER\s+SEQUENCE|DROP\s+SEQUENCE|CREATE\s+VIEW|DROP\s+VIEW|ALTER\s+VIEW|MATERIALIZED|REFRESH\s+MATERIALIZED\s+VIEW|CREATE\s+FUNCTION|ALTER\s+FUNCTION|DROP\s+FUNCTION|CREATE\s+PROCEDURE|DROP\s+PROCEDURE|CALL|CREATE\s+TRIGGER|DROP\s+TRIGGER|CREATE\s+RULE|DROP\s+RULE|CREATE\s+INDEX|DROP\s+INDEX|USING\s+BTREE|USING\s+GIN|USING\s+HASH|CREATE\s+EXTENSION|ALTER\s+EXTENSION|DROP\s+EXTENSION|CREATE\s+SCHEMA|DROP\s+SCHEMA|ALTER\s+SCHEMA|COPY|DO|LISTEN|NOTIFY|UNLISTEN|EXPLAIN|ANALYZE|SHOW|SET|RESET|CONFIG|LOAD|VACUUM|CHECKPOINT|REASSIGN\s+OWNED|pg_sleep|pg_cancel_backend|pg_terminate_backend|pg_reload_conf|pg_rotate_logfile|pg_stat_reset|pg_logical_emit_message|pg_backend_pid|pg_postmaster_start_time|pg_current_xact_id|txid_current|pg_is_in_recovery|pg_last_xact_replay_timestamp|pg_switch_wal|pg_create_physical_replication_slot|pg_drop_replication_slot|pg_create_logical_replication_slot|pg_drop_logical_replication_slot|pg_current_wal_lsn|pg_wal_lsn_diff|pg_replication_origin|pg_create_restore_point|pg_start_backup|pg_stop_backup|pg_promote|pg_stat_|pg_replication_|pg_settings|pg_file_|pg_ls_|pg_log_|pg_read_file|pg_read_binary_file|pg_stat_file|pg_tablespace|pg_database|pg_user|pg_roles|pg_shadow|pg_authid|pg_auth_members|pg_group|pg_size_pretty|pg_table_size|pg_database_size|pg_indexes_size|pg_total_relation_size|pg_column_size|pg_relation_size|inet_client_addr|inet_client_port|inet_server_addr|inet_server_port|pg_hba_file_rules|pg_ident_file_mappings|--|/\*|\*/|#|;|;--|OR\s+1=1|' OR '1'='1|\" OR \"1\"=\"1|UNION\s+SELECT|INFORMATION_SCHEMA|SYSTEM_USER|CURRENT_CATALOG|CURRENT_SCHEMA|client_encoding|application_name|standard_conforming_strings|statement_timeout|idle_in_transaction_session_timeout|log_min_duration_statement|work_mem|maintenance_work_mem|shared_buffers|effective_cache_size|user|current_user|session_user|system_user|is_superuser|datestyle|timezone|plpgsql|pgcrypto|postgis|pgstattuple|snowball|tsearch2|uuid-ossp|xml2|size)\b"###;
    let re = Regex::new(BLACKLIST_REGEX).unwrap();
    re.is_match(sql)
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
        // Decimal / Numeric
        "NUMERIC" | "DECIMAL" => {
            // Use String because Decimal might need special parsing
            json!(row.try_get::<Option<String>, _>(i).ok().flatten())
        }
        "BOOL" => json!(row.try_get::<Option<bool>, _>(i).ok().flatten()),

        // Text types
        "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" | "UUID" => {
            json!(row.try_get::<Option<String>, _>(i).ok().flatten())
        }

        // Binary data
        "BYTEA" => row
            .try_get::<Option<Vec<u8>>, _>(i)
            .ok()
            .flatten()
            .map(|b| json!(base64::encode(b)))
            .unwrap_or(json!(null)),

        // JSON types
        "JSON" | "JSONB" => row
            .try_get::<Option<Value>, _>(i)
            .ok()
            .flatten()
            .unwrap_or(json!(null)),

        // Date/Time types
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

        // Arrays (basic example for int arrays)
        "_INT4" => row
            .try_get::<Option<Vec<i32>>, _>(i)
            .ok()
            .flatten()
            .map(|arr| json!(arr))
            .unwrap_or(json!(null)),

        // Default fallback for anything else
        _ => {
            let val: Result<Option<String>, _> = row.try_get(i);
            val.map(|v| json!(v)).unwrap_or(json!(null))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::remove_sql_comments;

    #[test]
    fn test_remove_line_comments() {
        let sql = "SELECT * FROM users; -- fetch all users\nINSERT INTO users VALUES (1); // add seed";
        let expected = "SELECT * FROM users; \nINSERT INTO users VALUES (1); ";
        assert_eq!(remove_sql_comments(sql), expected);
    }

    #[test]
    fn test_remove_block_comments() {
        let sql = "/* setup */\nCREATE TABLE users (id INT); /* trailing */";
        let expected = "\nCREATE TABLE users (id INT); ";
        assert_eq!(remove_sql_comments(sql), expected);
    }

    #[test]
    fn test_combined_comments() {
        let sql = r#"
            /* start */
            SELECT 1; -- comment
            // another
            INSERT INTO t VALUES ('--not a comment in string'); /* end */
        "#;
        let cleaned = remove_sql_comments(sql);
        assert!(cleaned.contains("SELECT 1;"));
        assert!(cleaned.contains("INSERT INTO t VALUES ('--not a comment in string');"));
        assert!(!cleaned.contains("/* start */"));
        assert!(!cleaned.contains("-- comment"));
        assert!(!cleaned.contains("// another"));
        assert!(!cleaned.contains("/* end */"));
    }
}
