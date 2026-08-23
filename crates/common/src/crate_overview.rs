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
