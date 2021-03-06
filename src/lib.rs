#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use crate::frequency_list::JpFrequencyList;
use rocket::{Build, Rocket};

mod analyzer;
mod database;
mod field_validator;
mod frequency_list;
pub mod helpers;
pub mod jwt;
pub mod models;
pub mod responses;
pub mod routes;
pub mod schema;

pub fn rocket(database_url: &str) -> Rocket<Build> {
    dotenv::dotenv().ok();

    let database_pool = database::init_pool(database_url.to_string());
    let frequency_list = JpFrequencyList::new();

    rocket::build()
        .manage(database_pool)
        .manage(frequency_list)
        .mount(
            "/",
            routes![
                routes::analyzer::analyze,
                // routes::authentication::register,
                routes::authentication::login,
                routes::authentication::refresh,
                routes::authentication::me,
                routes::sentences::new,
                routes::sentences::get,
                routes::sentences::delete,
                routes::sentences::new_batch,
                routes::sentences::get_batch,
                routes::sentences::get_all_batches,
            ],
        )
        .register("/", catchers![routes::catcher::default])
}
