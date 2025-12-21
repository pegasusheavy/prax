//! Environment variable expansion.

use super::{ConnectionError, ConnectionResult};
use std::collections::HashMap;

/// Source for environment variables.
pub trait EnvSource: Send + Sync {
    /// Get an environment variable value.
    fn get(&self, name: &str) -> Option<String>;

    /// Check if a variable exists.
    fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
}

/// Default environment source using std::env.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdEnvSource;

impl EnvSource for StdEnvSource {
    fn get(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// Environment source backed by a HashMap.
#[derive(Debug, Clone, Default)]
pub struct MapEnvSource {
    vars: HashMap<String, String>,
}

impl MapEnvSource {
    /// Create a new map-based environment source.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable.
    pub fn set(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(name.into(), value.into());
        self
    }

    /// Add multiple variables.
    pub fn with_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.vars.extend(vars);
        self
    }
}

impl EnvSource for MapEnvSource {
    fn get(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
}

/// Expands environment variables in strings.
///
/// Supported syntax:
/// - `${VAR}` - Required variable
/// - `${VAR:-default}` - Variable with default value
/// - `${VAR:?error message}` - Required with custom error
/// - `$VAR` - Simple variable reference
#[derive(Debug, Clone)]
pub struct EnvExpander<S: EnvSource = StdEnvSource> {
    source: S,
}

impl EnvExpander<StdEnvSource> {
    /// Create a new expander using the standard environment.
    pub fn new() -> Self {
        Self {
            source: StdEnvSource,
        }
    }
}

impl Default for EnvExpander<StdEnvSource> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: EnvSource> EnvExpander<S> {
    /// Create an expander with a custom environment source.
    pub fn with_source(source: S) -> Self {
        Self { source }
    }

    /// Expand environment variables in a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use prax_query::connection::EnvExpander;
    ///
    /// // SAFETY: This is for documentation purposes only
    /// unsafe { std::env::set_var("PRAX_TEST_HOST", "localhost") };
    /// let expander = EnvExpander::new();
    /// let result = expander.expand("postgres://${PRAX_TEST_HOST}/db").unwrap();
    /// assert_eq!(result, "postgres://localhost/db");
    /// unsafe { std::env::remove_var("PRAX_TEST_HOST") };
    /// ```
    pub fn expand(&self, input: &str) -> ConnectionResult<String> {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'{') {
                    // ${VAR} syntax
                    chars.next(); // consume '{'
                    let expanded = self.expand_braced(&mut chars)?;
                    result.push_str(&expanded);
                } else if chars
                    .peek()
                    .map_or(false, |c| c.is_alphabetic() || *c == '_')
                {
                    // $VAR syntax
                    let expanded = self.expand_simple(&mut chars)?;
                    result.push_str(&expanded);
                } else {
                    // Literal $
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        Ok(result)
    }

    fn expand_braced(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> ConnectionResult<String> {
        let mut name = String::new();
        let mut modifier = None;
        let mut modifier_value = String::new();

        while let Some(&c) = chars.peek() {
            if c == '}' {
                chars.next();
                break;
            } else if c == ':' && modifier.is_none() {
                chars.next();
                // Check for modifier type
                if let Some(&next) = chars.peek() {
                    modifier = Some(next);
                    chars.next();
                }
            } else if modifier.is_some() {
                modifier_value.push(c);
                chars.next();
            } else {
                name.push(c);
                chars.next();
            }
        }

        if name.is_empty() {
            return Err(ConnectionError::InvalidEnvValue {
                name: "".to_string(),
                message: "Empty variable name".to_string(),
            });
        }

        match self.source.get(&name) {
            Some(value) if !value.is_empty() => Ok(value),
            _ => {
                match modifier {
                    Some('-') => Ok(modifier_value),
                    Some('?') => Err(ConnectionError::InvalidEnvValue {
                        name: name.clone(),
                        message: if modifier_value.is_empty() {
                            format!("Required variable '{}' is not set", name)
                        } else {
                            modifier_value
                        },
                    }),
                    Some('+') => {
                        // ${VAR:+value} - value if VAR is set, empty otherwise
                        Ok(String::new())
                    }
                    _ => Err(ConnectionError::EnvNotFound(name)),
                }
            }
        }
    }

    fn expand_simple(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> ConnectionResult<String> {
        let mut name = String::new();

        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }

        self.source
            .get(&name)
            .ok_or_else(|| ConnectionError::EnvNotFound(name))
    }

    /// Expand a connection URL.
    pub fn expand_url(&self, url: &str) -> ConnectionResult<String> {
        self.expand(url)
    }

    /// Check if a string contains environment variable references.
    pub fn has_variables(input: &str) -> bool {
        input.contains('$')
    }
}

/// Expand environment variables using the standard environment.
pub fn expand_env(input: &str) -> ConnectionResult<String> {
    EnvExpander::new().expand(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_source() -> MapEnvSource {
        MapEnvSource::new()
            .set("HOST", "localhost")
            .set("PORT", "5432")
            .set("USER", "testuser")
            .set("PASS", "secret")
            .set("EMPTY", "")
    }

    #[test]
    fn test_expand_simple() {
        let expander = EnvExpander::with_source(test_source());

        assert_eq!(
            expander.expand("postgres://$HOST/db").unwrap(),
            "postgres://localhost/db"
        );
    }

    #[test]
    fn test_expand_braced() {
        let expander = EnvExpander::with_source(test_source());

        assert_eq!(
            expander.expand("postgres://${HOST}:${PORT}/db").unwrap(),
            "postgres://localhost:5432/db"
        );
    }

    #[test]
    fn test_expand_default() {
        let expander = EnvExpander::with_source(test_source());

        // Variable exists
        assert_eq!(expander.expand("${HOST:-default}").unwrap(), "localhost");

        // Variable doesn't exist
        assert_eq!(expander.expand("${MISSING:-default}").unwrap(), "default");

        // Empty variable
        assert_eq!(expander.expand("${EMPTY:-default}").unwrap(), "default");
    }

    #[test]
    fn test_expand_required() {
        let expander = EnvExpander::with_source(test_source());

        // Variable exists
        assert_eq!(
            expander.expand("${HOST:?Host is required}").unwrap(),
            "localhost"
        );

        // Variable doesn't exist
        let result = expander.expand("${MISSING:?Missing is required}");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing is required")
        );
    }

    #[test]
    fn test_expand_missing() {
        let expander = EnvExpander::with_source(test_source());

        let result = expander.expand("${MISSING}");
        assert!(matches!(result, Err(ConnectionError::EnvNotFound(_))));
    }

    #[test]
    fn test_expand_full_url() {
        let expander = EnvExpander::with_source(test_source());

        let url = "postgres://${USER}:${PASS}@${HOST}:${PORT}/mydb?sslmode=require";
        let expanded = expander.expand(url).unwrap();

        assert_eq!(
            expanded,
            "postgres://testuser:secret@localhost:5432/mydb?sslmode=require"
        );
    }

    #[test]
    fn test_has_variables() {
        assert!(EnvExpander::<StdEnvSource>::has_variables("${VAR}"));
        assert!(EnvExpander::<StdEnvSource>::has_variables("$VAR"));
        assert!(!EnvExpander::<StdEnvSource>::has_variables("no variables"));
    }

    #[test]
    fn test_literal_dollar() {
        let expander = EnvExpander::with_source(test_source());

        // Dollar followed by non-variable character
        assert_eq!(expander.expand("cost: $5").unwrap(), "cost: $5");
    }
}
