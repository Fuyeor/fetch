<!-- @/views/Search.vue -->
<template>
  <div class="search-view">
    <header class="search-header">
      <SearchInput v-model="inputQuery" @search="triggerSearch" />
    </header>

    <main class="search-main">
      <div v-if="isLoading" class="search-status">正在搜索中...</div>
      <div v-else-if="error" class="search-status search-error">
        {{ error }}
      </div>
      <div v-else-if="!data?.results.length" class="search-status">
        未找到与 "{{ currentQuery }}" 相关的内容
      </div>

      <div v-else class="search-results">
        <SearchCard v-for="item in data.results" :key="item.url" :item="item" />
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from '@fuyeor/vue-router';
import { useQuery } from '@/composables/api/useQuery';
import SearchInput from '@/components/Search/Input.vue';
import SearchCard from '@/components/Search/Card.vue';

const route = useRoute();
const router = useRouter();

const currentQuery = computed(() => (route.query.q as string) || '');
const inputQuery = ref<string>(currentQuery.value);

watch(currentQuery, (newVal) => {
  inputQuery.value = newVal;
});

const { data, isLoading, error } = useQuery(() => ({
  q: currentQuery.value,
}));

const triggerSearch = (newQuery: string) => {
  if (newQuery === currentQuery.value) return;
  router.push({ name: 'Search', query: { q: newQuery } });
};
</script>

<style scoped>
.search-view {
  min-height: 100vh;
  background-color: #ffffff;
}

.search-header {
  position: sticky;
  top: 0;
  z-index: 10;
  padding: 16px 24px;
  background-color: #ffffff;
  border-bottom: 1px solid #ebeef5;
}

.search-main {
  max-width: 680px;
  padding: 24px;
}

.search-status {
  padding: 32px 0;
  font-size: 15px;
  color: #606266;
}

.search-error {
  color: #f56c6c;
}

.search-results {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
</style>
