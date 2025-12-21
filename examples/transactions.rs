#![allow(dead_code, unused, clippy::type_complexity)]
//! # Transaction Examples
//!
//! This example demonstrates transaction handling in Prax:
//! - Basic transactions
//! - Transaction with configuration (isolation levels, timeouts)
//! - Savepoints
//! - Nested transactions
//! - Error handling and rollback
//!
//! ## Running this example
//!
//! ```bash
//! cargo run --example transactions
//! ```

use std::time::Duration;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// Mock types
#[derive(Debug, Clone)]
struct User {
    id: i32,
    email: String,
    balance: i32,
}

#[derive(Debug, Clone)]
struct Transfer {
    id: i32,
    from_user_id: i32,
    to_user_id: i32,
    amount: i32,
}

// Transaction isolation levels
#[derive(Debug, Clone, Copy)]
enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

// Transaction configuration
struct TransactionConfig {
    isolation: Option<IsolationLevel>,
    timeout: Option<Duration>,
    read_only: bool,
}

impl TransactionConfig {
    fn new() -> Self {
        Self {
            isolation: None,
            timeout: None,
            read_only: false,
        }
    }

    fn isolation(mut self, level: IsolationLevel) -> Self {
        self.isolation = Some(level);
        self
    }

    fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

// Mock client and transaction
struct MockClient;

impl MockClient {
    async fn transaction<F, R>(&self, callback: F) -> Result<R, BoxError>
    where
        F: FnOnce(
            MockTransaction,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<R, BoxError>> + Send>,
        >,
    {
        let tx = MockTransaction::new();
        callback(tx).await
    }

    fn transaction_with_config(&self, _config: TransactionConfig) -> TransactionBuilder {
        TransactionBuilder
    }
}

struct TransactionBuilder;

impl TransactionBuilder {
    async fn run<F, R>(self, callback: F) -> Result<R, BoxError>
    where
        F: FnOnce(
            MockTransaction,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<R, BoxError>> + Send>,
        >,
    {
        let tx = MockTransaction::new();
        callback(tx).await
    }
}

#[derive(Clone)]
struct MockTransaction {
    committed: bool,
}

impl MockTransaction {
    fn new() -> Self {
        Self { committed: false }
    }

    fn user(&self) -> TxUserQuery {
        TxUserQuery
    }

    fn transfer(&self) -> TxTransferQuery {
        TxTransferQuery
    }

    async fn savepoint(&self, _name: &str) -> Result<Savepoint, BoxError> {
        Ok(Savepoint {
            name: _name.to_string(),
        })
    }
}

struct TxUserQuery;

impl TxUserQuery {
    fn find_unique(self) -> TxUserFindUnique {
        TxUserFindUnique
    }

    fn update(self) -> TxUserUpdate {
        TxUserUpdate
    }

    fn create(self, _data: CreateUserData) -> TxUserCreate {
        TxUserCreate
    }
}

struct TxUserFindUnique;

impl TxUserFindUnique {
    #[allow(non_snake_case)]
    fn r#where(self, _filter: &str) -> Self {
        self
    }

    async fn exec(self) -> Result<Option<User>, BoxError> {
        Ok(Some(User {
            id: 1,
            email: "alice@example.com".to_string(),
            balance: 1000,
        }))
    }
}

struct TxUserUpdate;

impl TxUserUpdate {
    #[allow(non_snake_case)]
    fn r#where(self, _filter: &str) -> Self {
        self
    }

    fn data(self, _data: UpdateUserData) -> Self {
        self
    }

    async fn exec(self) -> Result<User, BoxError> {
        Ok(User {
            id: 1,
            email: "alice@example.com".to_string(),
            balance: 900,
        })
    }
}

struct TxUserCreate;

impl TxUserCreate {
    async fn exec(self) -> Result<User, BoxError> {
        Ok(User {
            id: 3,
            email: "new@example.com".to_string(),
            balance: 0,
        })
    }
}

struct TxTransferQuery;

impl TxTransferQuery {
    fn create(self, _data: CreateTransferData) -> TxTransferCreate {
        TxTransferCreate
    }
}

struct TxTransferCreate;

impl TxTransferCreate {
    async fn exec(self) -> Result<Transfer, BoxError> {
        Ok(Transfer {
            id: 1,
            from_user_id: 1,
            to_user_id: 2,
            amount: 100,
        })
    }
}

struct CreateUserData {
    email: String,
    balance: i32,
}

struct UpdateUserData {
    balance: Option<i32>,
}

struct CreateTransferData {
    from_user_id: i32,
    to_user_id: i32,
    amount: i32,
}

struct Savepoint {
    name: String,
}

impl Savepoint {
    async fn rollback(self) -> Result<(), BoxError> {
        println!("    Rolled back to savepoint: {}", self.name);
        Ok(())
    }

