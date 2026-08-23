//! Merges the packages of an external `PyPI` index into the crate overview.
//!
//! Kellnr hosts crates. Deployments that serve a private Python index next to
//! it (see the `pypi` settings section) end up with packages that never show
//! up in the UI, because nothing in the kellnr database knows about them.
//! The helpers here read that index and turn its packages into the same
//! [`CrateOverview`] shape the crate list already uses, so both appear in one
//! list. A package that is also published as a crate is left out: the crate
//! entry, which has versions, downloads and docs, wins.

use std::collections::HashSet;
use std::sync::Arc;

use kellnr_common::crate_overview::CrateOverview;
use kellnr_common::pypi_index::{PypiIndexClient, PypiPackage};
use kellnr_db::DbProvider;

/// Upper bound on the packages a single search request resolves. Search
/// answers are not paginated, and every entry costs one (cached) request to
/// the index.
const MAX_SEARCH_RESULTS: usize = 50;

/// Comparable form of a package name across both ecosystems.
///
/// PEP 503 folds runs of `-`, `_` and `.` into a single `-` and lowercases,
/// cargo treats `-` and `_` as equivalent. Applying the PEP 503 rule to both
/// sides makes `arya_tools` (`PyPI`) and `arya-tools` (crate) the same name.
#[must_use]
pub fn normalize(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    for c in name.chars() {
        if matches!(c, '-' | '_' | '.') {
            if !normalized.ends_with('-') {
                normalized.push('-');
            }
        } else {
            normalized.extend(c.to_lowercase());
        }
    }
    normalized.trim_matches('-').to_string()
}

/// Package names of the index that are not published as a crate as well.
async fn unique_names(client: &PypiIndexClient, db: &Arc<dyn DbProvider>) -> Vec<String> {
    let names = client.package_names().await;
    if names.is_empty() {
        return Vec::new();
    }
    let crates: HashSet<String> = db
        .get_crate_summaries()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|c| normalize(&c.name))
        .collect();

    names
        .iter()
        .filter(|name| !crates.contains(&normalize(name)))
        .cloned()
        .collect()
}

/// A window of the PyPI-only packages, ready to be appended to a crate page.
///
/// The overall list is "all crates, then all PyPI-only packages", so `skip` is
/// the offset into the second part.
pub async fn listing(
    client: &Arc<PypiIndexClient>,
    db: &Arc<dyn DbProvider>,
    skip: usize,
    take: usize,
) -> Vec<CrateOverview> {
    if take == 0 {
        return Vec::new();
    }
    let names = unique_names(client, db).await;
    let window: Vec<String> = names.into_iter().skip(skip).take(take).collect();
    overviews(client, &window).await
}

/// PyPI-only packages whose name contains `query`, for the search endpoint.
pub async fn matching(
    client: &Arc<PypiIndexClient>,
    db: &Arc<dyn DbProvider>,
    query: &str,
) -> Vec<CrateOverview> {
    let query = normalize(query);
    let names: Vec<String> = unique_names(client, db)
        .await
        .into_iter()
        .filter(|name| query.is_empty() || normalize(name).contains(&query))
        .take(MAX_SEARCH_RESULTS)
        .collect();
    overviews(client, &names).await
}

async fn overviews(client: &Arc<PypiIndexClient>, names: &[String]) -> Vec<CrateOverview> {
    client
        .packages(names)
        .await
        .iter()
        .map(|package| overview(package))
        .collect()
}

/// Turn a package into the overview shape the crate list uses.
///
/// A simple index knows nothing about downloads, descriptions or docs, so
/// those stay empty and the UI hides them for `PyPI` entries.
fn overview(package: &PypiPackage) -> CrateOverview {
    CrateOverview {
        name: package.name.clone(),
        version: package.version.clone(),
        date: package.last_updated.clone().unwrap_or_default(),
        total_downloads: 0,
        description: None,
        documentation: None,
        // A PyPI package is never part of a kellnr crate collection.
        collection: None,
        collection_primary: false,
        is_cache: false,
        is_pypi: true,
        pypi_url: Some(package.url.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_folds_separators() {
        assert_eq!(normalize("Arya_Tools"), "arya-tools");
        assert_eq!(normalize("arya.tools"), "arya-tools");
        assert_eq!(normalize("arya--tools"), "arya-tools");
        assert_eq!(normalize("arya-tools"), normalize("arya_tools"));
    }

    #[test]
    fn normalize_trims_separators() {
        assert_eq!(normalize("_arya_"), "arya");
    }

    #[test]
    fn overview_marks_package_as_pypi() {
        let package = PypiPackage {
            name: "arya-tools".to_string(),
            version: "1.2.3".to_string(),
            last_updated: Some("2026-08-01T10:00:00Z".to_string()),
            url: "/pypi/simple/arya-tools/".to_string(),
        };
        let overview = overview(&package);
        assert!(overview.is_pypi);
        assert!(!overview.is_cache);
        assert_eq!(overview.version, "1.2.3");
        assert_eq!(overview.date, "2026-08-01T10:00:00Z");
        assert_eq!(
            overview.pypi_url.as_deref(),
            Some("/pypi/simple/arya-tools/")
        );
    }

    #[test]
    fn overview_without_upload_time_has_empty_date() {
        let package = PypiPackage {
            name: "qantm".to_string(),
            version: String::new(),
            last_updated: None,
            url: "/pypi/simple/qantm/".to_string(),
        };
        assert_eq!(overview(&package).date, "");
    }
}
