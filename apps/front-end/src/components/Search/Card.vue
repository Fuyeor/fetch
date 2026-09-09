<!-- @/components/Search/Card.vue -->
<template>
  <article class="search-card">
    <h2 class="search-card-title">
      <a :href="item.url" target="_blank" class="link">{{ item.title }}</a>
    </h2>

    <cite class="search-card-url cite">{{ item.url }}</cite>

    <div class="search-card-body">
      <img
        v-if="item.images.length > 0"
        :src="item.images[0]"
        alt="Thumbnail"
        class="search-card-thumb"
        loading="lazy"
      />
      <p class="search-card-snippet">{{ item.snippet }}</p>
    </div>

    <time class="search-card-date cite">{{
      formatDate(item.updated_at, { preset: 'relative' })
    }}</time>
  </article>
</template>

<script setup lang="ts">
import { useDateFormatter } from '@fuyeor/commons';
import type { SearchItem } from '@/types/search';

const props = defineProps<{
  item: SearchItem;
}>();

const { formatDate } = useDateFormatter();
</script>

<style scoped>
.search-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 30px;
  transition: background-color 0.2s;

  &:hover {
    background-color: var(--surface-raised-hover);
  }
}

.cite {
  color: var(--text-tertiary);
  font-style: normal;
  font-size: 0.85rem;
}

.search-card-title {
  margin: 0;
  padding: 0;
  font-size: 1.1rem;
  font-weight: 500;
  line-height: 1.4;
  border: none;
}

.search-card-url {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-card-body {
  display: flex;
  gap: 12px;
  margin: 8px 0;
}

.search-card-thumb {
  width: 90px;
  height: 60px;
  object-fit: cover;
  border-radius: 4px;
  flex-shrink: 0;
}

.search-card-snippet {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.58;
  color: var(--text-secondary);
}

.search-card-date {
  color: var(--text-tertiary);
}

@media (width <= 768px) {
  .search-card {
    padding: 12px 24px;
  }
}
</style>
