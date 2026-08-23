use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromQueryResult, ToSchema,
)]
pub struct CrateOverview {
    pub name: String,
    pub version: String,
    pub date: String,
    pub total_downloads: i64,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub collection: Option<String>,
    pub collection_primary: bool,
    pub is_cache: bool,
    /// Set for entries that come from the external `PyPI` index instead of the
    /// kellnr database. Never selected from SQL, hence skipped.
    #[sea_orm(skip)]
    #[serde(default)]
    pub is_pypi: bool,
    /// Link to the package page of the external `PyPI` index, `None` for crates.
    #[sea_orm(skip)]
    #[serde(default)]
    pub pypi_url: Option<String>,
}

/// A single crate inside a collection, carrying the intra-collection first-party
/// dependency edges the catalog uses to render the family's dependency tree.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CollectionCrate {
    pub name: String,
    pub version: String,
    /// Author-declared entrypoint flag (`collection_primary`).
    pub primary: bool,
    /// Names of this crate's dependencies that belong to the SAME collection.
    /// The frontend derives the tree from these edges: a crate that appears in no
    /// other member's `deps` is a root (the "main" crate you'd depend on).
    pub deps: Vec<String>,
}

/// A collection (crate family) and its member crates plus dependency edges,
/// returned by `GET /api/v1/ui/collections` for the dependency-tree catalog view.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CollectionView {
    pub collection: String,
    pub crates: Vec<CollectionCrate>,
}
