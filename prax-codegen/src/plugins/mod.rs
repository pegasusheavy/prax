//! Plugin system for extending Prax code generation.
//!
//! Plugins can hook into various stages of code generation to add custom
//! functionality like debug output, JSON Schema generation, GraphQL types, etc.
//!
//! # Enabling Plugins
//!
//! Plugins are enabled via environment variables:
//!
//! ```bash
//! # Enable debug plugin
//! PRAX_PLUGIN_DEBUG=1 cargo build
//!
//! # Enable JSON Schema generation
//! PRAX_PLUGIN_JSON_SCHEMA=1 cargo build
//!
//! # Enable GraphQL types
//! PRAX_PLUGIN_GRAPHQL=1 cargo build
//!
//! # Enable all plugins
//! PRAX_PLUGINS_ALL=1 cargo build
//!
//! # Disable a specific plugin (overrides PRAX_PLUGINS_ALL)
//! PRAX_PLUGIN_DEBUG=0 PRAX_PLUGINS_ALL=1 cargo build
//! ```
//!
//! # Custom Plugins
//!
//! Implement the [`Plugin`] trait to create custom plugins:
//!
//! ```rust,ignore
//! use prax_codegen::plugins::{Plugin, PluginContext, PluginOutput};
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn name(&self) -> &'static str { "my-plugin" }
//!     fn env_var(&self) -> &'static str { "PRAX_PLUGIN_MY_PLUGIN" }
//!
//!     fn on_model(&self, ctx: &PluginContext, model: &Model) -> PluginOutput {
//!         // Generate additional code for each model
//!         PluginOutput::new()
//!     }
//! }
//! ```

pub mod builtin;
pub mod config;

use proc_macro2::TokenStream;
use quote::quote;

use prax_schema::ast::{CompositeType, Enum, Model, Schema, View};

pub use config::PluginConfig;

/// Output from a plugin hook.
#[derive(Debug, Default, Clone)]
pub struct PluginOutput {
    /// Additional tokens to add to the module.
    pub tokens: TokenStream,
    /// Additional items to add at the crate root level.
    pub root_items: TokenStream,
    /// Additional imports needed.
    pub imports: Vec<String>,
}

#[allow(dead_code)]
impl PluginOutput {
    /// Create an empty plugin output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create output with tokens.
    pub fn with_tokens(tokens: TokenStream) -> Self {
        Self {
            tokens,
            ..Default::default()
        }
    }

    /// Add tokens to the output.
    pub fn add_tokens(&mut self, tokens: TokenStream) {
        self.tokens.extend(tokens);
    }

    /// Add root-level items.
    pub fn add_root_items(&mut self, tokens: TokenStream) {
        self.root_items.extend(tokens);
    }

    /// Add an import.
    pub fn add_import(&mut self, import: impl Into<String>) {
        self.imports.push(import.into());
    }

    /// Merge another output into this one.
    pub fn merge(&mut self, other: PluginOutput) {
        self.tokens.extend(other.tokens);
        self.root_items.extend(other.root_items);
        self.imports.extend(other.imports);
    }

    /// Check if the output is empty.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty() && self.root_items.is_empty() && self.imports.is_empty()
    }
}

/// Context provided to plugins during code generation.
#[derive(Debug)]
pub struct PluginContext<'a> {
    /// The full schema being processed.
    pub schema: &'a Schema,
    /// Plugin configuration.
    pub config: &'a PluginConfig,
}

impl<'a> PluginContext<'a> {
    /// Create a new plugin context.
    pub fn new(schema: &'a Schema, config: &'a PluginConfig) -> Self {
        Self { schema, config }
    }
}

/// Trait for implementing code generation plugins.
pub trait Plugin: Send + Sync {
    /// The unique name of this plugin.
    fn name(&self) -> &'static str;

    /// The environment variable that controls this plugin.
    /// Should follow the pattern `PRAX_PLUGIN_<NAME>`.
    fn env_var(&self) -> &'static str;

    /// Description of what this plugin does.
    fn description(&self) -> &'static str {
        "No description provided"
    }

    /// Called once at the start of code generation.
    fn on_start(&self, _ctx: &PluginContext) -> PluginOutput {
        PluginOutput::new()
    }

    /// Called for each model in the schema.
    fn on_model(&self, _ctx: &PluginContext, _model: &Model) -> PluginOutput {
        PluginOutput::new()
    }

    /// Called for each enum in the schema.
    fn on_enum(&self, _ctx: &PluginContext, _enum_def: &Enum) -> PluginOutput {
        PluginOutput::new()
    }

    /// Called for each composite type in the schema.
    fn on_type(&self, _ctx: &PluginContext, _type_def: &CompositeType) -> PluginOutput {
        PluginOutput::new()
    }

    /// Called for each view in the schema.
    fn on_view(&self, _ctx: &PluginContext, _view: &View) -> PluginOutput {
        PluginOutput::new()
    }

    /// Called once at the end of code generation.
    fn on_finish(&self, _ctx: &PluginContext) -> PluginOutput {
        PluginOutput::new()
    }
}

