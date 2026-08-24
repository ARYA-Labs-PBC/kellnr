//! Parse optional catalog-grouping hints a crate author declares in `Cargo.toml`:
//!
//! ```toml
//! [package.metadata.kellnr]
//! collection = "my-workspace"   # groups related crates together in the catalog
//! primary = true                # marks this crate as an entry point of the collection
//! ```
//!
//! Kellnr otherwise never reads the uploaded `.crate` tarball for metadata (all
//! metadata comes from the JSON blob cargo sends alongside it, which does not
//! carry `[package.metadata]`). We therefore unpack the tarball's `Cargo.toml`
//! here. Parsing is strictly best-effort: any error yields the default (no
//! collection) and never fails a publish.

use std::io::Read;

use flate2::read::GzDecoder;
use tar::Archive;

/// Grouping hints extracted from `[package.metadata.kellnr]`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CollectionMeta {
    /// Name of the collection this crate belongs to, if any.
    pub collection: Option<String>,
    /// Whether this crate is an entry point of its collection.
    pub primary: bool,
}

#[derive(serde::Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
}

#[derive(serde::Deserialize)]
struct CargoPackage {
    metadata: Option<CargoMetadata>,
}

#[derive(serde::Deserialize)]
struct CargoMetadata {
    kellnr: Option<KellnrMeta>,
}

#[derive(serde::Deserialize)]
struct KellnrMeta {
    collection: Option<String>,
    #[serde(default)]
    primary: bool,
}

/// Best-effort parse of `[package.metadata.kellnr]` from the published crate's
/// `Cargo.toml`. Returns [`CollectionMeta::default`] on any error.
pub fn parse_collection_meta(cratedata: &[u8]) -> CollectionMeta {
    extract(cratedata).unwrap_or_default()
}

fn extract(cratedata: &[u8]) -> Option<CollectionMeta> {
    let tar = GzDecoder::new(std::io::Cursor::new(cratedata));
    let mut archive = Archive::new(tar);

    for entry in archive.entries().ok()? {
        let mut entry = entry.ok()?;
        let is_manifest = {
            let path = entry.path().ok()?;
            // The manifest sits at `<name>-<version>/Cargo.toml` (two path
            // components). `Cargo.toml.orig` is skipped — cargo keeps
            // `[package.metadata]` in the normalized `Cargo.toml`.
            path.components().count() == 2 && path.file_name().is_some_and(|f| f == "Cargo.toml")
        };
        if !is_manifest {
            continue;
        }

        let mut contents = String::new();
        entry.read_to_string(&mut contents).ok()?;
        let manifest: CargoToml = toml::from_str(&contents).ok()?;
        let kellnr = manifest.package?.metadata?.kellnr?;
        let collection = kellnr
            .collection
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        return Some(CollectionMeta {
            collection,
            primary: kellnr.primary,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn make_crate_tarball(cargo_toml: &str) -> Vec<u8> {
        let mut tar_builder = tar::Builder::new(Vec::new());
        let bytes = cargo_toml.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder
            .append_data(&mut header, "foo-0.1.0/Cargo.toml", bytes)
            .unwrap();
        let tar = tar_builder.into_inner().unwrap();
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&tar).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn parses_collection_and_primary() {
        let toml = r#"
[package]
name = "foo"
version = "0.1.0"

[package.metadata.kellnr]
collection = "workspace-x"
primary = true
"#;
        let meta = parse_collection_meta(&make_crate_tarball(toml));
        assert_eq!(meta.collection.as_deref(), Some("workspace-x"));
        assert!(meta.primary);
    }

    #[test]
    fn defaults_primary_false_and_trims_blank_collection() {
        let toml = r#"
[package]
name = "foo"
version = "0.1.0"

[package.metadata.kellnr]
collection = "   "
"#;
        let meta = parse_collection_meta(&make_crate_tarball(toml));
        assert_eq!(meta.collection, None);
        assert!(!meta.primary);
    }

    #[test]
    fn no_metadata_yields_default() {
        let toml = r#"
[package]
name = "foo"
version = "0.1.0"
"#;
        let meta = parse_collection_meta(&make_crate_tarball(toml));
        assert_eq!(meta, CollectionMeta::default());
    }

    #[test]
    fn garbage_input_is_best_effort_default() {
        assert_eq!(
            parse_collection_meta(b"not a tarball"),
            CollectionMeta::default()
        );
    }
}
