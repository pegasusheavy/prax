# prax-mssql

Microsoft SQL Server query engine for Prax ORM.

## Overview

`prax-mssql` provides an async SQL Server backend using `tiberius` with `bb8` connection pooling.

## Features

- Async query execution with Tokio
- Connection pooling via bb8
- Transaction support with savepoints
- Row-Level Security (RLS) policy generation
- Session context management for RLS
- SQL Server authentication and Windows Authentication support
- TLS/SSL encryption support

## Usage

```rust
use prax_mssql::{MssqlPool, MssqlEngine};

let pool = MssqlPool::builder()
    .host("localhost")
    .database("mydb")
    .username("sa")
    .password("YourPassword123!")
    .max_connections(10)
    .trust_cert(true)  // For development
    .build()
    .await?;

// Create engine for Prax queries
let engine = MssqlEngine::new(pool);
```

## Connection Strings

Both URL-style and ADO.NET-style connection strings are supported:

```rust
// URL-style
let config = MssqlConfig::from_connection_string(
    "mssql://sa:Password@localhost:1433/mydb?encrypt=true"
)?;

// ADO.NET-style
let config = MssqlConfig::from_connection_string(
    "Server=localhost;Database=mydb;User Id=sa;Password=Password;"
)?;
```

## Row-Level Security

Generate SQL Server security policies from Prax schema definitions:

```rust
use prax_mssql::rls::{SecurityPolicyGenerator, RlsContextBuilder};

// Generate security policy from Prax policy
let generator = SecurityPolicyGenerator::new("Security");
let security_policy = generator.generate(&policy, "dbo.Users", "UserId")?;

// Apply the policy
conn.batch_execute(&security_policy.to_sql()).await?;

// Set session context for RLS
conn.set_session_context("UserId", "123").await?;
```

### Policy Generation

The generator converts Prax policies to SQL Server security policies:

```sql
-- Generated schema
CREATE SCHEMA Security;
GO

-- Generated predicate function
CREATE FUNCTION Security.fn_UserFilter_predicate(@UserId INT)
    RETURNS TABLE
WITH SCHEMABINDING
AS
    RETURN SELECT 1 AS fn_securitypredicate_result
    WHERE @UserId = CAST(SESSION_CONTEXT(N'UserId') AS INT);
GO

-- Generated security policy
CREATE SECURITY POLICY Security.UserFilter
ADD FILTER PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users,
ADD BLOCK PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users AFTER INSERT
WITH (STATE = ON);
```

### Session Context

Set up RLS context for a connection:

```rust
use prax_mssql::rls::RlsContextBuilder;

let sql = RlsContextBuilder::new()
    .user_id("123")
    .tenant_id("456")
    .custom("Role", "Admin")
    .to_sql();

conn.batch_execute(&sql).await?;
```

## Configuration

```rust
use prax_mssql::{MssqlConfig, EncryptionMode};
use std::time::Duration;

let config = MssqlConfig::builder()
    .host("myserver.database.windows.net")
    .port(1433)
    .database("mydb")
    .username("myuser")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .connect_timeout(Duration::from_secs(30))
    .application_name("my-app")
    .build()?;
```

## Azure SQL Database

For Azure SQL Database, use these recommended settings:

```rust
let pool = MssqlPool::builder()
    .host("myserver.database.windows.net")
    .database("mydb")
    .username("myuser@myserver")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .build()
    .await?;
```

## SQL Dialect Conversion

The engine automatically converts PostgreSQL-style queries to SQL Server:

- Parameter placeholders: `$1` → `@P1`
- Boolean literals: `true`/`false` → `1`/`0`
- LIMIT/OFFSET → `OFFSET FETCH`
- RETURNING → `OUTPUT INSERTED.*`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.





Microsoft SQL Server query engine for Prax ORM.

## Overview

`prax-mssql` provides an async SQL Server backend using `tiberius` with `bb8` connection pooling.

## Features

- Async query execution with Tokio
- Connection pooling via bb8
- Transaction support with savepoints
- Row-Level Security (RLS) policy generation
- Session context management for RLS
- SQL Server authentication and Windows Authentication support
- TLS/SSL encryption support

## Usage

```rust
use prax_mssql::{MssqlPool, MssqlEngine};

let pool = MssqlPool::builder()
    .host("localhost")
    .database("mydb")
    .username("sa")
    .password("YourPassword123!")
    .max_connections(10)
    .trust_cert(true)  // For development
    .build()
    .await?;

// Create engine for Prax queries
let engine = MssqlEngine::new(pool);
```

## Connection Strings

Both URL-style and ADO.NET-style connection strings are supported:

```rust
// URL-style
let config = MssqlConfig::from_connection_string(
    "mssql://sa:Password@localhost:1433/mydb?encrypt=true"
)?;

// ADO.NET-style
let config = MssqlConfig::from_connection_string(
    "Server=localhost;Database=mydb;User Id=sa;Password=Password;"
)?;
```

## Row-Level Security

