//! Read-only client for an external PEP 503 / PEP 691 Python package index.
//!
//! Kellnr does not host Python packages. Some deployments serve a private
//! index next to kellnr (e.g. behind the same reverse proxy at `/pypi`), and
//! the web UI lists those packages alongside the crates so that both live in
//! one overview. Everything here is read-only: the index is fetched, parsed
//! and cached, never written to.

use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use regex::Regex;
use reqwest::header::{ACCEPT, HeaderValue};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use tracing::{debug, warn};

/// Content types a PEP 691 aware index understands, in order of preference.
const SIMPLE_ACCEPT: &str = "application/vnd.pypi.simple.v1+json;q=1.0, text/html;q=0.5, application/vnd.pypi.simple.v1+html;q=0.5";

/// Archive suffixes of source distributions, longest first so that
/// `.tar.gz` wins over `.gz`.
const SDIST_SUFFIXES: [&str; 6] = [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".zip", ".tgz"];

/// A single package of the external index as shown in the kellnr UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PypiPackage {
    /// Name as listed by the index.
    pub name: String,
    /// Highest version found on the project page, empty if none could be read.
    pub version: String,
    /// Upload time of the newest file, if the index reports one (PEP 700).
    pub last_updated: Option<String>,
    /// Link to the project page for the UI.
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct SimpleProjectList {
    #[serde(default)]
    projects: Vec<SimpleProject>,
}

#[derive(Debug, Deserialize)]
struct SimpleProject {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SimpleProjectDetail {
    #[serde(default)]
    files: Vec<SimpleFile>,
    /// PEP 700 exposes the versions directly. Optional, older indexes omit it.
    #[serde(default)]
    versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SimpleFile {
    filename: String,
    #[serde(rename = "upload-time", default)]
    upload_time: Option<String>,
}

/// Fetches and caches the contents of an external simple index.
pub struct PypiIndexClient {
    client: Client,
    index: Url,
    credentials: Option<(String, String)>,
    public_base: String,
    max_packages: usize,
    names: Cache<(), Arc<Vec<String>>>,
    details: Cache<String, Arc<PypiPackage>>,
}

impl PypiIndexClient {
    #[must_use]
    pub fn new(
        index: Url,
        credentials: Option<(String, String)>,
        public_base: String,
        max_packages: usize,
        cache_ttl: Duration,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Self {
        let client = crate::cratesio_downloader::build_client(
            crate::cratesio_downloader::DEFAULT_USER_AGENT,
            connect_timeout,
            request_timeout,
        );
        Self {
            client,
            index: with_trailing_slash(index),
            credentials,
            public_base,
            max_packages,
            names: Cache::builder()
                .time_to_live(cache_ttl)
                .max_capacity(1)
                .build(),
            details: Cache::builder()
                .time_to_live(cache_ttl)
                .max_capacity(10_000)
                .build(),
        }
    }

    /// All package names of the index, sorted and cached.
    ///
    /// An unreachable or malformed index yields an empty list. The UI degrades
    /// to crates-only in that case instead of failing the whole request.
    pub async fn package_names(&self) -> Arc<Vec<String>> {
        if let Some(cached) = self.names.get(&()).await {
            return cached;
        }
        let names = Arc::new(self.fetch_package_names().await);
        self.names.insert((), names.clone()).await;
        names
    }

    /// Package details (latest version, upload time, link) for `name`.
    ///
    /// The project page is fetched once per cache interval. If it cannot be
    /// read, a package without version information is returned so the entry
    /// still shows up in the UI.
    pub async fn package(&self, name: &str) -> Arc<PypiPackage> {
        if let Some(cached) = self.details.get(name).await {
            return cached;
        }
        let package = Arc::new(self.fetch_package(name).await);
        self.details.insert(name.to_string(), package.clone()).await;
        package
    }

    /// Details for many packages at once, fetched with bounded concurrency.
    ///
    /// Results keep the order of `names`. Project pages are cached, so only a
    /// cold cache actually hits the index.
    pub async fn packages(self: &Arc<Self>, names: &[String]) -> Vec<Arc<PypiPackage>> {
        const MAX_PARALLEL_REQUESTS: usize = 8;

        let mut packages = Vec::with_capacity(names.len());
        for chunk in names.chunks(MAX_PARALLEL_REQUESTS) {
            let mut tasks = tokio::task::JoinSet::new();
            for (index, name) in chunk.iter().enumerate() {
                let client = self.clone();
                let name = name.clone();
                tasks.spawn(async move { (index, client.package(&name).await) });
            }
            let mut chunk_packages = tasks.join_all().await;
            chunk_packages.sort_by_key(|(index, _)| *index);
            packages.extend(chunk_packages.into_iter().map(|(_, package)| package));
        }
        packages
    }

    /// Link to the project page as shown in the UI.
    #[must_use]
    pub fn package_url(&self, name: &str) -> String {
        format!("{}/{}/", self.public_base, name)
    }

    async fn get(&self, url: Url) -> Option<(bool, String)> {
        let mut request = self
            .client
            .get(url.clone())
            .header(ACCEPT, HeaderValue::from_static(SIMPLE_ACCEPT));
        if let Some((user, password)) = &self.credentials {
            request = request.basic_auth(user, Some(password));
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                warn!("Cannot reach PyPI index at {url}: {e}");
                return None;
            }
        };
        if response.status() != StatusCode::OK {
            warn!("PyPI index {url} answered with {}", response.status());
            return None;
        }
        let is_json = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("json"));
        match response.text().await {
            Ok(body) => Some((is_json, body)),
            Err(e) => {
                warn!("Cannot read answer from PyPI index {url}: {e}");
                None
            }
        }
    }

