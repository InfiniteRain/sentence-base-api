-- This file should undo anything in `up.sql`
DROP TRIGGER set_mining_batches_timestamps ON mining_batches;
DROP TABLE mining_batches;
ALTER TABLE sentences
  DROP COLUMN mining_batch_id;
ALTER TABLE sentences
  ADD COLUMN is_mined BOOLEAN NOT NULL DEFAULT FALSE;
