use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::models::user::User;
use crate::schema::words;
use crate::schema::words::{dictionary_form as dictionary_form_column, reading as reading_column};
use diesel::pg::PgConnection;
use diesel::result::Error;
use diesel::BelongingToDsl;
use diesel::RunQueryDsl;
use diesel::SaveChangesDsl;
use rocket::serde::Serialize;

#[derive(Queryable, Serialize, Identifiable, PartialEq, Associations, Debug, AsChangeset)]
#[belongs_to(User)]
pub struct Word {
    pub id: i32,
    pub user_id: i32,
    pub dictionary_form: String,
    pub reading: String,
    pub frequency: i32,
    pub is_mined: bool,
}

#[derive(Insertable)]
#[table_name = "words"]
pub struct NewWord {
    pub user_id: i32,
    pub dictionary_form: String,
    pub reading: String,
}

impl Word {
    pub fn add_or_increase_frequency(
        database_connection: &PgConnection,
        user: &User,
        dictionary_form: &str,
        reading: &str,
    ) -> Result<Word, Error> {
        let potential_word: Result<Word, Error> = Word::belonging_to(user)
            .filter(dictionary_form_column.eq(dictionary_form))
            .filter(reading_column.eq(reading))
            .first(database_connection);

        match potential_word {
            Ok(mut found_word) => {
                found_word.frequency += 1;
                found_word.save_changes::<Word>(database_connection)
            }
            Err(_) => diesel::insert_into(words::table)
                .values(NewWord {
                    user_id: user.id,
                    dictionary_form: dictionary_form.to_string(),
                    reading: reading.to_string(),
                })
                .get_result::<Word>(database_connection),
        }
    }
}
