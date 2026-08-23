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
    /** Set for packages that come from the external PyPI index, not from kellnr */
    is_pypi?: boolean
    /** Link to the package page of the external PyPI index */
    pypi_url?: string
}
