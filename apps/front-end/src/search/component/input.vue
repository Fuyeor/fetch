<!-- @/search/component/input.vue -->
<template>
  <form class="search-bar-container" @submit.prevent="handleSearch">
    <input
      required
      type="text"
      class="search-top-input"
      :value="modelValue"
      :placeholder="t('search.placeholder')"
      @input="handleInput"
    />

    <button
      type="submit"
      class="search-button"
      v-tooltip="{ text: t('search'), enableAria: true }"
    >
      <img :src="getIconUrl('search')" />
    </button>
  </form>
</template>

<script setup lang="ts">
import { useLocale } from '@fuyeor/locale';
import { getIconUrl } from '@fuyeor/commons';

const { modelValue } = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
  (e: 'search', value: string): void;
}>();

const { t } = useLocale();

const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  emit('update:modelValue', target.value);
};

const handleSearch = () => {
  const trimmed = modelValue.trim();
  if (!trimmed) return;
  emit('search', trimmed);
};
</script>

<style scoped>
.search-bar-container input {
  padding: 14px 20px;
  border-radius: 24px;
}

.search-top-btn {
  padding: 10px 20px;
  font-size: 14px;
  font-weight: 500;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  white-space: nowrap;
  transition: background-color 0.2s;
}
</style>
