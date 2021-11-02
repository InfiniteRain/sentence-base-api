-- This file should undo anything in `up.sql`
DROP TRIGGER set_sentences_timestamps ON sentences;
DROP TRIGGER set_words_timestamps ON words;
DROP TRIGGER set_users_timestamps ON users;
DROP FUNCTION trigger_set_timestamp;
