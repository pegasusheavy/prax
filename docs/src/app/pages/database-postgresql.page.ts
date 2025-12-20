import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-postgresql-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-postgresql.page.html',
})
export class DatabasePostgresqlPage {
  configCode = `[database]
provider = "postgresql"
url = "postgres://user:password@localhost:5432/mydb"

# Connection pool settings
[database.pool]
max_connections = 20
min_connections = 5
connect_timeout = 30
idle_timeout = 600`;

  connectionCode = `# Standard format
postgres://user:password@host:port/database

# With SSL
postgres://user:password@host:port/database?sslmode=require

# With schema
postgres://user:password@host:port/database?schema=myschema

# Environment variable
DATABASE_URL=postgres://...`;

  poolCode = `use prax_postgres::{PostgresPool, PostgresConfig};

// Create pool with custom config
let config = PostgresConfig::from_url("postgres://localhost/mydb")?
    .max_connections(20)
    .min_connections(5)
    .connect_timeout(Duration::from_secs(30));

let pool = PostgresPool::new(config).await?;

// Use the pool
let client = PraxClient::with_pool(pool).await?;`;

  typesCode = `model Document {
    id       Int      @id @auto
    data     Json     // JSONB
    metadata Json?

    // Array types
    tags     String[]
    scores   Int[]

    // UUID
    uuid     String   @default(uuid())

    // Full-text search
    @@index([data], type: GIN)
}`;
}
