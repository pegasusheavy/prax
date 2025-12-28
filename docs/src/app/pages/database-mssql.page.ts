import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-mssql-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-mssql.page.html',
})
export class DatabaseMssqlPage {
  connectionExample = `// prax.toml configuration
[database]
provider = "mssql"
url = "mssql://user:password@localhost:1433/mydb"

# Connection pool settings
[database.pool]
max_connections = 10
min_connections = 2
acquire_timeout = "30s"

# TLS settings for Azure SQL
[database.tls]
enabled = true
trust_server_certificate = false`;

  schemaExample = `// MSSQL-specific schema features
generator client {
  provider = "prax-mssql"
  output   = "./generated"
}

datasource db {
  provider = "mssql"
  url      = env("DATABASE_URL")
}

model User {
  id        Int      @id @auto
  email     String   @unique @db.NVarChar(255)
  name      String?  @db.NVarChar(100)
  bio       String?  @db.NVarChar(Max)
  createdAt DateTime @default(now()) @db.DateTime2

  @@index([email])
  @@map("users")
}`;

  indexedViewExample = `// MSSQL Indexed Views (Materialized Views)
view OrderSummary {
  customerId   Int     @map("customer_id")
  orderCount   Int     @map("order_count")
  totalSpent   Decimal @map("total_spent")

  @@sql("""
    SELECT
      customer_id,
      COUNT_BIG(*) as order_count,
      SUM(total) as total_spent
    FROM orders WITH (NOLOCK)
    GROUP BY customer_id
  """)

  @@materialized
  @@index([customerId]) // Creates clustered index
}`;

  mergeExample = `use prax::upsert::{Upsert, ConflictTarget, ConflictAction};

// MSSQL MERGE statement for upsert
let upsert = Upsert::new("users")
    .columns(["email", "name", "updated_at"])
    .values(["alice@example.com", "Alice", "2024-01-01"])
    .on_conflict(
        ConflictTarget::columns(["email"]),
        ConflictAction::do_update(["name", "updated_at"])
    )
    .build_mssql();

// Generates:
// MERGE INTO users AS target
// USING (VALUES (@P1, @P2, @P3)) AS source (email, name, updated_at)
// ON target.email = source.email
// WHEN MATCHED THEN
//   UPDATE SET name = source.name, updated_at = source.updated_at
// WHEN NOT MATCHED THEN
//   INSERT (email, name, updated_at) VALUES (source.email, source.name, source.updated_at);`;

  outputClauseExample = `use prax::advanced::Returning;

// MSSQL OUTPUT clause
let insert = client
    .user()
    .create(user::Create {
        email: "new@example.com".into(),
        name: Some("New User".into()),
    })
    .returning(Returning::columns(["id", "created_at"]))
    .exec()
    .await?;

// Generates: INSERT INTO users (...) OUTPUT INSERTED.id, INSERTED.created_at VALUES (...)

// For updates
let updated = client
    .user()
    .update(user::id::equals(1))
    .data(user::Update { name: Some("Updated".into()), ..Default::default() })
    .returning(Returning::all())
    .exec()
    .await?;

// Generates: UPDATE users SET ... OUTPUT INSERTED.* WHERE id = @P1`;

  crossApplyExample = `use prax::advanced::{LateralJoin, LateralJoinBuilder};

// MSSQL CROSS APPLY (equivalent to LATERAL JOIN)
let query = LateralJoin::new(
    "SELECT TOP 3 * FROM orders WHERE customer_id = c.id ORDER BY created_at DESC",
    "recent_orders"
)
.cross() // CROSS APPLY
.build_mssql();

// Generates:
// SELECT c.*, recent_orders.*
// FROM customers c
// CROSS APPLY (
//   SELECT TOP 3 * FROM orders WHERE customer_id = c.id ORDER BY created_at DESC
// ) recent_orders

// OUTER APPLY for left join semantics
let query = LateralJoin::new(subquery, "alias")
    .outer() // OUTER APPLY
    .build_mssql();`;

  lockingExample = `use prax::advanced::{RowLock, RowLockBuilder};

// MSSQL table hints for locking
let locked_rows = client
    .user()
    .find_many()
    .where(user::status::equals("pending"))
    .lock(RowLock::for_update()) // WITH (UPDLOCK, ROWLOCK)
    .exec()
    .await?;

// Generates: SELECT * FROM users WITH (UPDLOCK, ROWLOCK) WHERE status = @P1

// HOLDLOCK for serializable reads
let query = RowLock::builder()
    .for_update()
    .hold_lock() // WITH (UPDLOCK, HOLDLOCK)
    .build_mssql();

// READPAST for skipping locked rows
let query = RowLock::builder()
    .for_update()
    .read_past() // WITH (UPDLOCK, READPAST)
    .build_mssql();`;

  sqlAgentExample = `use prax_migrate::procedure::{SqlAgentJob, JobStep, StepType, JobSchedule, Weekday};

// Create SQL Agent job for scheduled tasks
let job = SqlAgentJob::new("nightly_cleanup")
    .description("Cleanup old records nightly")
    .add_step(
        JobStep::new("delete_old_logs")
            .step_type(StepType::TSql)
            .database("mydb")
            .command("DELETE FROM logs WHERE created_at < DATEADD(day, -30, GETDATE())")
    )
    .add_step(
        JobStep::new("update_statistics")
            .step_type(StepType::TSql)
            .database("mydb")
            .command("EXEC sp_updatestats")
    )
    .schedule(
        JobSchedule::weekly()
            .on_days([Weekday::Monday, Weekday::Wednesday, Weekday::Friday])
            .at_time("02:00:00")
    )
    .enabled(true);

// Generates sp_add_job, sp_add_jobstep, sp_add_schedule calls`;

  alwaysOnExample = `use prax::replication::{ReplicaSetConfig, ReadPreference, ConnectionRouter};

// Configure AlwaysOn Availability Groups
let config = ReplicaSetConfig::builder()
    .primary("primary.sql.example.com:1433")
    .secondary("secondary1.sql.example.com:1433")
        .region("us-east")
        .priority(90)
    .secondary("secondary2.sql.example.com:1433")
        .region("us-west")
        .priority(80)
    .build();

// Route reads to secondaries
let router = ConnectionRouter::new(config);
let read_conn = router.route(QueryType::Read, ReadPreference::SecondaryPreferred)?;

// Check replica lag
let lag = prax::replication::lag_queries::check_lag_sql(DatabaseType::MSSQL);
// SELECT DATEDIFF(SECOND, last_commit_time, GETDATE()) FROM sys.dm_hadr_database_replica_states`;
}




