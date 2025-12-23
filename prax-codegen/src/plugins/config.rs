//! Plugin configuration via environment variables and prax.toml.

use std::collections::HashMap;
use std::env;

use prax_schema::ModelStyle;

/// Environment variable prefix for plugins.
pub const PLUGIN_PREFIX: &str = "PRAX_PLUGIN_";

/// Environment variable to enable all plugins.
pub const PLUGINS_ALL: &str = "PRAX_PLUGINS_ALL";

/// Environment variable to list enabled plugins (comma-separated).
pub const PLUGINS_ENABLED: &str = "PRAX_PLUGINS";

/// Plugin configuration loaded from environment variables and prax.toml.
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// Whether all plugins are enabled by default.
    all_enabled: bool,
    /// Per-plugin overrides (true = enabled, false = disabled).
    overrides: HashMap<String, bool>,
    /// List of explicitly enabled plugin names.
    enabled_list: Vec<String>,
    /// Model generation style from prax.toml configuration.
    model_style: ModelStyle,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[allow(dead_code)]
impl PluginConfig {
    /// Create a new empty config (all plugins disabled).
    pub fn new() -> Self {
        Self {
            all_enabled: false,
            overrides: HashMap::new(),
            enabled_list: Vec::new(),
            model_style: ModelStyle::default(),
        }
    }

    /// Create a config with all plugins enabled.
    pub fn all_enabled() -> Self {
        Self {
            all_enabled: true,
            overrides: HashMap::new(),
            enabled_list: Vec::new(),
            model_style: ModelStyle::default(),
        }
    }

    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `PRAX_PLUGINS_ALL=1` - Enable all plugins
    /// - `PRAX_PLUGINS=debug,json_schema` - Enable specific plugins by name
    /// - `PRAX_PLUGIN_DEBUG=1` - Enable a specific plugin
    /// - `PRAX_PLUGIN_DEBUG=0` - Disable a specific plugin (overrides ALL)
    pub fn from_env() -> Self {
        let mut config = Self::new();

        // Check for PRAX_PLUGINS_ALL
        if let Ok(val) = env::var(PLUGINS_ALL) {
            config.all_enabled = is_truthy(&val);
        }

        // Check for PRAX_PLUGINS (comma-separated list)
        if let Ok(val) = env::var(PLUGINS_ENABLED) {
            config.enabled_list = val
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // Scan for individual PRAX_PLUGIN_* variables
        for (key, value) in env::vars() {
            if let Some(plugin_name) = key.strip_prefix(PLUGIN_PREFIX) {
                let plugin_name = plugin_name.to_lowercase();
                // Skip the special "all" and "s" (from PRAX_PLUGINS)
                if plugin_name == "all" || plugin_name == "s" {
                    continue;
                }
                config.overrides.insert(key, is_truthy(&value));
            }
        }

        config
    }

    /// Check if a plugin is enabled by its environment variable name.
    pub fn is_enabled(&self, env_var: &str) -> bool {
        // First check for explicit override
        if let Some(&enabled) = self.overrides.get(env_var) {
            return enabled;
        }

        // Then check the enabled list
        if !self.enabled_list.is_empty() {
            // Extract plugin name from env var (PRAX_PLUGIN_DEBUG -> debug)
            if let Some(name) = env_var.strip_prefix(PLUGIN_PREFIX) {
                let name_lower = name.to_lowercase();
                if self.enabled_list.contains(&name_lower) {
                    return true;
                }
            }
        }

        // Finally, check if all plugins are enabled
        self.all_enabled
    }

    /// Check if a plugin is enabled by its name.
    pub fn is_enabled_by_name(&self, name: &str) -> bool {
        let env_var = format!("{}{}", PLUGIN_PREFIX, name.to_uppercase());
        self.is_enabled(&env_var)
    }

    /// Enable a plugin by its environment variable name.
    pub fn enable(&mut self, env_var: &str) {
        self.overrides.insert(env_var.to_string(), true);
    }

    /// Disable a plugin by its environment variable name.
    pub fn disable(&mut self, env_var: &str) {
        self.overrides.insert(env_var.to_string(), false);
    }

    /// Enable a plugin by name.
    pub fn enable_by_name(&mut self, name: &str) {
        let env_var = format!("{}{}", PLUGIN_PREFIX, name.to_uppercase());
        self.enable(&env_var);
    }

    /// Disable a plugin by name.
    pub fn disable_by_name(&mut self, name: &str) {
        let env_var = format!("{}{}", PLUGIN_PREFIX, name.to_uppercase());
        self.disable(&env_var);
    }

    /// Get list of all overridden plugins.
    pub fn overrides(&self) -> &HashMap<String, bool> {
        &self.overrides
    }

    /// Check if all plugins are enabled by default.
    pub fn all_plugins_enabled(&self) -> bool {
        self.all_enabled
    }

    /// Get the model generation style.
    pub fn model_style(&self) -> ModelStyle {
        self.model_style
    }

    /// Set the model generation style.
    pub fn set_model_style(&mut self, style: ModelStyle) {
        self.model_style = style;

        // When model_style is GraphQL, auto-enable the graphql and graphql_async plugins
        if style.is_graphql() {
            self.enable_by_name("graphql");
            self.enable_by_name("graphql_async");
        }
    }

    /// Create a config from prax.toml settings and environment variables.
    ///
    /// Environment variables take precedence over prax.toml settings.
    pub fn with_model_style(style: ModelStyle) -> Self {
        let mut config = Self::from_env();
        config.set_model_style(style);
        config
    }
}

/// Check if a string value is truthy.
fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "1" | "true" | "yes" | "on" | "enabled"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_truthy() {
        assert!(is_truthy("1"));
        assert!(is_truthy("true"));
        assert!(is_truthy("TRUE"));
        assert!(is_truthy("yes"));
        assert!(is_truthy("on"));
        assert!(is_truthy("enabled"));

        assert!(!is_truthy("0"));
        assert!(!is_truthy("false"));
        assert!(!is_truthy("no"));
        assert!(!is_truthy("off"));
        assert!(!is_truthy(""));
    }

    #[test]
    fn test_plugin_config_new() {
        let config = PluginConfig::new();
        assert!(!config.all_plugins_enabled());
        assert!(!config.is_enabled("PRAX_PLUGIN_DEBUG"));
    }

    #[test]
    fn test_plugin_config_all_enabled() {
        let config = PluginConfig::all_enabled();
        assert!(config.all_plugins_enabled());
        assert!(config.is_enabled("PRAX_PLUGIN_DEBUG"));
        assert!(config.is_enabled("PRAX_PLUGIN_ANYTHING"));
    }

    #[test]
    fn test_plugin_config_override() {
        let mut config = PluginConfig::all_enabled();

        // Disable a specific plugin
        config.disable("PRAX_PLUGIN_DEBUG");

        assert!(config.is_enabled("PRAX_PLUGIN_JSON_SCHEMA"));
        assert!(!config.is_enabled("PRAX_PLUGIN_DEBUG"));
    }

    #[test]
    fn test_plugin_config_enable_by_name() {
        let mut config = PluginConfig::new();

        config.enable_by_name("debug");
        assert!(config.is_enabled("PRAX_PLUGIN_DEBUG"));
        assert!(!config.is_enabled("PRAX_PLUGIN_OTHER"));
    }

    #[test]
    fn test_plugin_config_is_enabled_by_name() {
        let mut config = PluginConfig::new();
        config.enable_by_name("json_schema");

        assert!(config.is_enabled_by_name("json_schema"));
        assert!(!config.is_enabled_by_name("debug"));
    }
}
