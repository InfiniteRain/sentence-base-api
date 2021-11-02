-- Your SQL goes here
CREATE TABLE sentences (
  id SERIAL PRIMARY KEY,
  user_id INT NOT NULL,
  word_id INT NOT NULL,
  sentence TEXT NOT NULL,
  is_pending BOOLEAN NOT NULL DEFAULT TRUE,
  is_mined BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT fk_sentences_user_id
    FOREIGN KEY (user_id)
    REFERENCES users(id),
  CONSTRAINT fk_sentences_word_id
    FOREIGN KEY (word_id)
    REFERENCES words(id)
);
CREATE INDEX idx_sentences_is_pending ON sentences(is_pending);
