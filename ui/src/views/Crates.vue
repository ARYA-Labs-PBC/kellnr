<template>
  <v-container fluid class="pa-0 main-container">
    <!-- Search Header -->
    <v-card class="search-card pa-4 ma-3" elevation="0" rounded="lg">
      <v-row no-gutters align="center">
        <v-col cols="12" md="8" lg="6" class="pr-md-4">
          <v-text-field v-model="searchText" placeholder="Search for crates" variant="outlined" density="comfortable"
            hide-details prepend-inner-icon="mdi-magnify" color="primary" data-testid="crates-search"
            @keyup.enter="handleSearch(searchText)" class="search-field" rounded="lg"></v-text-field>
        </v-col>

        <v-col cols="12" md="4" lg="6" class="mt-3 mt-md-0 d-flex align-center">
          <v-switch v-model="store.searchCache" color="primary" hide-details data-testid="crates-proxy-toggle"
            @update:model-value="refreshCrates()">
            <template v-slot:label>
              <div class="d-flex align-center" data-testid="crates-proxy-label">
                <span class="mr-2 switch-label">Crates proxy</span>
                <v-tooltip location="top" text="Display crates from the crates.io proxy">
                  <template v-slot:activator="{ props }">
                    <v-icon icon="mdi-information-outline" size="small" v-bind="props" class="info-icon" />
                  </template>
                </v-tooltip>
              </div>
            </template>
          </v-switch>
        </v-col>
      </v-row>

      <!-- View mode toggle: Grouped (by collection) vs Flat grid -->
      <v-row no-gutters align="center" class="mt-3">
        <v-col cols="12" class="d-flex align-center flex-wrap">
          <span class="mr-2 switch-label">View</span>
          <v-btn-toggle :model-value="viewMode" color="primary" density="comfortable" variant="outlined" divided
            mandatory data-testid="crates-view-toggle" @update:model-value="setViewMode">
            <v-btn value="grouped" size="small" data-testid="crates-view-grouped">
              <v-icon icon="mdi-folder-multiple-outline" size="small" class="mr-1" />
              Grouped
            </v-btn>
            <v-btn value="flat" size="small" data-testid="crates-view-flat">
              <v-icon icon="mdi-view-grid-outline" size="small" class="mr-1" />
              Flat
            </v-btn>
          </v-btn-toggle>

          <!-- Active collection filter (arrived via a "Collection" link from the crate detail page) -->
          <v-chip v-if="collectionFilter" size="small" variant="tonal" color="secondary" class="ml-3"
            closable data-testid="crates-collection-filter-chip" @click:close="clearCollectionFilter">
            Collection: {{ collectionFilter }}
          </v-chip>
        </v-col>
      </v-row>
    </v-card>

    <!-- Scrollable Content Container -->
    <div class="content-container" ref="scrollContainer" data-testid="crates-scroll-container" @scroll="handleScroll">
      <!-- Empty State -->
      <v-card v-if="crates.length === 0 && !isLoading" class="pa-6 text-center mx-auto my-8" max-width="500"
        variant="outlined" data-testid="crates-empty-state">
        <v-card-title class="text-h6 font-weight-medium">No crates found</v-card-title>
        <v-card-text>
          <p>To learn how to publish crates to <strong>Kellnr</strong>, read the
            <a href="https://kellnr.io/documentation" target="_blank" class="text-decoration-none font-weight-medium">
              documentation
            </a>
          </p>
          <v-icon icon="mdi-package-variant" size="x-large" color="grey-lighten-1" class="my-4"></v-icon>
        </v-card-text>
      </v-card>

      <!-- Flat Crates Grid -->
      <v-row v-if="viewMode === 'flat'" class="pa-3">
        <v-col cols="12">
          <crate-card v-for="crate in crates" :key="`${crate.name}-${crate.version}`" :crate="crate.name"
            :version="crate.version" :updated="crate.date" :downloads="crate.total_downloads" :desc="crate.description"
            :doc-link="crate.documentation" :is-cache="crate.is_cache"
            :is-primary="isCollectionPrimary(crate)"></crate-card>
        </v-col>
      </v-row>

      <!--
        Grouped-by-collection view. NOTE: this groups whatever crates are currently loaded into
        `crates` (the flat infinite-scroll/search list), not the full backend catalog — the
        `/api/v1/ui/crates` endpoint is paginated and has no server-side "group by collection"
        mode. Scrolling further (or searching) simply feeds more crates into the same grouping.
      -->
      <div v-else class="pa-3 grouped-view" data-testid="crates-grouped-view">
        <div v-if="collectionFilter && groupedSections.length === 0" class="text-center my-6 text-body-2 text-grey">
          No loaded crates match collection "{{ collectionFilter }}" yet — keep scrolling or search to load more.
        </div>
        <v-expansion-panels v-model="expandedKeys" multiple variant="accordion" class="collection-panels">
          <v-expansion-panel v-for="section in groupedSections" :key="section.key" :value="section.key">
            <v-expansion-panel-title>
              <div class="d-flex align-center collection-panel-header">
                <v-icon :icon="section.isUngrouped ? 'mdi-package-variant-closed' : 'mdi-folder-multiple'"
                  size="small" class="mr-2 collection-icon" />
                <span class="collection-name font-weight-medium">{{ section.name }}</span>
                <v-chip size="small" variant="tonal" color="primary" class="ml-3 collection-count-chip">
                  {{ section.crates.length }} {{ section.crates.length === 1 ? 'crate' : 'crates' }}
                </v-chip>
              </div>
            </v-expansion-panel-title>
            <v-expansion-panel-text>
              <crate-card v-for="crate in section.crates" :key="`${crate.name}-${crate.version}`" :crate="crate.name"
                :version="crate.version" :updated="crate.date" :downloads="crate.total_downloads"
                :desc="crate.description" :doc-link="crate.documentation" :is-cache="crate.is_cache"
                :is-primary="isCollectionPrimary(crate)"></crate-card>
            </v-expansion-panel-text>
          </v-expansion-panel>
        </v-expansion-panels>
      </div>

      <!-- Loading Indicator -->
      <div v-if="isLoading" class="text-center my-4 pb-4" data-testid="crates-loading">
        <v-progress-circular indeterminate color="primary" :size="40"></v-progress-circular>
        <div class="text-body-2 mt-2">Loading crates...</div>
      </div>

      <!-- End of Results -->
      <div v-if="allLoaded && crates.length > 0" class="text-center my-6 text-body-2 text-grey"
        data-testid="crates-end-of-results">
        — End of crates —
      </div>
    </div>
  </v-container>
