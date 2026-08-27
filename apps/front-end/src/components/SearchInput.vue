<!-- @/components/SearchInput.vue -->
<template>
  <div class="search-input-wrapper">
    <input
      ref="inputRef"
      type="text"
      :value="modelValue"
      class="search-input"
      @input="handleInput"
      @keydown="handleKeydown"
    />
    <button type="button" class="search-btn" @click="handleSearch">搜索</button>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from '@fuyeor/vue-router';

const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
  (e: 'search', value: string): void;
}>();

const router = useRouter();
const inputRef = ref<HTMLInputElement | null>(null);

const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  emit('update:modelValue', target.value);
};

const handleSearch = () => {
  const trimmed = props.modelValue.trim();
  if (!trimmed) return;

  emit('search', trimmed);
  router.push({ name: 'Search', query: { q: trimmed } });
};

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Enter') {
    handleSearch();
  }
};
</script>

<style scoped>
.search-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  max-width: 600px;
  margin: 0 auto;
}

.search-input {
  width: 100%;
  padding: 14px 100px 14px 20px;
  font-size: 16px;
  background-color: #ffffff;
  border: 1px solid #dcdfe6;
  border-radius: 9999px;
  outline: none;
  transition:
    border-color 0.2s,
    box-shadow 0.2s;
}

.search-input:focus {
  border-color: #409eff;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

.search-btn {
  position: absolute;
  right: 4px;
  padding: 10px 24px;
  color: #ffffff;
  background-color: #409eff;
  border: none;
  border-radius: 9999px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s;
}

.search-btn:hover {
  background-color: #66b1ff;
}

.search-btn:active {
  background-color: #3a8ee6;
}
</style>
