-- Add embedding column for semantic search (384-dim float32 vectors stored as BLOB)
ALTER TABLE memories ADD COLUMN embedding BLOB;
