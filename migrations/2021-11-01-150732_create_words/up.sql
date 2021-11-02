-- Your SQL goes here
CREATE TABLE words (
  id SERIAL PRIMARY KEY,
  user_id INT NOT NULL,
  dictionary_form VARCHAR (128) NOT NULL,
  reading varchar (128) NOT NULL,
  frequency INT NOT NULL DEFAULT 1,
  is_mined BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (user_id, dictionary_form, reading),
  CONSTRAINT fk_words_user_id
    FOREIGN KEY (user_id)
    REFERENCES users(id)
);
CREATE INDEX idx_words_dictionary_form ON words(dictionary_form);
CREATE INDEX idx_words_reading ON words(reading);
