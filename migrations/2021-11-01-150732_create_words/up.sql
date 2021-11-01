-- Your SQL goes here
CREATE TABLE words (
  id SERIAL PRIMARY KEY,
  user_id INT NOT NULL,
  word VARCHAR (128) NOT NULL,
  frequency INT NOT NULL DEFAULT 1,
  is_mined BOOLEAN NOT NULL DEFAULT FALSE,
  UNIQUE (user_id, word),
  CONSTRAINT fk_words_user_id
    FOREIGN KEY (user_id)
    REFERENCES users(id)
);
CREATE INDEX idx_words_word ON words(word);
