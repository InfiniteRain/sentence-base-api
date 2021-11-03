use crate::models::user::User;
use crate::schema::mining_batches;
use chrono::NaiveDateTime;
use diesel::result::Error;
use diesel::{PgConnection, RunQueryDsl};
use rocket::serde::Serialize;

#[derive(Queryable, Serialize, Identifiable, PartialEq, Associations, Debug, AsChangeset)]
#[belongs_to(User)]
#[table_name = "mining_batches"]
pub struct MiningBatch {
    pub id: i32,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "mining_batches"]
pub struct NewMining {
    pub user_id: i32,
}

impl MiningBatch {
    pub fn new(database_connection: &PgConnection, user: &User) -> Result<Self, Error> {
        diesel::insert_into(mining_batches::table)
            .values(NewMining { user_id: user.id })
            .get_result::<MiningBatch>(database_connection)
    }
}
