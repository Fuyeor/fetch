<!-- @/console/component/domain/card.vue -->
<template>
  <div
    :class="['domain-card', { 'is-clickable': isVerified }]"
    @click="handleCardClick"
  >
    <div class="card-header">
      <span class="domain-name">{{ item.domain }}</span>
      <span :class="['status-badge', item.status]">
        {{
          isVerified
            ? t('console.domain.verified')
            : t('console.domain.unverified')
        }}
      </span>
    </div>

    <!-- 待验证状态下的 DNS TXT 提示引导 -->
    <div v-if="item.status === 'pending'" class="dns-guide">
      <div class="guide-title">
        請前往您的 DNS 服務商（如 Cloudflare）新增以下 TXT 記錄：
      </div>

      <div class="dns-row">
        <span class="dns-label">記錄類型：</span>
        <code>TXT</code>
      </div>
      <div class="dns-row">
        <span class="dns-label">主機名 (Host)：</span>
        <code>{{ item.dnsRecordName }}</code>
        <button class="copy-btn" @click.stop="copyText(item.dnsRecordName)">
          複製
        </button>
      </div>
      <div class="dns-row">
        <span class="dns-label">記錄值 (Value)：</span>
        <code>{{ item.dnsRecordValue }}</code>
        <button class="copy-btn" @click.stop="copyText(item.dnsRecordValue)">
          複製
        </button>
      </div>

      <div class="card-actions">
        <button
          class="action-btn primary"
          :disabled="isVerifying"
          @click.stop="emit('verify', item.domain)"
        >
          {{ isVerifying ? '驗證中...' : '立即驗證所有權' }}
        </button>
        <button class="action-btn danger" @click.stop="showConfirmModal = true">
          {{ t('delete') }}
        </button>
      </div>
    </div>

    <!-- 已验证状态 -->
    <div v-else class="verified-info">
      <p>
        {{
          t('console.domain.verified.at', {
            date: new Date(item.verifiedAt!).toLocaleString(),
          })
        }}
      </p>
      <div class="card-actions">
        <button class="action-btn danger" @click.stop="showConfirmModal = true">
          {{ t('console.domain.delete.title') }}
        </button>
      </div>
    </div>
  </div>

  <confirm-modal
    v-model="showConfirmModal"
    :title="t('console.domain.delete.title')"
    :message="t('console.domain.delete.desc', { domain: item.domain })"
    :confirm-text="t('delete.confirm')"
    confirm-button-type="danger"
    @confirm="handleConfirmDelete"
  />
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRouter } from '@fuyeor/vue-router';
import { useLocale } from '@fuyeor/locale';
import { ConfirmModal } from '@fuyeor/interactify';
import { useCopy } from '@app/composable/useCopy';
import type { Domain } from '@/console/type';

const { item, isVerifying } = defineProps<{
  item: Domain;
  isVerifying?: boolean;
}>();

const emit = defineEmits<{
  (e: 'verify', domain: string): void;
  (e: 'delete', domain: string): void;
}>();

const router = useRouter();

const { t } = useLocale();
const { copyText } = useCopy();

const isVerified = computed(() => item.status === 'verified');

// 控制确认弹窗的显示/隐藏
const showConfirmModal = ref(false);

// 确认删除时触发
const handleConfirmDelete = () => {
  showConfirmModal.value = false;
  emit('delete', item.domain);
};

const handleCardClick = () => {
  if (!isVerified.value) return;
  router.push({
    name: 'Console.Domain.Overview',
    params: { domain: item.domain },
  });
};
</script>

<style scoped>
.domain-card {
  background: rgba(255, 255, 255, 0.03);
  border-radius: var(--radius-lg);
  padding: 18px 24px;
  transition: background 0.3s ease;

  &.is-clickable {
    cursor: pointer;
  }
  &.is-clickable:hover {
    background: var(--surface-hover);
  }
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.domain-name {
  font-size: 1.25rem;
  font-weight: 600;
}

.status-badge {
  font-size: 0.75rem;
  padding: 4px 10px;
  border-radius: 9999px;
  font-weight: 600;
}

.status-badge.verified {
  background: rgba(16, 185, 129, 0.2);
  color: #34d399;
}

.status-badge.pending {
  background: rgba(245, 158, 11, 0.2);
  color: #fbbf24;
}

.dns-guide {
  background: rgba(0, 0, 0, 0.25);
  border-radius: 8px;
  padding: 16px;
  margin-top: 12px;
}

.guide-title {
  font-size: 0.875rem;
  color: #d1d5db;
  margin-bottom: 12px;
}

.dns-row {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 0.85rem;
  margin-bottom: 8px;
}

.dns-label {
  width: 140px;
  color: #9ca3af;
}

code {
  background: rgba(255, 255, 255, 0.06);
  padding: 2px 8px;
  border-radius: 4px;
  color: #a7f3d0;
  font-family: monospace;
}

.copy-btn {
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #d1d5db;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.75rem;
}

.verified-info {
  margin-top: 12px;
  color: #9ca3af;
  font-size: 0.9rem;
}

.card-actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;
}
</style>
