<template>
  <div class="tree-node" data-testid="crate-tree-node">
    <div class="tree-row" :style="{ paddingLeft: depth * 22 + 'px' }">
      <v-icon
        :icon="hasChildren ? 'mdi-file-tree-outline' : 'mdi-package-variant-closed'"
        size="x-small"
        class="mr-1 tree-icon"
      />
      <router-link
        :to="{ name: 'Crate', params: { name: node.name, version: node.version } }"
        class="tree-crate-name"
        data-testid="crate-tree-link"
      >
        {{ node.name }}
      </router-link>
      <span class="tree-version">v{{ node.version }}</span>
      <v-chip
        v-if="node.primary"
        size="x-small"
        variant="tonal"
        color="secondary"
        class="ml-2 tree-main-chip"
      >
        main
      </v-chip>
      <!-- A crate reached through a dependency cycle is shown once, then not re-expanded. -->
      <v-chip v-else-if="cycle" size="x-small" variant="tonal" color="warning" class="ml-2">
        cyclic
      </v-chip>
    </div>

    <!-- Recurse into this crate's same-collection dependencies. -->
    <template v-if="!cycle">
      <crate-tree-node
        v-for="child in children"
        :key="child.name"
        :node="child"
        :lookup="lookup"
        :depth="depth + 1"
        :ancestors="[...ancestors, node.name]"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue"
import type { CollectionCrate } from "../types/collection"

const props = defineProps<{
  /** The crate rendered by this node. */
  node: CollectionCrate
  /** name -> crate map for the whole collection, to resolve dependency children. */
  lookup: Record<string, CollectionCrate>
  /** Indentation depth (0 = a root / main crate). */
  depth: number
  /** Crate names already on the path from the root, to break dependency cycles. */
  ancestors: string[]
}>()

// If this crate already appears on the current path, we've hit a cycle — render it
// once (labelled) and stop, so the tree can't recurse forever.
const cycle = computed(() => props.ancestors.includes(props.node.name))

// Children = this crate's same-collection dependencies, resolved to their crate rows.
const children = computed<CollectionCrate[]>(() =>
  cycle.value
    ? []
    : props.node.deps
        .map((name) => props.lookup[name])
        .filter((c): c is CollectionCrate => Boolean(c))
        .sort((a, b) => a.name.localeCompare(b.name))
)

const hasChildren = computed(() => children.value.length > 0)
</script>

<style scoped>
.tree-node {
  width: 100%;
}

.tree-row {
  display: flex;
  align-items: center;
  padding-top: 4px;
  padding-bottom: 4px;
  border-radius: 6px;
  transition: background-color 0.12s ease;
}

.tree-row:hover {
  background-color: rgba(var(--v-theme-primary), 0.06);
}

.tree-icon {
  color: rgb(var(--v-theme-primary));
  opacity: 0.75;
  flex-shrink: 0;
}

.tree-crate-name {
  font-weight: 500;
  color: rgb(var(--v-theme-primary));
  text-decoration: none;
}

.tree-crate-name:hover {
  text-decoration: underline;
}

.tree-version {
  margin-left: 8px;
  font-size: 0.78rem;
  color: rgb(var(--v-theme-on-surface));
  opacity: 0.6;
}

.tree-main-chip {
  font-weight: 600;
}
</style>
