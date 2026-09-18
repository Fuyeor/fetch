<!-- @/search/index.vue -->
<template>
  <header class="search-header">
    <search-input v-model="inputQuery" @search="triggerSearch" />
  </header>

  <div v-if="isLoading" class="search-layout">
    <search-skeleton v-for="i in 5" :key="i" />
  </div>

  <div v-else-if="isRetrieved && data!.results.length > 0">
    <search-card v-for="item in data!.results" :key="item.url" :item="item" />
  </div>

  <div v-else class="search-status">
    {{ t('search.empty', { query: q }) }}

    <ExternalSearchSuggestions
      :query="{
        gpt: `I just searched for '${inputQuery}' on Fuyeor Fetch Search but found no results. Please explain it to me in locale:${locale}`,
        gemini: `I just searched for '${inputQuery}' on Fuyeor Fetch Search but found no results. Please explain it to me in locale:${locale}`,
        default: `${inputQuery} site:fuyeor.com`,
      }"
    />
  </div>
</template>

<script setup lang="ts">
import SearchInput from './component/input.vue';
import SearchSkeleton from './component/skeleton.vue';
import SearchCard from './component/card.vue';

import { ref, watch, computed } from 'vue';
import { useRouter } from '@fuyeor/vue-router';
import { useLocale } from '@fuyeor/locale';
import { ExternalSearchSuggestions } from '@fuyeor/interactify';
import { useSearchResultsQuery } from '@/search/composable/useSearch.js';

const { q = '' } = defineProps<{
  q?: string;
}>();

const { t, locale } = useLocale();
const router = useRouter();
const inputQuery = ref<string>(q);

const checkAndRedirect = (query?: string) => {
  if (!query || !query.trim()) {
    router.replace({ name: 'Home' });
    return true;
  }
  return false;
};

watch(
  () => q,
  (newVal) => {
    if (checkAndRedirect(newVal)) return;
    inputQuery.value = newVal;
  },
  { immediate: true },
);

const { data, isLoading, error, isRetrieved } = useSearchResultsQuery(() => ({
  q,
}));

const triggerSearch = (newQuery: string) => {
  const trimmed = newQuery?.trim();
  if (trimmed === q) return;
  router.push({ name: 'Search', query: { q: trimmed } });
};
</script>

<style scoped>
.search-header {
  position: sticky;
  top: 0;
  z-index: 10;
  padding: 30px 24px 16px;
}

.search-layout {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 30px;
}
.search-status {
  padding: 32px;
}

@media (width <= 768px) {
  .search-header {
    top: var(--height-sticky-header);
    height: calc(var(--height-sticky-header) - 10px);

    padding: 0 16px;
  }
}
</style>
