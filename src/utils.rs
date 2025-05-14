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
    ["SUI_MAINNET", "SUI_TESTNET", "SUI_DEVNET"]
        .iter()
        .any(|target| upper.contains(target))
}
