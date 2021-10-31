#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use rocket::{Build, Rocket};

mod analyzer;
mod database;
mod field_validator;
pub mod jwt;
pub mod models;
mod responses;
mod routes;
mod schema;

pub fn rocket(database_url: &str) -> Rocket<Build> {
    dotenv::dotenv().ok();

    rocket::build()
        .manage(database::init_pool(database_url.to_string()))
        .mount(
            "/",
            routes![
                routes::analyzer::analyze,
                routes::authentication::register,
                routes::authentication::login,
                routes::authentication::me
            ],
        )
        .register("/", catchers![routes::catcher::default])
}
