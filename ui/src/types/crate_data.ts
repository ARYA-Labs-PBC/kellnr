export type CrateData = {
    name: string,
    owners: Array<string>,
    max_version: string,
    total_downloads: number,
    last_updated: string,
    homepage?: string,
    description?: string,
    repository?: string,
    categories: Array<string>,
    keywords: Array<string>,
    authors: Array<string>,
    versions: Array<CrateVersionData>,
    /** Collection this crate belongs to (e.g. "rutorch" groups all rutorch-* crates). Null/absent = ungrouped. */
    collection?: string | null,
    /** True if this crate is the entry-point of its collection, vs an internal dependency crate. Absent on older backends. */
    collection_primary?: boolean,
}

export const defaultCrateData : CrateData = {
    name: "",
    owners: [],
    max_version: "",
    total_downloads: 0,
    last_updated: "",
    homepage: "",
    description: "",
    repository: "",
    categories: [],
    keywords: [],
    authors: [],
    versions: [],
    collection: null,
    collection_primary: false,
}

export type CrateVersionData = {
    version: string,
    created: string,
    downloads: number,
    readme?: string,
    license?: string,
    license_file?: string,
    documentation?: string,
    dependencies: Array<CrateRegistryDep>,
    checksum: string,
    features: { [key: string]: Array<string> },
    features2?: { [key: string]: Array<string> },
    yanked: boolean,
    links?: string,
    v: number,
}

export const defaultCrateVersionData : CrateVersionData = {
    version: "",
    created: "",
    downloads: 0,
    readme: "",
    license: "",
    license_file: "",
    documentation: "",
    dependencies: [],
    checksum: "",
    features: {},
    features2: undefined,
    yanked: false,
    links: "",
    v: 0,
}

export type CrateRegistryDep = {
    name: string,
    description?: string,
    version_req: string,
    features?: Array<string>,
    optional: boolean,
    default_features: boolean,
    target?: string,
    kind?: string,
    registry?: string,
    explicit_name_in_toml?: string,
}

export type CrateAccessData = {
    is_download_restricted: boolean,
}

export const defaultCrateAccessData : CrateAccessData = {
    is_download_restricted: true,
}

export interface CrateGroup {
    name: string;
}
