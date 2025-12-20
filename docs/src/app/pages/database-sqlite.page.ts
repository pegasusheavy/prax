import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-sqlite-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-sqlite.page.html',
})
export class DatabaseSqlitePage {
  configCode = `[database]
provider = "sqlite"
url = "file:./dev.db"

# Or in-memory
# url = "file::memory:"`;

  optionsCode = `use prax_sqlite::{SqlitePool, SqliteConfig, JournalMode};

let config = SqliteConfig::from_url("file:./app.db")?
    .journal_mode(JournalMode::Wal)
    .synchronous(SynchronousMode::Normal)
    .cache_size(10000);

let pool = SqlitePool::new(config).await?;`;
}
