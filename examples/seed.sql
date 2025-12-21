-- Example SQL seed file for Prax
-- Run with: prax db seed --seed-file examples/seed.sql

-- Clear existing data (optional)
-- TRUNCATE TABLE users, posts CASCADE;

-- Insert users
INSERT INTO users (id, email, name, role, created_at) VALUES
    (1, 'admin@example.com', 'Admin User', 'ADMIN', CURRENT_TIMESTAMP),
    (2, 'john@example.com', 'John Doe', 'USER', CURRENT_TIMESTAMP),
    (3, 'jane@example.com', 'Jane Smith', 'USER', CURRENT_TIMESTAMP),
    (4, 'bob@example.com', 'Bob Wilson', 'USER', CURRENT_TIMESTAMP),
    (5, 'alice@example.com', 'Alice Brown', 'MODERATOR', CURRENT_TIMESTAMP)
ON CONFLICT (id) DO NOTHING;

-- Insert posts
INSERT INTO posts (id, title, content, published, author_id, created_at) VALUES
    (1, 'Welcome to Prax', 'This is the first post using Prax ORM!', true, 1, CURRENT_TIMESTAMP),
    (2, 'Getting Started Guide', 'Learn how to use Prax in your Rust projects.', true, 1, CURRENT_TIMESTAMP),
    (3, 'My First Post', 'Hello world from John!', true, 2, CURRENT_TIMESTAMP),
    (4, 'Draft Post', 'This is a draft that is not published yet.', false, 2, CURRENT_TIMESTAMP),
    (5, 'Tips and Tricks', 'Some useful tips for Prax users.', true, 3, CURRENT_TIMESTAMP)
ON CONFLICT (id) DO NOTHING;

-- Insert comments
INSERT INTO comments (id, content, post_id, author_id, created_at) VALUES
    (1, 'Great post!', 1, 2, CURRENT_TIMESTAMP),
    (2, 'Thanks for sharing!', 1, 3, CURRENT_TIMESTAMP),
    (3, 'Very helpful guide.', 2, 4, CURRENT_TIMESTAMP),
    (4, 'Welcome John!', 3, 1, CURRENT_TIMESTAMP)
ON CONFLICT (id) DO NOTHING;

-- Update sequences (PostgreSQL specific)
-- SELECT setval('users_id_seq', (SELECT MAX(id) FROM users));
-- SELECT setval('posts_id_seq', (SELECT MAX(id) FROM posts));
-- SELECT setval('comments_id_seq', (SELECT MAX(id) FROM comments));

