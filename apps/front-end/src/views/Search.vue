<!-- @/views/Search.vue -->
<template>
  <header class="search-header">
    <SearchInput v-model="inputQuery" @search="triggerSearch" />
  </header>

  <SearchCard
    v-if="isRetrieved && data!.results.length > 0"
    v-for="item in data!.results"
    :key="item.url"
    :item="item"
  />

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
import SearchInput from '@/components/Search/Input.vue';
import SearchCard from '@/components/Search/Card.vue';

import { ref, watch } from 'vue';
import { useRouter } from '@fuyeor/vue-router';
import { useLocale } from '@fuyeor/locale';
import { ExternalSearchSuggestions } from '@fuyeor/interactify';
import { useSearchResultsQuery } from '@/composables/api/useSearch';

const { q } = defineProps<{
  q: string;
}>();

const { t, locale } = useLocale();

const router = useRouter();
const inputQuery = ref<string>(q);

watch(
  () => q,
  (newVal) => {
    inputQuery.value = newVal;
  },
);

const { data, isLoading, error, isRetrieved } = useSearchResultsQuery(() => ({
  q: q,
}));

const triggerSearch = (newQuery: string) => {
  if (newQuery === q) return;
  router.push({ name: 'Search', query: { q: newQuery } });
};
</script>

<style scoped>
.search-header {
  position: sticky;
  top: 0;
  z-index: 10;
  padding: 16px 24px;
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
