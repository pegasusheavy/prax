//! Fuzz target for the Prax config parser.
//!
//! This target feeds arbitrary TOML strings to the config parser
//! to find crashes and panics.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_config_parser
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use prax_schema::config::Config;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string
    if let Ok(input) = std::str::from_utf8(data) {
        // The parser should never panic, only return errors
        let _ = Config::from_toml(input);
    }
});

// Additional structured fuzzing for config
#[cfg(test)]
mod structured {
    use arbitrary::{Arbitrary, Unstructured};
    use prax_schema::config::Config;

    /// A structured config for fuzzing.
    #[derive(Debug, Arbitrary)]
    struct FuzzConfig {
        database_provider: Option<String>,
        database_url: Option<String>,
        output_dir: Option<String>,
        preview_features: Vec<String>,
    }

    impl FuzzConfig {
        fn to_toml(&self) -> String {
            let mut toml = String::new();

            toml.push_str("[database]\n");
            if let Some(ref provider) = self.database_provider {
                toml.push_str(&format!("provider = \"{}\"\n", sanitize_toml_string(provider)));
            }
            if let Some(ref url) = self.database_url {
                toml.push_str(&format!("url = \"{}\"\n", sanitize_toml_string(url)));
            }

            toml.push_str("\n[generator]\n");
            if let Some(ref dir) = self.output_dir {
                toml.push_str(&format!("output = \"{}\"\n", sanitize_toml_string(dir)));
            }

            if !self.preview_features.is_empty() {
                toml.push_str("preview_features = [");
                let features: Vec<String> = self.preview_features
                    .iter()
                    .map(|f| format!("\"{}\"", sanitize_toml_string(f)))
                    .collect();
                toml.push_str(&features.join(", "));
                toml.push_str("]\n");
            }

            toml
        }
    }

    fn sanitize_toml_string(s: &str) -> String {
        s.chars()
            .filter(|c| !matches!(c, '"' | '\\' | '\n' | '\r'))
            .take(100)
            .collect()
    }
}

