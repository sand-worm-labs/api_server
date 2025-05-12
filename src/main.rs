use sui_ql_core::interpreter::Interpreter;

#[macro_use] extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/run?<query>")]
async fn run_query(query: &str) -> Result<String, Status> {
    query.to_owned()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