Generate SQL Server security policies from Prax schema definitions:

```rust
use prax_mssql::rls::{SecurityPolicyGenerator, RlsContextBuilder};

// Generate security policy from Prax policy
let generator = SecurityPolicyGenerator::new("Security");
let security_policy = generator.generate(&policy, "dbo.Users", "UserId")?;

// Apply the policy
conn.batch_execute(&security_policy.to_sql()).await?;

// Set session context for RLS
conn.set_session_context("UserId", "123").await?;
```

### Policy Generation

The generator converts Prax policies to SQL Server security policies:

```sql
-- Generated schema
CREATE SCHEMA Security;
GO

-- Generated predicate function
CREATE FUNCTION Security.fn_UserFilter_predicate(@UserId INT)
    RETURNS TABLE
WITH SCHEMABINDING
AS
    RETURN SELECT 1 AS fn_securitypredicate_result
    WHERE @UserId = CAST(SESSION_CONTEXT(N'UserId') AS INT);
GO

-- Generated security policy
CREATE SECURITY POLICY Security.UserFilter
ADD FILTER PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users,
ADD BLOCK PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users AFTER INSERT
WITH (STATE = ON);
```

### Session Context

Set up RLS context for a connection:

```rust
use prax_mssql::rls::RlsContextBuilder;

let sql = RlsContextBuilder::new()
    .user_id("123")
    .tenant_id("456")
    .custom("Role", "Admin")
    .to_sql();

conn.batch_execute(&sql).await?;
```

## Configuration

```rust
use prax_mssql::{MssqlConfig, EncryptionMode};
use std::time::Duration;

let config = MssqlConfig::builder()
    .host("myserver.database.windows.net")
    .port(1433)
    .database("mydb")
    .username("myuser")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .connect_timeout(Duration::from_secs(30))
    .application_name("my-app")
    .build()?;
```

## Azure SQL Database

For Azure SQL Database, use these recommended settings:

```rust
let pool = MssqlPool::builder()
    .host("myserver.database.windows.net")
    .database("mydb")
    .username("myuser@myserver")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .build()
    .await?;
```

## SQL Dialect Conversion

The engine automatically converts PostgreSQL-style queries to SQL Server:

- Parameter placeholders: `$1` → `@P1`
- Boolean literals: `true`/`false` → `1`/`0`
- LIMIT/OFFSET → `OFFSET FETCH`
- RETURNING → `OUTPUT INSERTED.*`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.






Microsoft SQL Server query engine for Prax ORM.

## Overview

`prax-mssql` provides an async SQL Server backend using `tiberius` with `bb8` connection pooling.

## Features

- Async query execution with Tokio
- Connection pooling via bb8
- Transaction support with savepoints
- Row-Level Security (RLS) policy generation
- Session context management for RLS
- SQL Server authentication and Windows Authentication support
- TLS/SSL encryption support

## Usage

```rust
use prax_mssql::{MssqlPool, MssqlEngine};

let pool = MssqlPool::builder()
    .host("localhost")
    .database("mydb")
    .username("sa")
    .password("YourPassword123!")
    .max_connections(10)
    .trust_cert(true)  // For development
    .build()
    .await?;

// Create engine for Prax queries
let engine = MssqlEngine::new(pool);
```

## Connection Strings

Both URL-style and ADO.NET-style connection strings are supported:

```rust
// URL-style
let config = MssqlConfig::from_connection_string(
    "mssql://sa:Password@localhost:1433/mydb?encrypt=true"
)?;

// ADO.NET-style
let config = MssqlConfig::from_connection_string(
    "Server=localhost;Database=mydb;User Id=sa;Password=Password;"
)?;
```

## Row-Level Security

Generate SQL Server security policies from Prax schema definitions:

```rust
use prax_mssql::rls::{SecurityPolicyGenerator, RlsContextBuilder};

// Generate security policy from Prax policy
let generator = SecurityPolicyGenerator::new("Security");
let security_policy = generator.generate(&policy, "dbo.Users", "UserId")?;

// Apply the policy
conn.batch_execute(&security_policy.to_sql()).await?;

// Set session context for RLS
conn.set_session_context("UserId", "123").await?;
```

### Policy Generation

The generator converts Prax policies to SQL Server security policies:

```sql
-- Generated schema
CREATE SCHEMA Security;
GO

-- Generated predicate function
CREATE FUNCTION Security.fn_UserFilter_predicate(@UserId INT)
    RETURNS TABLE
WITH SCHEMABINDING
AS
    RETURN SELECT 1 AS fn_securitypredicate_result
    WHERE @UserId = CAST(SESSION_CONTEXT(N'UserId') AS INT);
GO

-- Generated security policy
CREATE SECURITY POLICY Security.UserFilter
ADD FILTER PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users,
ADD BLOCK PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users AFTER INSERT
WITH (STATE = ON);
```

### Session Context

Set up RLS context for a connection:

