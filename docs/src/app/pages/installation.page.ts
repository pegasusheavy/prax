import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-installation-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './installation.page.html',
})
export class InstallationPage {
  cliInstall = `cargo install prax-cli`;

  postgresInstall = `[dependencies]
prax = { version = "0.1", features = ["postgres"] }
prax-postgres = "0.1"
tokio = { version = "1", features = ["full"] }`;

  mysqlInstall = `[dependencies]
prax = { version = "0.1", features = ["mysql"] }
prax-mysql = "0.1"
tokio = { version = "1", features = ["full"] }`;

  sqliteInstall = `[dependencies]
prax = { version = "0.1", features = ["sqlite"] }
prax-sqlite = "0.1"
tokio = { version = "1", features = ["full"] }`;
}