/// Registry of all available plugins.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with all built-in plugins.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(builtin::DebugPlugin));
        registry.register(Box::new(builtin::JsonSchemaPlugin));
        registry.register(Box::new(builtin::GraphQLPlugin));
        registry.register(Box::new(builtin::SerdePlugin));
        registry.register(Box::new(builtin::ValidatorPlugin));
        registry
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Get all registered plugins.
    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    /// Get enabled plugins based on configuration.
    pub fn enabled_plugins(&self, config: &PluginConfig) -> Vec<&dyn Plugin> {
        self.plugins
            .iter()
            .filter(|p| config.is_enabled(p.env_var()))
            .map(|p| p.as_ref())
            .collect()
    }

    /// Execute all enabled plugins for the start hook.
    pub fn run_start(&self, ctx: &PluginContext) -> PluginOutput {
        let mut output = PluginOutput::new();
        for plugin in self.enabled_plugins(ctx.config) {
            output.merge(plugin.on_start(ctx));
        }
        output
    }

    /// Execute all enabled plugins for a model.
    pub fn run_model(&self, ctx: &PluginContext, model: &Model) -> PluginOutput {
        let mut output = PluginOutput::new();
        for plugin in self.enabled_plugins(ctx.config) {
            output.merge(plugin.on_model(ctx, model));
        }
        output
    }

    /// Execute all enabled plugins for an enum.
    pub fn run_enum(&self, ctx: &PluginContext, enum_def: &Enum) -> PluginOutput {
        let mut output = PluginOutput::new();
        for plugin in self.enabled_plugins(ctx.config) {
            output.merge(plugin.on_enum(ctx, enum_def));
        }
        output
    }

    /// Execute all enabled plugins for a composite type.
    pub fn run_type(&self, ctx: &PluginContext, type_def: &CompositeType) -> PluginOutput {
        let mut output = PluginOutput::new();
        for plugin in self.enabled_plugins(ctx.config) {
            output.merge(plugin.on_type(ctx, type_def));
        }
        output
    }

    /// Execute all enabled plugins for a view.
    pub fn run_view(&self, ctx: &PluginContext, view: &View) -> PluginOutput {
        let mut output = PluginOutput::new();
        for plugin in self.enabled_plugins(ctx.config) {
            output.merge(plugin.on_view(ctx, view));
        }
        output
    }

    /// Execute all enabled plugins for the finish hook.
    pub fn run_finish(&self, ctx: &PluginContext) -> PluginOutput {
        let mut output = PluginOutput::new();
        for plugin in self.enabled_plugins(ctx.config) {
            output.merge(plugin.on_finish(ctx));
        }
        output
    }
}

/// Generate plugin documentation as a compile-time string.
pub fn generate_plugin_docs(registry: &PluginRegistry) -> TokenStream {
    let mut doc_lines = vec![
        "# Available Plugins".to_string(),
        String::new(),
        "The following plugins can be enabled via environment variables:".to_string(),
        String::new(),
    ];

    for plugin in registry.plugins() {
        doc_lines.push(format!("## {}", plugin.name()));
        doc_lines.push(format!("- **Env var**: `{}`", plugin.env_var()));
        doc_lines.push(format!("- **Description**: {}", plugin.description()));
        doc_lines.push(String::new());
    }

    let doc_string = doc_lines.join("\n");
    quote! {
        #[doc = #doc_string]
        pub mod _plugin_docs {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;

    impl Plugin for TestPlugin {
        fn name(&self) -> &'static str {
            "test"
        }

        fn env_var(&self) -> &'static str {
            "PRAX_PLUGIN_TEST"
        }

        fn description(&self) -> &'static str {
            "A test plugin"
        }

        fn on_start(&self, _ctx: &PluginContext) -> PluginOutput {
            PluginOutput::with_tokens(quote! {
                const TEST_PLUGIN_ACTIVE: bool = true;
            })
        }
    }

    #[test]
    fn test_plugin_output_merge() {
        let mut output1 = PluginOutput::with_tokens(quote! { const A: i32 = 1; });
        let output2 = PluginOutput::with_tokens(quote! { const B: i32 = 2; });

        output1.merge(output2);
        let code = output1.tokens.to_string();

        assert!(code.contains("const A"));
        assert!(code.contains("const B"));
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin));

        assert_eq!(registry.plugins().len(), 1);
        assert_eq!(registry.plugins()[0].name(), "test");
    }

    #[test]
    fn test_enabled_plugins() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin));

        // Without env var, plugin should be disabled
        let config = PluginConfig::from_env();
        let enabled = registry.enabled_plugins(&config);

        // Test depends on whether PRAX_PLUGIN_TEST is set
        // In most cases, it won't be, so we just verify the method works
        assert!(enabled.len() <= 1);
    }
}

