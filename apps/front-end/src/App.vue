<!-- @/App.vue -->
<template>
  <left-sidebar />

  <main>
    <router-view />
  </main>

  <back-top />

  <!-- toast notifications -->
  <toast-provider position="bottom-right" />
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useHeadManager } from '@fuyeor/commons';
import {
  useFontLoader,
  useMobileDetection,
  BackTop,
  ToastProvider,
} from '@fuyeor/interactify';

// 启用字体加载
useFontLoader();
// 启动全局标题管理器
useHeadManager({
  nameKey: 'site.name',
  titleKey: 'site.title',
});
</script>