    async fn fetch_package_names(&self) -> Vec<String> {
        let Some((is_json, body)) = self.get(self.index.clone()).await else {
            return Vec::new();
        };
        let mut names = if is_json {
            match serde_json::from_str::<SimpleProjectList>(&body) {
                Ok(list) => list.projects.into_iter().map(|p| p.name).collect(),
                Err(e) => {
                    warn!("Cannot parse PyPI index answer as JSON: {e}");
                    return Vec::new();
                }
            }
        } else {
            parse_anchor_texts(&body)
        };
        names.sort_unstable();
        names.dedup();
        if names.len() > self.max_packages {
            debug!(
                "PyPI index lists {} packages, truncated to {}",
                names.len(),
                self.max_packages
            );
            names.truncate(self.max_packages);
        }
        names
    }

    async fn fetch_package(&self, name: &str) -> PypiPackage {
        let url = self.package_url(name);
        let mut package = PypiPackage {
            name: name.to_string(),
            version: String::new(),
            last_updated: None,
            url: url.clone(),
        };

        let Ok(project_url) = self.index.join(&format!("{name}/")) else {
            warn!("Cannot build project URL for PyPI package {name}");
            return package;
        };
        let Some((is_json, body)) = self.get(project_url).await else {
            return package;
        };

        let (versions, uploads) = if is_json {
            match serde_json::from_str::<SimpleProjectDetail>(&body) {
                Ok(detail) => {
                    let mut versions = detail.versions;
                    versions.extend(
                        detail
                            .files
                            .iter()
                            .filter_map(|f| version_from_filename(&f.filename)),
                    );
                    let uploads = detail
                        .files
                        .iter()
                        .filter_map(|f| {
                            version_from_filename(&f.filename).zip(f.upload_time.clone())
                        })
                        .collect::<Vec<_>>();
                    (versions, uploads)
                }
                Err(e) => {
                    warn!("Cannot parse PyPI project page for {name} as JSON: {e}");
                    (Vec::new(), Vec::new())
                }
            }
        } else {
            let versions = parse_anchor_texts(&body)
                .iter()
                .filter_map(|f| version_from_filename(f))
                .collect();
            (versions, Vec::new())
        };

        if let Some(max) = max_version(&versions) {
            package.last_updated = uploads
                .iter()
                .filter(|(version, _)| version == &max)
                .map(|(_, upload)| upload.clone())
                .max();
            package.version = max;
        }
        package
    }
}

/// A simple index is a directory: a URL without a trailing slash would make
/// `Url::join` replace the last path segment instead of descending into it.
fn with_trailing_slash(url: Url) -> Url {
    if url.as_str().ends_with('/') {
        return url;
    }
    Url::parse(&format!("{url}/")).unwrap_or(url)
}

/// Text of every `<a>` element of a simple-index HTML page. Both the index
/// listing (package names) and a project page (file names) are built that way.
fn parse_anchor_texts(html: &str) -> Vec<String> {
    static ANCHOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?is)<a\b[^>]*>(.*?)</a>").expect("anchor regex is valid")
    });
    ANCHOR
        .captures_iter(html)
        .map(|c| c[1].trim().to_string())
        .filter(|text| !text.is_empty())
        .collect()
}

