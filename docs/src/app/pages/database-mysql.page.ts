import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-mysql-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-mysql.page.html',
})
export class DatabaseMysqlPage {
  configCode = `[database]
provider = "mysql"
url = "mysql://user:password@localhost:3306/mydb"

[database.pool]
max_connections = 20`;

  connectionCode = `# Standard format
mysql://user:password@host:port/database

# With charset
mysql://user:password@host:port/database?charset=utf8mb4`;

  featuresCode = `model User {
    id        Int      @id @auto
    email     String   @unique @db.VarChar(255)
    bio       String?  @db.Text
    createdAt DateTime @default(now()) @db.Timestamp(6)
}`;
}
