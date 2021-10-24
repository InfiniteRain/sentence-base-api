#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

mod analyzer;
mod database;
mod responses;
mod routes;
mod schema;

#[rocket::main]
#[allow(unused_must_use)]
async fn main() {
    dotenv::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    rocket::build()
        .manage(database::init_pool(database_url))
        .mount("/", routes![routes::analyzer::analyze])
        .register("/", catchers![routes::catcher::default])
        .launch()
        .await;
}
