use serde::{Deserialize, Serialize};

/// Stable runtime capability report shared across bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub tokenizer: bool,
    pub simd_json: bool,
    pub transport: bool,
    pub chunker_code: bool,
    pub chunker_markdown: bool,
    pub chunker_text: bool,
    pub ffi_available: bool,
}

/// Return the capabilities compiled into this build.
pub fn capabilities() -> CapabilitySet {
    CapabilitySet {
        tokenizer: cfg!(feature = "tokenizer"),
        simd_json: cfg!(feature = "simd"),
        transport: true,
        chunker_code: cfg!(feature = "chunker"),
        chunker_markdown: cfg!(any(feature = "chunker", feature = "chunker-base")),
        chunker_text: cfg!(any(feature = "chunker", feature = "chunker-base")),
        ffi_available: cfg!(feature = "ffi"),
    }
}