</template>

<script setup lang="ts">
import { onBeforeMount, onMounted, ref, computed, watch, nextTick } from "vue"
import CrateCard from "../components/CrateCard.vue"
import type { CrateOverview } from "../types/crate_overview"
import { crateService } from "../services"
import { isSuccess } from "../services/api"
import { useRouter } from "vue-router"
import { useStore } from "../store/store"

// Constants
const ITEMS_PER_PAGE = 20
// Sentinel key for the "Ungrouped" section (crates with collection === null/absent)
const UNGROUPED_KEY = "__ungrouped__"

// State
const crates = ref<CrateOverview[]>([])
const currentPage = ref(0)
const isLoading = ref(false)
const allLoaded = ref(false)
const searchText = ref("")
const scrollContainer = ref<HTMLElement | null>(null)
const router = useRouter()
const store = useStore()

// View mode: `null` means "not yet chosen by the user" -> follow the computed default below.
// Once the user clicks the toggle, their choice sticks for the rest of the session.
const manualViewMode = ref<"grouped" | "flat" | null>(null)
// Collection to restrict the grouped view to, set when arriving via a "Collection" link
// from the crate detail page (see About.vue). Null = show every collection.
const collectionFilter = ref<string | null>(null)
// Tracks which collection panels are expanded; new sections auto-expand the first time they appear.
const expandedKeys = ref<string[]>([])

// Treat a missing/undefined `collection` the same as an explicit null (older backend responses
// won't have the field at all yet).
function getCollection(crate: CrateOverview): string | null {
  return crate.collection ?? null
}

