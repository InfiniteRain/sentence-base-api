use crate::models::user::User;
use crate::models::word::Word;
use crate::schema::sentences;
use chrono::NaiveDateTime;
use diesel::result::Error;
use diesel::{PgConnection, RunQueryDsl};
use rocket::serde::Serialize;

#[derive(Queryable, Serialize, Identifiable, PartialEq, Associations)]
#[belongs_to(User)]
#[belongs_to(Word)]
pub struct Sentence {
    pub id: i32,
    pub user_id: i32,
    pub word_id: i32,
    pub sentence: String,
    pub is_pending: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub mining_batch_id: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "sentences"]
pub struct NewSentence {
    pub user_id: i32,
    pub word_id: i32,
    pub sentence: String,
}

impl Sentence {
    pub fn add(
        database_connection: &PgConnection,
        user: &User,
        word: &Word,
        sentence: &str,
    ) -> Result<Sentence, Error> {
        diesel::insert_into(sentences::table)
            .values(NewSentence {
                user_id: user.id,
                word_id: word.id,
                sentence: sentence.to_string(),
            })
            .get_result::<Sentence>(database_connection)
    }
}
