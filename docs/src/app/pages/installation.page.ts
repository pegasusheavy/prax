import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-installation-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './installation.page.html',
})
export class InstallationPage {
  cliInstall = `cargo install prax-orm-cli`;

  postgresInstall = `[dependencies]
prax-orm = { version = "0.4", features = ["postgres"] }
prax-postgres = "0.4"
tokio = { version = "1", features = ["full"] }`;

  mysqlInstall = `[dependencies]
prax-orm = { version = "0.4", features = ["mysql"] }
prax-mysql = "0.4"
tokio = { version = "1", features = ["full"] }`;

  sqliteInstall = `[dependencies]
prax-orm = { version = "0.4", features = ["sqlite"] }
prax-sqlite = "0.4"
tokio = { version = "1", features = ["full"] }`;
}
