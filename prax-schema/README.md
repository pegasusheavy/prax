# prax-schema

Schema definition language parser for Prax ORM.

## Overview

`prax-schema` provides a Prisma-like schema language parser for defining database models, relations, enums, and views.

## Features

- Custom `.prax` schema files with intuitive syntax
- AST types for models, fields, relations, enums, views
- Schema validation and semantic analysis
- Documentation comments with validation directives
- GraphQL and async-graphql integration support

## Example Schema

```prax
model User {
    id        Int      @id @auto
    email     String   @unique
    name      String?
    posts     Post[]
    createdAt DateTime @default(now())
}

model Post {
    id        Int      @id @auto
    title     String
    content   String?
    published Boolean  @default(false)
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int
}
```

## Usage

```rust
use prax_schema::parse_schema;

let schema = parse_schema(r#"
    model User {
        id    Int    @id @auto
        email String @unique
    }
"#)?;

for model in &schema.models {
    println!("Model: {}", model.name);
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

