-- Add platform_id back to sources table
ALTER TABLE library.sources ADD COLUMN platform_id VARCHAR(255) NOT NULL DEFAULT 'library';
