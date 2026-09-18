<!-- @/console/component/domain/modal.vue -->
<template>
  <modal
    size="small"
    :model-value="visible"
    @update:model-value="handleUpdateVisible"
  >
    <template #header>
      <h3>{{ t('console.domain.add') }}</h3>
    </template>

    <div class="add-domain-form">
      <span class="modal-desc">{{ t('console.domain.add.desc') }}</span>

      <form-field>
        <input
          ref="inputRef"
          v-model="domainInput"
          placeholder="example.com"
          class="modal-input"
          :disabled="loading"
          @keyup.enter="handleSubmit"
        />
      </form-field>

      <div class="modal-actions">
        <button
          type="button"
          class="btn-secondary"
          :disabled="loading"
          @click="close"
        >
          {{ t('cancel') }}
        </button>
        <button
          type="button"
          class="btn-primary"
          :disabled="loading || !domainInput.trim()"
          @click="handleSubmit"
        >
          {{ loading ? '添加中...' : '確認新增' }}
        </button>
      </div>
    </div>
  </modal>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useLocale } from '@fuyeor/locale';
import { Modal, FormField } from '@fuyeor/interactify';

const props = defineProps<{
  visible: boolean;
  loading?: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void;
  (e: 'submit', domain: string): void;
}>();

const { t } = useLocale();
const domainInput = ref('');
const inputRef = ref<HTMLInputElement | null>(null);

// 弹窗打开时自动聚焦并清空输入框
watch(
  () => props.visible,
  (val) => {
    if (val) {
      domainInput.value = '';
      nextTick(() => {
        inputRef.value?.focus();
      });
    }
  },
);

const handleUpdateVisible = (val: boolean) => {
  emit('update:visible', val);
};

const close = () => {
  emit('update:visible', false);
};

const handleSubmit = () => {
  const trimmed = domainInput.value.trim();
  if (!trimmed || props.loading) return;
  emit('submit', trimmed);
};
</script>

<style scoped>
.add-domain-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.modal-desc {
  color: var(--text-tertiary);
}

.modal-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
