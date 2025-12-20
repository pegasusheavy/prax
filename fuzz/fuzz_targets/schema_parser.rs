//! Fuzz target for the Prax schema parser.
//!
//! This target feeds arbitrary byte sequences to the schema parser to find
//! crashes, panics, and other unexpected behavior.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_schema_parser
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use prax_schema::parser::parse_schema;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, ignoring invalid UTF-8
    if let Ok(input) = std::str::from_utf8(data) {
        // The parser should never panic, only return errors
        let _ = parse_schema(input);
    }
});

