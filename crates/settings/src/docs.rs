use provcfg::{ClapArgs, Configurable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Configurable, ClapArgs)]
#[serde(default)]
#[configurable(clap_prefix = "docs")]
pub struct Docs {
    /// Enable documentation hosting
    pub enabled: bool,

    /// Max docs size in MB
    pub max_size: usize,

    /// Build rustdoc with ALL of a crate's features enabled.
    ///
    /// Off by default, matching `docs.rs`. Forcing every feature on turns
    /// author-optional backends (CUDA, Metal, `PyO3`, Z3, ...) into hard build
    /// requirements, so a crate that is CPU-only by default fails to document
    /// on a machine without that toolchain — and a macOS-only feature cannot
    /// be built on Linux at all. A crate that genuinely needs extra features
    /// for meaningful docs should opt in per-crate via
    /// `[package.metadata.docs.rs]` instead of flipping this globally.
    pub all_features: bool,
}

impl Default for Docs {
    fn default() -> Self {
        Self {
            enabled: false,
            max_size: 100,
            all_features: false,
        }
    }
}
