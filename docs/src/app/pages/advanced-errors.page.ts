import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-errors',
  standalone: true,
  imports: [CommonModule, CodeBlockComponent],
  templateUrl: './advanced-errors.page.html',
})
export class AdvancedErrorsPage {
  basicHandling = `use prax_query::{QueryError, ErrorCode};

// Handle specific error types
match result {
    Ok(user) => println!("Found user: {:?}", user),
    Err(e) if e.is_not_found() => {
        println!("User not found");
    }
    Err(e) if e.is_constraint_violation() => {
        println!("Constraint violated: {}", e.message);
    }
    Err(e) if e.is_timeout() => {
        println!("Query timed out");
    }
    Err(e) if e.is_retryable() => {
        println!("Transient error, retrying...");
        // Retry logic
    }
    Err(e) => {
        println!("Error [{}]: {}", e.code, e.message);
    }
}`;

  errorCodes = `// Error codes follow the Pxxxx format

// Query Errors (P1xxx)
ErrorCode::RecordNotFound       // P1001
ErrorCode::NotUnique            // P1002
ErrorCode::InvalidFilter        // P1003
ErrorCode::RequiredFieldMissing // P1005

// Constraint Errors (P2xxx)
ErrorCode::UniqueConstraint     // P2001
ErrorCode::ForeignKeyConstraint // P2002
ErrorCode::NotNullConstraint    // P2004

// Connection Errors (P3xxx)
ErrorCode::ConnectionFailed     // P3001
ErrorCode::PoolExhausted        // P3002
ErrorCode::AuthenticationFailed // P3004

// Transaction Errors (P4xxx)
ErrorCode::Deadlock             // P4002
ErrorCode::SerializationFailure // P4003

// Query Execution (P5xxx)
ErrorCode::QueryTimeout         // P5001
ErrorCode::SqlSyntax            // P5002`;

  actionableErrors = `use prax_query::QueryError;

// Errors include actionable suggestions
let err = QueryError::unique_violation("User", "email");
println!("{}", err.display_full());

// Output:
// Error [P2001]: Unique constraint violated on User.email
//   → Model: User
//   → Field: email
//
// Suggestions:
//   1. A record with this email already exists
//   2. Use upsert() to update if exists, create if not
//      \`\`\`
//      client.user().upsert()
//        .where(user::email::equals(value))
//        .create(...)
//        .update(...)
//        .exec().await
//      \`\`\`
//
// More info: https://prax.rs/docs/errors/P2001`;

  coloredOutput = `use prax_query::QueryError;

// Display with ANSI colors for terminal
let err = QueryError::not_found("User")
    .with_context("Finding user by email")
    .with_suggestion("Verify the user exists");

// Colored output for CLI tools
eprintln!("{}", err.display_colored());

// Plain text for logging
log::error!("{}", err.display_full());`;

  customContext = `use prax_query::{QueryError, ErrorCode, Suggestion};

// Add context to errors
let err = QueryError::not_found("User")
    .with_context("Authenticating user login")
    .with_model("User")
    .with_field("email")
    .with_suggestion("Check the email address is correct")
    .with_code_suggestion(
        "Register the user first",
        "client.user().create(data! { email, password }).exec().await"
    )
    .with_help("Users must be registered before they can log in");

// Access error details
println!("Code: {}", err.code);           // P1001
println!("Message: {}", err.message);
println!("Model: {:?}", err.context.model);
println!("Docs: {}", err.docs_url());`;

  errorChecks = `use prax_query::QueryError;

// Boolean checks for error categories
fn handle_error(err: &QueryError) {
    // Record errors
    if err.is_not_found() { /* ... */ }

    // Constraint violations
    if err.is_constraint_violation() { /* ... */ }

    // Timeout errors (connection or query)
    if err.is_timeout() { /* ... */ }

    // Connection-related errors
    if err.is_connection_error() { /* ... */ }

    // Can this error be retried?
    if err.is_retryable() {
        // Safe to retry: timeouts, deadlocks, pool exhaustion
    }
}`;

  errorMacro = `use prax_query::{query_error, ErrorCode};

// Create custom errors with the macro
let err = query_error!(
    ErrorCode::InvalidParameter,
    "Email format is invalid",
    with_field = "email",
    with_suggestion = "Use a valid email like user@example.com",
    with_help = "Email must contain @ and a domain"
);`;
}

