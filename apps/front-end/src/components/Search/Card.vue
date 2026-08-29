<!-- @/components/Search/Card.vue -->
<template>
  <article class="search-card">
    <div class="search-card-meta">
      <cite class="search-card-url">{{ item.url }}</cite>
      <time class="search-card-date">{{ formattedDate }}</time>
    </div>

    <h2 class="search-card-title">
      <a
        :href="item.url"
        target="_blank"
        rel="noopener noreferrer"
        class="search-card-link"
        v-html="item.title"
      />
    </h2>

    <div class="search-card-body">
      <img
        v-if="item.images.length > 0"
        :src="item.images[0]"
        alt="Thumbnail"
        class="search-card-thumb"
        loading="lazy"
      />
      <p class="search-card-snippet" v-html="item.snippet" />
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { SearchItem } from '@/types/search';

const props = defineProps<{
  item: SearchItem;
}>();

const formattedDate = computed(() => {
  if (!props.item.updated_at) return '';
  return new window.Date(props.item.updated_at).toLocaleDateString();
});
</script>

<style scoped>
.search-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 0;
}

.search-card-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.search-card-url {
  color: #606266;
  font-style: normal;
  max-width: 400px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-card-date {
  color: #909399;
}

.search-card-title {
  margin: 0;
  font-size: 18px;
  font-weight: 500;
  line-height: 1.4;
}

.search-card-link {
  color: #1a0dab;
  text-decoration: none;
}

.search-card-link:hover {
  text-decoration: underline;
}

.search-card-body {
  display: flex;
  gap: 12px;
  margin-top: 2px;
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
  font-size: 14px;
  line-height: 1.58;
  color: #4d5156;
  white-space: pre-line;
}
</style>