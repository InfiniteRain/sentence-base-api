use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{self, FromRequest, Outcome};
use rocket::{Request, State};
use std::ops::Deref;

type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_pool(database_url: String) -> Pool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::new(manager).expect("Database pool")
}

pub struct DbConnection(pub r2d2::PooledConnection<ConnectionManager<PgConnection>>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DbConnection {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<DbConnection, ()> {
        let pool = try_outcome!(request.guard::<&State<Pool>>().await);
        match pool.get() {
            Ok(connection) => Outcome::Success(DbConnection(connection)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

impl Deref for DbConnection {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
