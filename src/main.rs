use diesel::pg::PgConnection;
use diesel::Connection;
use rocket::Error;

#[rocket::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database_connection =
        PgConnection::establish(&database_url).expect("database connection should be established");
    diesel_migrations::run_pending_migrations(&database_connection).expect("migrations should run");

    sentence_base::rocket(&database_url).launch().await
}
