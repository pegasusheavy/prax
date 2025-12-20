# prax-codegen

Procedural macro code generation for Prax ORM.

## Overview

`prax-codegen` provides proc-macros for generating type-safe Rust code from Prax schema definitions.

## Features

- `#[derive(Model)]` macro for struct generation
- `prax_schema!` macro for schema-based code generation
- Plugin system for extensible code generation
- Built-in plugins: debug, JSON Schema, GraphQL, serde, validator

## Usage

```rust
use prax_codegen::prax_schema;

prax_schema!("prax/schema.prax");

// Generated code includes:
// - User struct with all fields
// - user module with filter functions
// - Type-safe query builders
```

## Plugins

Enable plugins in your schema:

```prax
generator client {
    provider = "prax-rust"
    plugins  = ["graphql", "validator", "json-schema"]
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

