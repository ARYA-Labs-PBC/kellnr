export type CrateOverview = {
    name: string
    version: string
    date: string
    total_downloads: number
    description?: string
    documentation?: string
    is_kellnr: boolean
    /** Alias for !is_kellnr - indicates if this is from crates.io cache */
    is_cache?: boolean
    /** Collection this crate belongs to (e.g. "rutorch" groups all rutorch-* crates). Null/absent = ungrouped. */
    collection?: string | null
    /** True if this crate is the entry-point of its collection, vs an internal dependency crate. Absent on older backends. */
    collection_primary?: boolean
}
