//! Mandate content: JSONC loading, schemas, validation, and the embedded base
//! pack (DEC-053, DEC-054, DEC-055).

pub mod bundle;
pub mod jsonc;
pub mod overrides;
pub mod types;
pub mod validate;

#[cfg(feature = "embedded")]
pub mod embedded;

pub use bundle::{ContentBundle, Source, load};
pub use types::*;
pub use validate::{Issue, Severity};

/// JSON Schema for content files, generated from the Rust types.
pub fn content_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(types::ContentFile)).unwrap_or_default()
}
