import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-views-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-views.page.html',
})
export class SchemaViewsPage {
  basicView = `// Define a view based on SQL query
view UserStats {
    id          Int
    email       String
    postCount   Int      @map("post_count")
    totalLikes  Int      @map("total_likes")
    lastPostAt  DateTime? @map("last_post_at")

    @@sql("""
        SELECT
            u.id,
            u.email,
            COUNT(p.id) as post_count,
            COALESCE(SUM(p.likes), 0) as total_likes,
            MAX(p.created_at) as last_post_at
        FROM users u
        LEFT JOIN posts p ON p.author_id = u.id
        GROUP BY u.id, u.email
    """)
}`;

  simpleView = `// Simple view for filtering active users
view ActiveUser {
    id        Int
    email     String
    name      String?
    createdAt DateTime @map("created_at")

    @@sql("SELECT id, email, name, created_at FROM users WHERE active = true")
}

// View with joined data
view PostWithAuthor {
    id          Int
    title       String
    authorName  String  @map("author_name")
    authorEmail String  @map("author_email")
    publishedAt DateTime? @map("published_at")

    @@sql("""
        SELECT
            p.id,
            p.title,
            u.name as author_name,
            u.email as author_email,
            p.published_at
        FROM posts p
        JOIN users u ON u.id = p.author_id
        WHERE p.published = true
    """)
}`;

  materializedView = `// Materialized view for expensive queries (PostgreSQL)
view MonthlyRevenue {
    month       String    @id
    year        Int
    revenue     Decimal
    orderCount  Int       @map("order_count")
    avgOrder    Decimal   @map("avg_order")

    @@sql("""
        SELECT
            TO_CHAR(created_at, 'YYYY-MM') as month,
            EXTRACT(YEAR FROM created_at)::INT as year,
            SUM(total) as revenue,
            COUNT(*) as order_count,
            AVG(total) as avg_order
        FROM orders
        WHERE status = 'COMPLETED'
        GROUP BY TO_CHAR(created_at, 'YYYY-MM'),
                 EXTRACT(YEAR FROM created_at)
    """)

    @@materialized          // Create as MATERIALIZED VIEW
    @@refreshInterval("1h") // Refresh every hour (if supported)
}`;

  viewWithRelations = `// Base models
model User {
    id    Int    @id @auto
    email String @unique
    name  String?
    posts Post[]
}

model Post {
    id        Int      @id @auto
    title     String
    content   String?
    published Boolean  @default(false)
    likes     Int      @default(0)
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int      @map("author_id")
    createdAt DateTime @default(now()) @map("created_at")
}

// View aggregating user stats
view UserDashboard {
    userId       Int     @id @map("user_id")
    email        String
    name         String?
    totalPosts   Int     @map("total_posts")
    publishedCnt Int     @map("published_count")
    totalLikes   Int     @map("total_likes")
    avgLikes     Float   @map("avg_likes")

    @@sql("""
        SELECT
            u.id as user_id,
            u.email,
            u.name,
            COUNT(p.id) as total_posts,
            COUNT(p.id) FILTER (WHERE p.published) as published_count,
            COALESCE(SUM(p.likes), 0) as total_likes,
            COALESCE(AVG(p.likes), 0) as avg_likes
        FROM users u
        LEFT JOIN posts p ON p.author_id = u.id
        GROUP BY u.id, u.email, u.name
    """)
}`;

  queryingViews = `// Views are queried like models (read-only)
use prax::generated::{UserStats, user_stats};

// Find all user stats
let stats = client
    .user_stats()
    .find_many()
    .exec()
    .await?;

// Filter and sort
let top_users = client
    .user_stats()
    .find_many()
    .where(user_stats::post_count::gte(10))
    .order_by(user_stats::total_likes::desc())
    .take(10)
    .exec()
    .await?;

// Find unique by ID
let user_stat = client
    .user_stats()
    .find_unique()
    .where(user_stats::id::equals(1))
    .exec()
    .await?;

// Aggregations work on views too
let total = client
    .user_stats()
    .aggregate()
    .sum(user_stats::total_likes)
    .avg(user_stats::post_count)
    .exec()
    .await?;`;

  databaseSpecific = `// PostgreSQL-specific view features
view SearchResults {
    id         Int
    title      String
    content    String
    rank       Float
    highlights String

    @@sql("""
        SELECT
            id,
            title,
            content,
            ts_rank(search_vector, query) as rank,
            ts_headline('english', content, query) as highlights
        FROM documents,
             to_tsquery('english', $1) query
        WHERE search_vector @@ query
        ORDER BY rank DESC
    """)

    // Parameterized view (query at runtime)
    @@parameterized
}

// MySQL-specific
view RecentOrders {
    id         Int
    total      Decimal
    status     String
    customerName String @map("customer_name")

    @@sql("""
        SELECT
            o.id,
            o.total,
            o.status,
            c.name as customer_name
        FROM orders o
        JOIN customers c ON c.id = o.customer_id
        WHERE o.created_at > DATE_SUB(NOW(), INTERVAL 30 DAY)
    """)
}`;

  viewBestPractices = `// ✅ Good: Descriptive view names that indicate aggregation
view UserPostStatistics { ... }
view MonthlyRevenueReport { ... }
view ActiveSubscriptions { ... }

// ✅ Good: Document complex views
/// Aggregated statistics for the admin dashboard
/// Updated: Real-time (not materialized)
/// Performance: ~200ms for 10k users
view AdminDashboardStats {
    ...
}

// ✅ Good: Use column aliases for clarity
@@sql("""
    SELECT
        user_id,          -- Clear reference
        COUNT(*) as order_count,  -- Descriptive alias
        SUM(total) as total_spent -- Meaningful name
    FROM orders
    GROUP BY user_id
""")

// ❌ Avoid: Overly complex views
// Break into smaller views or use CTEs

// ❌ Avoid: Views without indexes on base tables
// Ensure proper indexes exist for view queries`;
}