function isCollectionPrimary(crate: CrateOverview): boolean {
  return crate.collection_primary ?? false
}

// Default to the Grouped view once any loaded crate actually has a collection; otherwise Flat.
const hasAnyCollection = computed(() => crates.value.some((c) => getCollection(c) !== null))
const viewMode = computed<"grouped" | "flat">(
  () => manualViewMode.value ?? (hasAnyCollection.value ? "grouped" : "flat")
)

function setViewMode(mode: "grouped" | "flat") {
  manualViewMode.value = mode
}

type CollectionSection = {
  key: string
  name: string
  isUngrouped: boolean
  crates: CrateOverview[]
}

// Group the currently-loaded crates by `collection`, sorted alphabetically by collection name,
// with primary/entry crates surfaced first within each section and an "Ungrouped" section last.
const groupedSections = computed<CollectionSection[]>(() => {
  const byCollection = new Map<string, CrateOverview[]>()

  for (const crate of crates.value) {
    const key = getCollection(crate) ?? UNGROUPED_KEY
    const bucket = byCollection.get(key)
    if (bucket) {
      bucket.push(crate)
    } else {
      byCollection.set(key, [crate])
    }
  }

  const sortWithinSection = (a: CrateOverview, b: CrateOverview) => {
    const aPrimary = isCollectionPrimary(a)
    const bPrimary = isCollectionPrimary(b)
    if (aPrimary !== bPrimary) return aPrimary ? -1 : 1
    return a.name.localeCompare(b.name)
  }

  const sections: CollectionSection[] = [...byCollection.keys()]
    .filter((key) => key !== UNGROUPED_KEY)
    .filter((key) => !collectionFilter.value || key === collectionFilter.value)
    .sort((a, b) => a.localeCompare(b))
    .map((key) => ({
      key,
      name: key,
      isUngrouped: false,
      crates: [...byCollection.get(key)!].sort(sortWithinSection),
    }))

  const ungrouped = byCollection.get(UNGROUPED_KEY)
  if (ungrouped && !collectionFilter.value) {
    sections.push({
      key: UNGROUPED_KEY,
      name: "Ungrouped",
      isUngrouped: true,
      crates: [...ungrouped].sort((a, b) => a.name.localeCompare(b.name)),
    })
  }

  return sections
})

// Auto-expand any section the first time it appears; never auto-collapse a section the
// user has manually closed (we only ever add keys here, never remove them).
watch(
  groupedSections,
  (sections) => {
    for (const section of sections) {
      if (!expandedKeys.value.includes(section.key)) {
        expandedKeys.value.push(section.key)
      }
    }
  },
  { immediate: true }
)

function clearCollectionFilter() {
  collectionFilter.value = null
  const query = { ...router.currentRoute.value.query }
  delete query.collection
  router.replace({ query })
}

// Initial setup
onBeforeMount(() => {
  if (router.currentRoute.value.query.search) {
    searchText.value = router.currentRoute.value.query.search as string
    handleSearch(searchText.value)
  }

  if (router.currentRoute.value.query.collection) {
    collectionFilter.value = router.currentRoute.value.query.collection as string
    manualViewMode.value = "grouped"
  }
})

onMounted(() => {
  if (searchText.value === "") {
    loadMoreCrates()
  }

  // Add resize event listener to handle window size changes
  window.addEventListener('resize', updateContainerHeight)
  // Initial height setup
  updateContainerHeight()
})

// Update container height to fill available space
function updateContainerHeight() {
  nextTick(() => {
    if (scrollContainer.value) {
      const headerHeight = document.querySelector('.v-card.pa-3.ma-3')?.clientHeight || 0
      const headerMargin = 24 // 3 * 8px (ma-3)

      // Calculate and set the height of the scrollable container
      const windowHeight = window.innerHeight
      const footerHeight = 48 // Height of the footer if present
      const availableHeight = windowHeight - headerHeight - headerMargin - footerHeight - 16

      scrollContainer.value.style.height = `${Math.max(300, availableHeight)}px`
    }
  })
}

