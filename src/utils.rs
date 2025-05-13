use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::Statement;

pub fn is_query_only(sql: &str) -> bool {
    let dialect = GenericDialect {};
    match Parser::parse_sql(&dialect, sql) {
        Ok(statements) => statements.iter().all(|stmt| matches!(stmt, Statement::Query(_))),
        Err(_) => false, // Not even valid SQL
    }
}
