-- This file should undo anything in `up.sql`
-- Your SQL goes here
ALTER TABLE users
  DROP COLUMN token_generation