/// Version of a distribution file, derived from its name.
///
/// Wheels encode it as the second `-` separated field
/// (`kellnr_tools-1.2.3-py3-none-any.whl`), source distributions as the part
/// after the last `-` (`kellnr-tools-1.2.3.tar.gz`).
#[must_use]
pub fn version_from_filename(filename: &str) -> Option<String> {
    let filename = filename.split(['#', '?']).next().unwrap_or(filename);
    if let Some(stem) = filename
        .strip_suffix(".whl")
        .or_else(|| filename.strip_suffix(".egg"))
    {
        let mut fields = stem.split('-');
        let _name = fields.next()?;
        let version = fields.next()?;
        return (!version.is_empty()).then(|| version.to_string());
    }
    let stem = SDIST_SUFFIXES
        .iter()
        .find_map(|suffix| filename.strip_suffix(suffix))?;
    let (_name, version) = stem.rsplit_once('-')?;
    (!version.is_empty()).then(|| version.to_string())
}

/// Highest of the given versions.
///
/// Release segments are compared numerically, a version with a suffix
/// (`1.2.0rc1`, `1.2.0.dev3`) ranks below the same release without one. That
/// covers the ordering PEP 440 defines for the versions a private index
/// realistically serves, without pulling in a full PEP 440 implementation.
#[must_use]
pub fn max_version(versions: &[String]) -> Option<String> {
    versions
        .iter()
        .max_by(|a, b| version_key(a).cmp(&version_key(b)))
        .cloned()
}

