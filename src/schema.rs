table! {
    sentences (id) {
        id -> Int4,
        user_id -> Int4,
        word_id -> Int4,
        sentence -> Text,
        is_pending -> Bool,
        is_mined -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    users (id) {
        id -> Int4,
        username -> Text,
        email -> Text,
        hash -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        token_generation -> Int4,
    }
}

table! {
    words (id) {
        id -> Int4,
        user_id -> Int4,
        dictionary_form -> Varchar,
        reading -> Varchar,
        frequency -> Int4,
        is_mined -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

joinable!(sentences -> users (user_id));
joinable!(sentences -> words (word_id));
joinable!(words -> users (user_id));

allow_tables_to_appear_in_same_query!(
    sentences,
    users,
    words,
);