    async fn release(self) -> Result<(), BoxError> {
        println!("    Released savepoint: {}", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    println!("=== Prax Transaction Examples ===\n");

    let client = MockClient;

    // =========================================================================
    // BASIC TRANSACTION
    // =========================================================================
    println!("--- Basic Transaction ---");

    let result = client
        .transaction(|tx| {
            Box::pin(async move {
                // Create a user
                let user = tx
                    .user()
                    .create(CreateUserData {
                        email: "new@example.com".to_string(),
                        balance: 100,
                    })
                    .exec()
                    .await?;

                println!(
                    "Created user in transaction: {} (id: {})",
                    user.email, user.id
                );

                Ok(user)
            })
        })
        .await?;

    println!("Transaction committed. User id: {}", result.id);
    println!();

    // =========================================================================
    // TRANSACTION WITH CONFIGURATION
    // =========================================================================
    println!("--- Transaction with Configuration ---");

    let result = client
        .transaction_with_config(
            TransactionConfig::new()
                .isolation(IsolationLevel::Serializable)
                .timeout(Duration::from_secs(30)),
        )
        .run(|tx| {
            Box::pin(async move {
                let user = tx
                    .user()
                    .find_unique()
                    .r#where("id = 1")
                    .exec()
                    .await?
                    .ok_or("User not found")?;

                println!("Found user: {} with balance: {}", user.email, user.balance);

                Ok(user)
            })
        })
        .await?;

    println!(
        "Serializable transaction completed. Balance: {}",
        result.balance
    );
    println!();

    // =========================================================================
    // MONEY TRANSFER EXAMPLE (Multiple Operations)
    // =========================================================================
    println!("--- Money Transfer Transaction ---");

    let transfer_amount = 100;
    let from_user_id = 1;
    let to_user_id = 2;

    let transfer = client
        .transaction(|tx| {
            Box::pin(async move {
                // 1. Check sender balance
                let sender = tx
                    .user()
                    .find_unique()
                    .r#where(&format!("id = {}", from_user_id))
                    .exec()
                    .await?
                    .ok_or("Sender not found")?;

                if sender.balance < transfer_amount {
                    return Err("Insufficient balance".into());
                }

                // 2. Deduct from sender
                tx.user()
                    .update()
                    .r#where(&format!("id = {}", from_user_id))
                    .data(UpdateUserData {
                        balance: Some(sender.balance - transfer_amount),
                    })
                    .exec()
                    .await?;

                // 3. Add to recipient
                tx.user()
                    .update()
                    .r#where(&format!("id = {}", to_user_id))
                    .data(UpdateUserData {
                        balance: Some(1000 + transfer_amount), // Mock recipient balance
                    })
                    .exec()
                    .await?;

                // 4. Record the transfer
                let transfer = tx
                    .transfer()
                    .create(CreateTransferData {
                        from_user_id,
                        to_user_id,
                        amount: transfer_amount,
                    })
                    .exec()
                    .await?;

                println!(
                    "Transfer {} from user {} to user {}",
                    transfer_amount, from_user_id, to_user_id
                );

                Ok(transfer)
            })
        })
        .await?;

    println!("Transfer completed. Transfer id: {}", transfer.id);
    println!();

    // =========================================================================
    // SAVEPOINTS
    // =========================================================================
    println!("--- Savepoints ---");

    let _result = client
        .transaction(|tx| {
            Box::pin(async move {
                // First operation
                let user = tx
                    .user()
                    .create(CreateUserData {
                        email: "first@example.com".to_string(),
                        balance: 0,
                    })
                    .exec()
                    .await?;

                println!("  Created first user: {}", user.email);

                // Create a savepoint
                let savepoint = tx.savepoint("before_second_user").await?;

                // Try second operation (might fail)
                let second_result: Result<User, BoxError> = tx
                    .user()
                    .create(CreateUserData {
                        email: "second@example.com".to_string(),
                        balance: 0,
                    })
                    .exec()
                    .await;

                match second_result {
                    Ok(second_user) => {
                        println!("  Created second user: {}", second_user.email);
                        savepoint.release().await?;
                    }
                    Err(e) => {
                        println!("  Second user creation failed: {}", e);
                        savepoint.rollback().await?;
                        // First user is still created
                    }
                }

                Ok(user)
            })
        })
        .await?;

    println!("Transaction with savepoint completed");
    println!();

    // =========================================================================
    // READ-ONLY TRANSACTION
    // =========================================================================
    println!("--- Read-Only Transaction ---");

    let users = client
        .transaction_with_config(TransactionConfig::new().read_only())
        .run(|tx| {
            Box::pin(async move {
                let user = tx.user().find_unique().r#where("id = 1").exec().await?;

                Ok(user)
            })
        })
        .await?;

    println!(
        "Read-only transaction completed. Found user: {:?}",
        users.map(|u| u.email)
    );
    println!();

    // =========================================================================
    // ERROR HANDLING AND ROLLBACK
    // =========================================================================
    println!("--- Error Handling (Automatic Rollback) ---");

    let result: Result<User, BoxError> = client
        .transaction(|tx| {
            Box::pin(async move {
                // Create user (will be rolled back)
                let _user = tx
                    .user()
                    .create(CreateUserData {
                        email: "will-be-rolled-back@example.com".to_string(),
                        balance: 100,
                    })
                    .exec()
                    .await?;

                // Simulate an error
                Err("Something went wrong!".into())
            })
        })
        .await;

    match result {
        Ok(_) => println!("Transaction succeeded (unexpected)"),
        Err(e) => println!("Transaction rolled back due to error: {}", e),
    }
    println!();

    // =========================================================================
    // ISOLATION LEVELS EXPLANATION
    // =========================================================================
    println!("--- Isolation Levels Reference ---");
    println!(
        r#"
| Level            | Dirty Read | Non-Repeatable Read | Phantom Read |
|------------------|------------|---------------------|--------------|
| ReadUncommitted  | Possible   | Possible            | Possible     |
| ReadCommitted    | No         | Possible            | Possible     |
| RepeatableRead   | No         | No                  | Possible     |
| Serializable     | No         | No                  | No           |

Usage:
```rust
client
    .transaction_with_config(
        TransactionConfig::new()
            .isolation(IsolationLevel::Serializable)
            .timeout(Duration::from_secs(30))
    )
    .run(|tx| async move {{ ... }})
    .await?;
```
"#
    );

    println!("=== All examples completed successfully! ===");

    Ok(())
}
