-- Your SQL goes here
CREATE TABLE mining_batches (
  id SERIAL PRIMARY KEY,
  user_id INT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT fk_mining_batches_user_id
    FOREIGN KEY (user_id)
    REFERENCES users(id)
);

CREATE TRIGGER set_mining_batches_timestamps
  BEFORE UPDATE ON mining_batches
  FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

ALTER TABLE sentences
  ADD COLUMN mining_batch_id INT;
ALTER TABLE sentences
  ADD CONSTRAINT fk_sentences_mining_batch_id
  FOREIGN KEY (mining_batch_id)
  REFERENCES mining_batches(id);
ALTER TABLE sentences
  DROP COLUMN is_mined;
