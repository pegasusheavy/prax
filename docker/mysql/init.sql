-- =============================================================================
-- Prax ORM - MySQL Initialization Script
-- =============================================================================
-- This script runs when the MySQL container is first created.
-- It sets up the test database with proper permissions.

-- Create additional test databases for isolation
CREATE DATABASE IF NOT EXISTS prax_test_migrations;
CREATE DATABASE IF NOT EXISTS prax_test_integration;

-- Grant permissions to prax user
GRANT ALL PRIVILEGES ON prax_test.* TO 'prax'@'%';
GRANT ALL PRIVILEGES ON prax_test_migrations.* TO 'prax'@'%';
GRANT ALL PRIVILEGES ON prax_test_integration.* TO 'prax'@'%';
FLUSH PRIVILEGES;

-- Use main test database
USE prax_test;

-- Create a sample schema for testing (will be overwritten by migrations)
CREATE TABLE IF NOT EXISTS _prax_migrations (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Log initialization (visible in container logs)
SELECT 'Prax MySQL test database initialized successfully' AS message;

