-- =============================================================================
-- Prax ORM - PostgreSQL Initialization Script
-- =============================================================================
-- This script runs when the PostgreSQL container is first created.
-- It sets up the test database with proper permissions and extensions.

-- Enable useful extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create additional test databases for isolation
CREATE DATABASE prax_test_migrations;
CREATE DATABASE prax_test_integration;

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE prax_test TO prax;
GRANT ALL PRIVILEGES ON DATABASE prax_test_migrations TO prax;
GRANT ALL PRIVILEGES ON DATABASE prax_test_integration TO prax;

-- Connect to main test database and set up schema
\c prax_test

-- Create a sample schema for testing (will be overwritten by migrations)
CREATE TABLE IF NOT EXISTS _prax_migrations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- =============================================================================
-- Benchmark Tables
-- =============================================================================

-- Users table for benchmarking
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    age INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    score INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Posts table for relation benchmarking
CREATE TABLE IF NOT EXISTS posts (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    view_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for benchmark queries
CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);
CREATE INDEX IF NOT EXISTS idx_posts_user_id ON posts(user_id);
CREATE INDEX IF NOT EXISTS idx_posts_published ON posts(published);

-- Seed some initial data for benchmarks
INSERT INTO users (name, email, age, status, role, verified, score)
SELECT
    'User ' || i,
    'user' || i || '@example.com',
    20 + (i % 50),
    CASE WHEN i % 10 = 0 THEN 'inactive' ELSE 'active' END,
    CASE WHEN i % 100 = 0 THEN 'admin' WHEN i % 20 = 0 THEN 'moderator' ELSE 'user' END,
    i % 3 = 0,
    (i * 17) % 1000
FROM generate_series(1, 1000) AS i
ON CONFLICT (email) DO NOTHING;

-- Seed posts
INSERT INTO posts (title, content, user_id, published, view_count)
SELECT
    'Post ' || i || ' by User ' || ((i % 1000) + 1),
    'Content for post ' || i,
    ((i % 1000) + 1),
    i % 5 != 0,
    (i * 13) % 10000
FROM generate_series(1, 5000) AS i;

-- Log initialization
DO $$
BEGIN
    RAISE NOTICE 'Prax PostgreSQL test database initialized successfully';
    RAISE NOTICE 'Seeded % users and % posts', (SELECT COUNT(*) FROM users), (SELECT COUNT(*) FROM posts);
END $$;

