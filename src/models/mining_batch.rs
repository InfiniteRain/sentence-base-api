use crate::diesel::prelude::*;
use crate::frequency_list::JpFrequencyList;
use crate::models::sentence::Sentence;
use crate::models::user::{User, UserSentenceEntry};
use crate::models::word::Word;
use crate::schema::mining_batches;
use crate::schema::words::dsl::words as dsl_words;
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

    pub fn get_sentences(
        &self,
        database_connection: &PgConnection,
        frequency_list: &JpFrequencyList,
    ) -> Result<Vec<UserSentenceEntry>, Error> {
        let rows: Vec<(Sentence, Word)> = Sentence::belonging_to(self)
            .inner_join(dsl_words)
            .load(database_connection)?;

        let sentences = rows
            .into_iter()
            .map(|(sentence, word)| UserSentenceEntry::new(&word, &sentence, frequency_list))
            .collect();

        Ok(sentences)
    }
}
