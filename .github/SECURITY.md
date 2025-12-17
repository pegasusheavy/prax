# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security vulnerabilities by emailing:

📧 **security@pegasusheavy.com**

### What to Include

Please include the following information in your report:

- Type of vulnerability (e.g., SQL injection, buffer overflow, etc.)
- Full paths of source file(s) related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Resolution Target**: Within 90 days (depending on severity)

### What to Expect

1. **Acknowledgment**: We'll acknowledge receipt of your report within 48 hours.
2. **Investigation**: We'll investigate and determine the impact and severity.
3. **Fix Development**: We'll develop and test a fix.
4. **Disclosure**: We'll coordinate disclosure with you.
5. **Credit**: We'll credit you in our security advisory (unless you prefer anonymity).

### Safe Harbor

We consider security research conducted consistent with this policy to be:

- Authorized concerning any applicable anti-hacking laws
- Authorized concerning any relevant anti-circumvention laws
- Exempt from restrictions in our Terms of Service that would interfere with conducting security research

We will not pursue civil action or initiate a complaint to law enforcement for accidental, good faith violations of this policy.

## Security Best Practices for Users

### Database Connections

```rust
// ✅ Good: Use environment variables for credentials
let database_url = std::env::var("DATABASE_URL")?;
let client = PraxClient::new(&database_url).await?;

// ❌ Bad: Hardcoded credentials
let client = PraxClient::new("postgres://user:password@localhost/db").await?;
```

### Query Parameters

```rust
// ✅ Good: Use parameterized queries (automatic with Prax)
let users = client
    .user()
    .find_many()
    .where_(user::email::equals(user_input))
    .exec()
    .await?;

// ❌ Bad: String interpolation in raw queries
let query = format!("SELECT * FROM users WHERE email = '{}'", user_input);
```

### Connection Pooling

```rust
// ✅ Good: Use connection pooling with reasonable limits
let client = PraxClient::builder()
    .max_connections(20)
    .connect(&database_url)
    .await?;
```

## Known Security Considerations

### SQL Injection

Prax uses parameterized queries internally, which protects against SQL injection. However, when using raw queries, ensure you use parameter binding:

```rust
// ✅ Safe: Parameters are bound
client.raw_query("SELECT * FROM users WHERE id = $1", &[&id]).await?;

// ❌ Unsafe: String interpolation
client.raw_query(&format!("SELECT * FROM users WHERE id = {}", id)).await?;
```

## Acknowledgments

We thank the following individuals for responsibly disclosing security issues:

<!-- Security researchers will be acknowledged here -->

*No security issues have been reported yet.*

