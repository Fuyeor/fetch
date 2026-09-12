<!-- @/views/Home.vue -->
<template>
  <div class="home-content">
    <div class="search-wrapper">
      <span class="title">Fetch 🔍︎</span>
      <SearchInput v-model="searchQuery" />
    </div>
  </div>
</template>

<script setup lang="ts">
import SearchInput from '@/components/SearchInput.vue';

import { ref } from 'vue';

const searchQuery = ref('');
</script>

<style>
.home-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;

  min-height: 100vh;
  box-sizing: border-box;

  /* 30% */
  &::before {
    content: '';
    flex: 1;
  }

  /* 60% */
  &::after {
    content: '';
    flex: 2;
  }

  @media (width <= 768px) {
    min-height: calc(100vh - var(--height-header));

    &::before {
      content: '';
      flex: 0.5;
    }
  }
}

.search-wrapper {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  width: 100%;
  max-width: 640px;
  padding: 0 20px;
  gap: 26px;

  @media (width <= 768px) {
    padding: 0 0 0 20px;
  }
}

.title {
  font-size: 3rem;
  font-weight: 700;
  padding-left: 10px;
  letter-spacing: -0.025em;

  @media (width <= 768px) {
    font-size: 2.5rem;
  }
}
</style>
