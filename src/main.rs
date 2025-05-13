use sui_ql_core::interpreter::Interpreter  as SuiQlInterpreter;
use eql_core::interpreter::Interpreter  as EQlInterpreter;
use rocket::response::content::RawText;
use {
        // gluesql::{gluesql_mongo_storage::MongoStorage, gluesql_redis_storage::RedisStorage, prelude::Glue},
        std::fs,
};
mod utils;
use utils::is_query_only;

#[macro_use] extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/run?<query>")]
async fn run_query(query: &str) -> RawText<&'static str> {
    if utils::is_query_only(query)  == false {
        return RawText("Error: Only read-only SELECT queries are supported. Statements like CREATE, DROP, INSERT, UPDATE, and DELETE are not allowed");
    }
    match EQlInterpreter::run_program(query).await {
        Ok(result) => println!("{:?}", result),
        Err(err) => eprintln!("Query failed: {:?}", err),
    };
    RawText("true")
}


#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, run_query])
    
}
