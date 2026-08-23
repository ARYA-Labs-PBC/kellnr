use provcfg::{ClapArgs, Configurable};
use serde::{Deserialize, Serialize};
use url::Url;

fn default_index_url() -> Url {
    Url::parse("http://localhost/simple/").unwrap()
}

/// Configuration for an external PEP 503 ("simple") `PyPI` package index that
/// is served next to kellnr, e.g. behind the same reverse proxy at `/pypi`.
///
/// Kellnr does not host Python packages itself. When this is enabled, the web
/// UI additionally lists the packages found in that index, so a registry that
/// serves both crates and wheels shows both in one place. Packages whose name
/// matches a crate already hosted by kellnr are not listed twice.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Configurable, ClapArgs)]
#[serde(default)]
#[configurable(clap_prefix = "pypi")]
pub struct Pypi {
    /// Show packages from an external `PyPI` index in the web UI
    pub enabled: bool,

    /// URL of the PEP 503 simple index, e.g. <https://example.com/pypi/simple/>
    #[arg(long = "pypi-index")]
    pub index: Url,

    /// Public base URL used for links to a package, e.g. `/pypi/simple`.
    /// Empty means the links point at `index` itself.
    #[arg(long = "pypi-link-base")]
    pub link_base: String,

    /// Basic-auth user for the index. Empty means no authentication.
    pub username: String,

    /// Basic-auth password for the index (prefer setting via
    /// `KELLNR_PYPI__PASSWORD` env var)
    #[serde(skip_serializing, default)]
    #[configurable(secret)]
    pub password: String,

    /// How long index responses are cached before they are fetched again
    #[arg(long = "pypi-cache-seconds")]
    pub cache_seconds: u64,

    /// Upper bound on the number of packages read from the index
    #[arg(long = "pypi-max-packages")]
    pub max_packages: usize,

    /// Connect timeout in seconds for requests to the index
    #[arg(long = "pypi-connect-timeout")]
    pub connect_timeout_seconds: u64,

    /// Request timeout in seconds for requests to the index
    #[arg(long = "pypi-request-timeout")]
    pub request_timeout_seconds: u64,
}

impl Default for Pypi {
    fn default() -> Self {
        Self {
            enabled: false,
            index: default_index_url(),
            link_base: String::new(),
            username: String::new(),
            password: String::new(),
            cache_seconds: 300,
            max_packages: 5000,
            connect_timeout_seconds: 5,
            request_timeout_seconds: 30,
        }
    }
}

impl Pypi {
    /// Basic-auth credentials for the index, if a user is configured.
    #[must_use]
    pub fn credentials(&self) -> Option<(&str, &str)> {
        if self.username.is_empty() {
            None
        } else {
            Some((&self.username, &self.password))
        }
    }

    /// Base URL a UI link to a single package is built from. Falls back to the
    /// index URL when no separate public base is configured.
    #[must_use]
    pub fn public_base(&self) -> String {
        let base = if self.link_base.is_empty() {
            self.index.as_str()
        } else {
            &self.link_base
        };
        base.trim_end_matches('/').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        let pypi = Pypi::default();
        assert!(!pypi.enabled);
        assert_eq!(pypi.credentials(), None);
    }

    #[test]
    fn deserialize_from_toml() {
        let toml = r#"
            enabled = true
            index = "https://registry.example.com/pypi/simple/"
            username = "ci"
            password = "secret"
        "#;
        let pypi: Pypi = toml::from_str(toml).unwrap();
        assert!(pypi.enabled);
        assert_eq!(
            pypi.index.as_str(),
            "https://registry.example.com/pypi/simple/"
        );
        assert_eq!(pypi.credentials(), Some(("ci", "secret")));
        // Untouched leaves keep their defaults.
        assert_eq!(pypi.cache_seconds, 300);
    }

    #[test]
    fn public_base_falls_back_to_index() {
        let pypi: Pypi = toml::from_str(r#"index = "https://example.com/pypi/simple/""#).unwrap();
        assert_eq!(pypi.public_base(), "https://example.com/pypi/simple");
    }

    #[test]
    fn public_base_prefers_link_base() {
        let toml = r#"
            index = "http://pypi-internal:8080/simple/"
            link_base = "/pypi/simple/"
        "#;
        let pypi: Pypi = toml::from_str(toml).unwrap();
        assert_eq!(pypi.public_base(), "/pypi/simple");
    }
}
