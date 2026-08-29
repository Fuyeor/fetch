<!-- @/views/Home.vue -->
<template>
  <main class="home-container">
    <div class="home-content">
      <SearchInput v-model="searchQuery" @search="onSearch" />
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import SearchInput from '@/components/SearchInput.vue';

const searchQuery = ref('');

const onSearch = (query: string) => {
  window.console.log('Executing search for:', query);
};
</script>

<style scoped>
.home-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: 0 16px;
  background-color: #f7f8fa;
}

.home-content {
  width: 100%;
  max-width: 800px;
  text-align: center;
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
