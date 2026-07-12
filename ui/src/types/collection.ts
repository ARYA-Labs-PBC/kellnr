// Types for the dependency-tree catalog view, returned by GET /api/v1/ui/collections.

/** A single crate within a collection, plus its intra-collection dependency edges. */
export type CollectionCrate = {
    name: string
    version: string
    /** Author-declared entrypoint flag (collection_primary). */
    primary: boolean
    /** Names of this crate's dependencies that belong to the SAME collection. */
    deps: string[]
}

/** A collection (crate family) with its member crates and dependency edges. */
export type CollectionView = {
    collection: string
    crates: CollectionCrate[]
}
