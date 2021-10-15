#[macro_use]
extern crate rocket;

mod analyzer;
mod responses;
mod routes;

#[rocket::main]
#[allow(unused_must_use)]
async fn main() {
    rocket::build()
        .mount("/", routes![routes::analyze])
        .register("/", catchers![routes::default])
        .launch()
        .await;
}
