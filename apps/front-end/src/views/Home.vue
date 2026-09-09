<!-- @/views/Home.vue -->
<template>
  <div class="home-content">
    <SearchInput v-model="searchQuery" @search="onSearch" />
  </div>
</template>

<script setup lang="ts">
import SearchInput from '@/components/SearchInput.vue';

import { ref } from 'vue';

const searchQuery = ref('');

const onSearch = (query: string) => {
  window.console.log('Executing search for:', query);
};
</script>

<style>
.home-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;

  min-height: 100vh;
  box-sizing: border-box;
}

/* 30% */
.home-content::before {
  content: '';
  flex: 1;
}

/* 60% */
.home-content::after {
  content: '';
  flex: 2;
}

.home-title {
  font-size: 48px;
  font-weight: 700;
  color: #1a1a1a;
  margin-bottom: 16px;
  letter-spacing: -0.025em;
}

.home-subtitle {
  font-size: 18px;
  color: #606266;
  margin-bottom: 32px;
}
</style>