```rust
use prax_mssql::rls::RlsContextBuilder;

let sql = RlsContextBuilder::new()
    .user_id("123")
    .tenant_id("456")
    .custom("Role", "Admin")
    .to_sql();

conn.batch_execute(&sql).await?;
```

## Configuration

```rust
use prax_mssql::{MssqlConfig, EncryptionMode};
use std::time::Duration;

let config = MssqlConfig::builder()
    .host("myserver.database.windows.net")
    .port(1433)
    .database("mydb")
    .username("myuser")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .connect_timeout(Duration::from_secs(30))
    .application_name("my-app")
    .build()?;
```

## Azure SQL Database

For Azure SQL Database, use these recommended settings:

```rust
let pool = MssqlPool::builder()
    .host("myserver.database.windows.net")
    .database("mydb")
    .username("myuser@myserver")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .build()
    .await?;
```

## SQL Dialect Conversion

The engine automatically converts PostgreSQL-style queries to SQL Server:

- Parameter placeholders: `$1` → `@P1`
- Boolean literals: `true`/`false` → `1`/`0`
- LIMIT/OFFSET → `OFFSET FETCH`
- RETURNING → `OUTPUT INSERTED.*`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.





Microsoft SQL Server query engine for Prax ORM.

## Overview

`prax-mssql` provides an async SQL Server backend using `tiberius` with `bb8` connection pooling.

## Features

- Async query execution with Tokio
- Connection pooling via bb8
- Transaction support with savepoints
- Row-Level Security (RLS) policy generation
- Session context management for RLS
- SQL Server authentication and Windows Authentication support
- TLS/SSL encryption support

## Usage

```rust
use prax_mssql::{MssqlPool, MssqlEngine};

let pool = MssqlPool::builder()
    .host("localhost")
    .database("mydb")
    .username("sa")
    .password("YourPassword123!")
    .max_connections(10)
    .trust_cert(true)  // For development
    .build()
    .await?;

// Create engine for Prax queries
let engine = MssqlEngine::new(pool);
```

## Connection Strings

Both URL-style and ADO.NET-style connection strings are supported:

```rust
// URL-style
let config = MssqlConfig::from_connection_string(
    "mssql://sa:Password@localhost:1433/mydb?encrypt=true"
)?;

// ADO.NET-style
let config = MssqlConfig::from_connection_string(
    "Server=localhost;Database=mydb;User Id=sa;Password=Password;"
)?;
```

## Row-Level Security

Generate SQL Server security policies from Prax schema definitions:

```rust
use prax_mssql::rls::{SecurityPolicyGenerator, RlsContextBuilder};

// Generate security policy from Prax policy
let generator = SecurityPolicyGenerator::new("Security");
let security_policy = generator.generate(&policy, "dbo.Users", "UserId")?;

// Apply the policy
conn.batch_execute(&security_policy.to_sql()).await?;

// Set session context for RLS
conn.set_session_context("UserId", "123").await?;
```

### Policy Generation

The generator converts Prax policies to SQL Server security policies:

```sql
-- Generated schema
CREATE SCHEMA Security;
GO

-- Generated predicate function
CREATE FUNCTION Security.fn_UserFilter_predicate(@UserId INT)
    RETURNS TABLE
WITH SCHEMABINDING
AS
    RETURN SELECT 1 AS fn_securitypredicate_result
    WHERE @UserId = CAST(SESSION_CONTEXT(N'UserId') AS INT);
GO

-- Generated security policy
CREATE SECURITY POLICY Security.UserFilter
ADD FILTER PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users,
ADD BLOCK PREDICATE Security.fn_UserFilter_predicate(UserId) ON dbo.Users AFTER INSERT
WITH (STATE = ON);
```

### Session Context

Set up RLS context for a connection:

```rust
use prax_mssql::rls::RlsContextBuilder;

let sql = RlsContextBuilder::new()
    .user_id("123")
    .tenant_id("456")
    .custom("Role", "Admin")
    .to_sql();

conn.batch_execute(&sql).await?;
```

## Configuration

```rust
use prax_mssql::{MssqlConfig, EncryptionMode};
use std::time::Duration;

let config = MssqlConfig::builder()
    .host("myserver.database.windows.net")
    .port(1433)
    .database("mydb")
    .username("myuser")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .connect_timeout(Duration::from_secs(30))
    .application_name("my-app")
    .build()?;
```

## Azure SQL Database

For Azure SQL Database, use these recommended settings:

```rust
let pool = MssqlPool::builder()
    .host("myserver.database.windows.net")
    .database("mydb")
    .username("myuser@myserver")
    .password("mypassword")
    .encryption(EncryptionMode::Required)
    .trust_cert(false)
    .build()
    .await?;
```

## SQL Dialect Conversion

The engine automatically converts PostgreSQL-style queries to SQL Server:

- Parameter placeholders: `$1` → `@P1`
- Boolean literals: `true`/`false` → `1`/`0`
- LIMIT/OFFSET → `OFFSET FETCH`
- RETURNING → `OUTPUT INSERTED.*`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.








