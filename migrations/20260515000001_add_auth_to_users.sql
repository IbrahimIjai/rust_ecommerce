-- Add role enum
CREATE TYPE user_role AS ENUM ('customer', 'admin');

-- Extend users table with auth columns
ALTER TABLE users
    ADD COLUMN password_hash TEXT NOT NULL DEFAULT '',
    ADD COLUMN role          user_role NOT NULL DEFAULT 'customer',
    ADD COLUMN is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN reset_token             TEXT,
    ADD COLUMN reset_token_expires_at  TIMESTAMPTZ;

-- Drop the temporary default (existing rows got '' which is invalid but migration-safe)
ALTER TABLE users ALTER COLUMN password_hash DROP DEFAULT;

-- Indexes for auth lookups
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_reset_token ON users(reset_token)
    WHERE reset_token IS NOT NULL;
