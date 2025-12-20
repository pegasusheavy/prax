import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-migrations-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-migrations.page.html',
})
export class DatabaseMigrationsPage {
  devWorkflow = `# Create a new migration based on schema changes
prax migrate dev --name add_user_roles

# This will:
# 1. Generate SQL migration file
# 2. Apply it to your dev database
# 3. Regenerate the Prax client`;

  prodWorkflow = `# Check migration status
prax migrate status

# Apply all pending migrations
prax migrate deploy

# The deploy command is safe:
# - Only applies pending migrations
# - Never modifies migration history
# - Fails fast on errors`;

  migrationFiles = `prax/
├── schema.prax          # Your schema definition
└── migrations/          # Migration directory
    ├── 20240101_init/
    │   └── migration.sql
    ├── 20240115_add_posts/
    │   └── migration.sql
    └── 20240201_add_user_roles/
        └── migration.sql

# Each migration.sql contains the raw SQL
# that was generated from your schema changes`;
}
