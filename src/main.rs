use rocket::Error;

#[rocket::main]
async fn main() -> Result<(), Error> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    sentence_base::rocket(&database_url).launch().await
}
