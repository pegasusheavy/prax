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

-- =============================================================================
-- Benchmark Tables
-- =============================================================================

-- Users table for benchmarking
CREATE TABLE IF NOT EXISTS users (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    age INT NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    score INT NOT NULL DEFAULT 0,
    attempts INT NOT NULL DEFAULT 0,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_status (status),
    INDEX idx_email (email),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Posts table for relation benchmarking
CREATE TABLE IF NOT EXISTS posts (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    user_id BIGINT NOT NULL,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    view_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_user_id (user_id),
    INDEX idx_published (published),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Seed initial data for benchmarks using a procedure
DELIMITER //

CREATE PROCEDURE seed_benchmark_data()
BEGIN
    DECLARE i INT DEFAULT 1;

    -- Seed users
    WHILE i <= 1000 DO
        INSERT IGNORE INTO users (name, email, age, status, role, verified, score)
        VALUES (
            CONCAT('User ', i),
            CONCAT('user', i, '@example.com'),
            20 + (i MOD 50),
            IF(i MOD 10 = 0, 'inactive', 'active'),
            CASE
                WHEN i MOD 100 = 0 THEN 'admin'
                WHEN i MOD 20 = 0 THEN 'moderator'
                ELSE 'user'
            END,
            i MOD 3 = 0,
            (i * 17) MOD 1000
        );
        SET i = i + 1;
    END WHILE;

    -- Seed posts
    SET i = 1;
    WHILE i <= 5000 DO
        INSERT INTO posts (title, content, user_id, published, view_count)
        VALUES (
            CONCAT('Post ', i, ' by User ', ((i - 1) MOD 1000) + 1),
            CONCAT('Content for post ', i),
            ((i - 1) MOD 1000) + 1,
            i MOD 5 != 0,
            (i * 13) MOD 10000
        );
        SET i = i + 1;
    END WHILE;
END //

DELIMITER ;

-- Execute seeding
CALL seed_benchmark_data();

-- Clean up
DROP PROCEDURE IF EXISTS seed_benchmark_data;

-- Log initialization (visible in container logs)
SELECT 'Prax MySQL test database initialized successfully' AS message;
SELECT CONCAT('Seeded ', (SELECT COUNT(*) FROM users), ' users and ', (SELECT COUNT(*) FROM posts), ' posts') AS stats;

