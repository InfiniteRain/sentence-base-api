#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use rocket::{Build, Rocket};

mod analyzer;
mod database;
mod field_validator;
pub mod models;
mod responses;
mod routes;
mod schema;

pub fn rocket(database_url: &str) -> Rocket<Build> {
    rocket::build()
        .manage(database::init_pool(database_url.to_string()))
        .mount(
            "/",
            routes![routes::analyzer::analyze, routes::authentication::register],
        )
        .register("/", catchers![routes::catcher::default])
}
