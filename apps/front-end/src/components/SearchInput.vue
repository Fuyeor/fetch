<!-- @/components/SearchInput.vue -->
<template>
  <div class="search-input-wrapper">
    <input
      ref="inputRef"
      type="text"
      class="search-input"
      :value="modelValue"
      :placeholder="t('search.placeholder')"
      @input="handleInput"
      @keydown="handleKeydown"
    />
    <button type="button" class="search-btn" @click="handleSearch">
      <img :src="getIconUrl('search')" v-tooltip="t('search.placeholder')" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from '@fuyeor/vue-router';
import { useLocale } from '@fuyeor/locale';
import { getIconUrl } from '@fuyeor/commons';

const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
  (e: 'search', value: string): void;
}>();

const router = useRouter();

const { t } = useLocale();

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
}

.search-input {
  width: 100%;
  border: none;
  border-radius: 32px;
  padding: 18px 100px 18px 20px;
  color: var(--text-secondary);
  font-size: 0.95rem;
  border: var(--border-subtle);
  background-color: var(--surface-raised);
  transition:
    border-color 0.2s,
    box-shadow 0.2s;

  &:focus {
    box-shadow: var(--input-border-shadow);
  }
}

.search-btn {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  cursor: pointer;
  transition: background-color 0.2s;

  img {
    width: 1.8rem;
  }
}

@media (width <= 768px) {
  .search-input-wrapper {
    width: 90%;
  }

  .search-input {
    padding: 16px 100px 16px 20px;
  }
}
</style>
