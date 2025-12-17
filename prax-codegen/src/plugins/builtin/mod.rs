//! Built-in plugins for Prax code generation.

mod debug;
mod graphql;
mod json_schema;
mod serde_plugin;
mod validator;

pub use debug::DebugPlugin;
pub use graphql::GraphQLPlugin;
pub use json_schema::JsonSchemaPlugin;
pub use serde_plugin::SerdePlugin;
pub use validator::ValidatorPlugin;