fn version_key(version: &str) -> (Vec<u64>, bool, String) {
    let mut release = Vec::new();
    let mut is_final = true;
    for segment in version.split('.') {
        let digits: String = segment.chars().take_while(char::is_ascii_digit).collect();
        if digits.len() != segment.len() {
            is_final = false;
        }
        if digits.is_empty() {
            break;
        }
        release.push(digits.parse::<u64>().unwrap_or(u64::MAX));
        if !is_final {
            break;
        }
    }
    (release, is_final, version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_from_wheel_filename() {
        assert_eq!(
            version_from_filename("arya_tools-1.2.3-py3-none-any.whl").as_deref(),
            Some("1.2.3")
        );
    }

    #[test]
    fn version_from_sdist_filename() {
        assert_eq!(
            version_from_filename("arya-tools-1.2.3.tar.gz").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            version_from_filename("arya-tools-0.1.0.zip").as_deref(),
            Some("0.1.0")
        );
    }

    #[test]
    fn version_from_filename_ignores_fragment() {
        assert_eq!(
            version_from_filename("arya-tools-1.2.3.tar.gz#sha256=abc").as_deref(),
            Some("1.2.3")
        );
    }

    #[test]
    fn version_from_unknown_filename_is_none() {
        assert_eq!(version_from_filename("README.md"), None);
    }

    #[test]
    fn max_version_compares_numerically() {
        let versions = ["1.9.0", "1.10.0", "1.2.0"].map(String::from).to_vec();
        assert_eq!(max_version(&versions).as_deref(), Some("1.10.0"));
    }

    #[test]
    fn max_version_prefers_final_over_prerelease() {
        let versions = ["2.0.0rc1", "2.0.0", "2.0.0.dev4"]
            .map(String::from)
            .to_vec();
        assert_eq!(max_version(&versions).as_deref(), Some("2.0.0"));
    }

    #[test]
    fn max_version_of_empty_is_none() {
        assert_eq!(max_version(&[]), None);
    }

    #[test]
    fn index_url_gets_trailing_slash() {
        let url = with_trailing_slash(Url::parse("https://example.com/pypi/simple").unwrap());
        assert_eq!(url.as_str(), "https://example.com/pypi/simple/");
        assert_eq!(
            url.join("arya-tools/").unwrap().as_str(),
            "https://example.com/pypi/simple/arya-tools/"
        );
    }

    #[test]
    fn parse_anchor_texts_reads_index_page() {
        let html = r#"<!DOCTYPE html><html><body>
            <a href="/simple/arya-tools/">arya-tools</a>
            <a href="/simple/qantm/">qantm</a>
            </body></html>"#;
        assert_eq!(parse_anchor_texts(html), vec!["arya-tools", "qantm"]);
    }

    /// Serve a fake simple index on a random port and return its URL.
    ///
    /// The HTML flavour is used on purpose: it is what a plain
    /// `pypiserver`/nginx setup answers with, and it is the harder of the two
    /// formats to get right.
    async fn serve_index(index_page: &'static str, project_page: &'static str) -> Url {
        use axum::Router;
        use axum::routing::get;

        let app = Router::new()
            .route(
                "/simple/",
                get(move || async move { axum::response::Html(index_page) }),
            )
            .route(
                "/simple/arya-tools/",
                get(move || async move { axum::response::Html(project_page) }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Url::parse(&format!("http://{addr}/simple/")).unwrap()
    }

    fn client(index: Url) -> Arc<PypiIndexClient> {
        Arc::new(PypiIndexClient::new(
            index,
            None,
            "/pypi/simple".to_string(),
            100,
            Duration::from_mins(1),
            Duration::from_secs(5),
            Duration::from_secs(5),
        ))
    }

    const INDEX_PAGE: &str = r#"<!DOCTYPE html><html><body>
        <a href="arya-tools/">arya-tools</a>
        <a href="qantm/">qantm</a>
        </body></html>"#;

    const PROJECT_PAGE: &str = r#"<!DOCTYPE html><html><body>
        <a href="arya_tools-1.9.0-py3-none-any.whl">arya_tools-1.9.0-py3-none-any.whl</a>
        <a href="arya_tools-1.10.0-py3-none-any.whl">arya_tools-1.10.0-py3-none-any.whl</a>
        <a href="arya-tools-1.10.0.tar.gz">arya-tools-1.10.0.tar.gz</a>
        </body></html>"#;

    #[tokio::test]
    async fn package_names_are_read_from_the_index() {
        let client = client(serve_index(INDEX_PAGE, PROJECT_PAGE).await);
        assert_eq!(*client.package_names().await, vec!["arya-tools", "qantm"]);
    }

    #[tokio::test]
    async fn package_reads_the_highest_version_of_the_project_page() {
        let client = client(serve_index(INDEX_PAGE, PROJECT_PAGE).await);
        let package = client.package("arya-tools").await;
        assert_eq!(package.version, "1.10.0");
        assert_eq!(package.url, "/pypi/simple/arya-tools/");
        assert_eq!(package.last_updated, None);
    }

    #[tokio::test]
    async fn packages_keeps_the_requested_order() {
        let client = client(serve_index(INDEX_PAGE, PROJECT_PAGE).await);
        let names = ["qantm", "arya-tools"].map(String::from).to_vec();
        let packages = client.packages(&names).await;
        let read: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(read, vec!["qantm", "arya-tools"]);
        // The project page of "qantm" is not served, the entry survives anyway.
        assert_eq!(packages[0].version, "");
        assert_eq!(packages[1].version, "1.10.0");
    }

    #[tokio::test]
    async fn unreachable_index_yields_no_packages() {
        // Port 1 on loopback refuses the connection immediately.
        let client = client(Url::parse("http://127.0.0.1:1/simple/").unwrap());
        assert!(client.package_names().await.is_empty());
    }
}