// Load more crates for infinite scrolling
async function loadMoreCrates() {
  if (isLoading.value || allLoaded.value) return

  isLoading.value = true

  const result = await crateService.getCrates(
    currentPage.value,
    ITEMS_PER_PAGE,
    store.searchCache
  )

  isLoading.value = false

  if (isSuccess(result)) {
    const newCrates = result.data.crates

    // Add crates to the list
    crates.value = [...crates.value, ...newCrates]

    // Increment page for next load
    currentPage.value = result.data.page + 1

    // Check if we've loaded all available crates
    if (newCrates.length < ITEMS_PER_PAGE) {
      allLoaded.value = true
    }
  }
}

// Handle scroll event for infinite scrolling
function handleScroll(event: Event) {
  const target = event.target as HTMLElement

  // If we're in search mode, don't use infinite scroll
  if (searchText.value !== "") return

  // Calculate if we're near the bottom (within 200px)
  const scrollTop = target.scrollTop
  const scrollHeight = target.scrollHeight
  const clientHeight = target.clientHeight

  const scrollBottom = scrollHeight - scrollTop - clientHeight
  const isNearBottom = scrollBottom < 200

  if (isNearBottom && !isLoading.value && !allLoaded.value) {
    loadMoreCrates()
  }
}

// Refresh crates (used when changing filters)
function refreshCrates() {
  crates.value = []
  currentPage.value = 0
  allLoaded.value = false

  // Reset scroll position
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = 0
  }

  loadMoreCrates()
}

// Search crates by name
async function handleSearch(query: string) {
  const searchQuery = query.trim()

  if (!searchQuery) {
    refreshCrates()
    return
  }

  isLoading.value = true
  allLoaded.value = false

  // Reset scroll position
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = 0
  }

  const result = await crateService.searchCrates(searchQuery, store.searchCache)

  isLoading.value = false

  if (isSuccess(result)) {
    crates.value = result.data.crates
    allLoaded.value = true // Search results are all loaded at once
  } else {
    crates.value = []
    allLoaded.value = true
  }
}
</script>

<style scoped>
.main-container {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 64px);
  /* Adjust for app bar height */
  overflow: hidden;
  position: relative;
}

/* Search Card */
.search-card {
  background: rgb(var(--v-theme-surface));
  border: 1px solid rgb(var(--v-theme-outline));
}

/* Search Field */
.search-field :deep(.v-field) {
  background: rgb(var(--v-theme-surface));
  border-radius: 8px;
}

.search-field :deep(.v-field__input) {
  padding-top: 10px;
  padding-bottom: 10px;
  min-height: 44px;
}

.search-field :deep(.v-field__prepend-inner .v-icon) {
  color: rgb(var(--v-theme-primary));
  opacity: 0.8;
}

/* Switch Label */
.switch-label {
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
}

.info-icon {
  color: rgb(var(--v-theme-primary));
  opacity: 0.7;
}

/* Grouped (by collection) view */
.collection-panels {
  background: transparent;
}

.collection-panel-header {
  flex-wrap: wrap;
}

.collection-icon {
  color: rgb(var(--v-theme-primary));
  opacity: 0.8;
}

.collection-name {
  color: rgb(var(--v-theme-on-surface));
}

.collection-count-chip {
  font-size: 0.75rem;
  font-weight: 500;
  height: 22px;
}

.content-container {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  padding: 0;
  margin: 0 12px 12px 12px;
}

/* Improve scrollbar appearance */
.content-container::-webkit-scrollbar {
  width: 6px;
}

.content-container::-webkit-scrollbar-thumb {
  background-color: rgba(0, 0, 0, 0.2);
  border-radius: 3px;
}

.content-container::-webkit-scrollbar-track {
  background: transparent;
}

/* Dark mode adjustments */
:deep(.v-theme--dark) .content-container::-webkit-scrollbar-thumb {
  background-color: rgba(255, 255, 255, 0.2);
}

/* Mobile adjustments */
@media (max-width: 600px) {
  .main-container {
    height: calc(100vh - 56px);
    /* Adjust for smaller mobile app bar */
  }

  .content-container {
    margin: 0 8px 8px 8px;
  }
}
</style>
