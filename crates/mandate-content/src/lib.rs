//! Mandate content: JSONC loading, schemas, validation, and the embedded base
//! pack (DEC-053, DEC-054, DEC-055).

pub mod bundle;
pub mod conversation;
pub mod jsonc;
pub mod knowledge;
pub mod layout;
pub mod map_layers;
pub mod media;
pub mod narrative;
pub mod overrides;
pub mod types;
pub mod validate;

#[cfg(feature = "embedded")]
pub mod embedded;

pub use bundle::{ContentBundle, Source, load};
pub use media::*;
pub use narrative::*;
pub use types::*;
pub use validate::{Issue, Severity};

/// JSON Schema for content files, generated from the Rust types.
pub fn content_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(types::ContentFile)).unwrap_or_default()
}
