<!-- @/components/Search/Input.vue -->
<template>
  <div class="search-top-bar">
    <input
      type="text"
      :value="modelValue"
      placeholder="请输入搜索关键词..."
      class="search-top-input"
      @input="handleInput"
      @keydown.enter="handleSearch"
    />
    <button
      type="button"
      class="search-top-btn"
      @click="handleSearch"
    >
      搜索
    </button>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
  (e: 'search', value: string): void;
}>();

const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  emit('update:modelValue', target.value);
};

const handleSearch = () => {
  const trimmed = props.modelValue.trim();
  if (!trimmed) return;
  emit('search', trimmed);
};
</script>

<style scoped>
.search-top-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  max-width: 680px;
}

.search-top-input {
  flex: 1;
  padding: 10px 16px;
  font-size: 15px;
  background-color: #ffffff;
  border: 1px solid #dcdfe6;
  border-radius: 8px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.search-top-input:focus {
  border-color: #409eff;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

.search-top-btn {
  padding: 10px 20px;
  font-size: 14px;
  font-weight: 500;
  color: #ffffff;
  background-color: #409eff;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  white-space: nowrap;
  transition: background-color 0.2s;
}

.search-top-btn:hover {
  background-color: #66b1ff;
}

.search-top-btn:active {
  background-color: #3a8ee6;
}
</style>