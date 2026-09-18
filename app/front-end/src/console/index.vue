<!-- @/console/index.vue -->
<template>
  <header-bar :title="t('console')" />

  <div class="content-layout">
    <header-section
      :title="t('console')"
      description="管理您的站點並驗證 DNS 所有權，以啟用 SPP 自動收錄。"
      :action-text="`+ ${t('console.domain.add')}`"
      @action="showAddModal = true"
    />

    <state-display
      v-if="isLoading"
      :type="status"
      :not-found-title="t('content.notFound')"
      :not-found-message="t('content.notFound.desc')"
    />

    <div v-else-if="isRetrieved" class="item-list">
      <domain-card
        v-for="item in domains"
        :key="item.domain"
        :item="item"
        :is-verifying="isVerifying"
        @verify="handleVerify"
        @delete="handleDelete"
      />
    </div>

    <div v-else class="empty-state">
      <p>目前尚未添加任何站點。</p>
      <button class="empty-add-btn" @click="showAddModal = true">
        立即添加第一個站點
      </button>
    </div>

    <add-domain-modal
      v-model:visible="showAddModal"
      :loading="isAdding"
      @submit="handleAddDomain"
    />
  </div>
</template>

<script setup lang="ts">
import DomainCard from './component/domain/card.vue';
import AddDomainModal from './component/domain/modal.vue';

import { ref } from 'vue';
import { useLocale } from '@fuyeor/locale';
import { HeaderBar, HeaderSection, StateDisplay } from '@fuyeor/interactify';
import {
  useAddDomainMutation,
  useDeleteDomainMutation,
  useDomainsQuery,
  useVerifyDomainMutation,
} from './composable/useDomain';

const { t } = useLocale();
const { data: domains, isLoading, status, isRetrieved } = useDomainsQuery();
const { mutate: addDomain, isPending: isAdding } = useAddDomainMutation();
const { mutate: verifyDomain, isPending: isVerifying } =
  useVerifyDomainMutation();
const { mutate: deleteDomain } = useDeleteDomainMutation();

const showAddModal = ref(false);

// 提交新增站点
function handleAddDomain(domain: string) {
  addDomain(
    { domain },
    {
      onSuccess: () => {
        showAddModal.value = false;
      },
      onError: (err: any) => {
        window.alert(`新增失敗: ${err.message}`);
      },
    },
  );
}

// 触发 DNS TXT 验证
function handleVerify(domain: string) {
  verifyDomain(domain, {
    onSuccess: (result) => {
      window.alert(result.message);
    },
    onError: (err: any) => {
      window.alert(`驗證失敗: ${err.message}`);
    },
  });
}

// 删除站点
function handleDelete(domain: string) {
  deleteDomain(domain, {
    onError: (err: any) => {
      window.alert(`刪除失敗: ${err.message}`);
    },
  });
}
</script>

<style scoped>
.empty-state {
  text-align: center;
  padding: 48px 0;
  color: #9ca3af;
}

.empty-state p {
  margin-bottom: 16px;
}

.empty-add-btn {
  background: #10b981;
  color: #ffffff;
  padding: 10px 20px;
  border-radius: 8px;
  border: none;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.2s;
}

.empty-add-btn:hover {
  opacity: 0.9;
}
</style>
