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

-- Log initialization
DO $$
BEGIN
    RAISE NOTICE 'Prax PostgreSQL test database initialized successfully';
END $$;

