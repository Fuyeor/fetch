<!-- @/search/component/skeleton.vue -->
<template>
  <div class="search-skeleton-card">
    <!-- 模拟顶部网站图标 + URL 域名条 -->
    <div class="skeleton-header">
      <div class="skeleton-box skeleton-favicon" />
      <div class="skeleton-box skeleton-site" />
    </div>

    <!-- 模拟搜索结果的大标题 -->
    <div class="skeleton-box skeleton-title" />

    <!-- 模拟搜索摘要（两行文本） -->
    <div class="skeleton-box skeleton-line skeleton-line-full" />
    <div class="skeleton-box skeleton-line skeleton-line-short" />
  </div>
</template>

<style>
:root {
  --skeleton-background: linear-gradient(
    90deg,
    rgba(0, 0, 0, 0.04) 25%,
    rgba(0, 0, 0, 0.1) 37%,
    rgba(0, 0, 0, 0.04) 63%
  );
}

:root[data-theme='dark'],
:root[data-theme='black'] {
  --skeleton-background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.04) 25%,
    rgba(255, 255, 255, 0.1) 37%,
    rgba(255, 255, 255, 0.04) 63%
  );
}

.search-skeleton-card {
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 核心流光块 */
.skeleton-box {
  background: var(--skeleton-background);
  background-size: 400% 100%;
  animation: skeleton-shimmer 1.4s ease infinite;
  border-radius: 6px;
}

@keyframes skeleton-shimmer {
  0% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0 50%;
  }
}

/* 模拟 Favicon 小圆点 */
.skeleton-header {
  display: flex;
  align-items: center;
  gap: 8px;
}
.skeleton-favicon {
  width: 16px;
  height: 16px;
  border-radius: 50%;
}

/* 模拟 URL 地址条 */
.skeleton-site {
  width: 140px;
  height: 14px;
}

/* 模拟大标题 */
.skeleton-title {
  width: 60%;
  height: 22px;
  margin-bottom: 4px;
}

/* 模拟正文摘要第一行（全宽） */
.skeleton-line-full {
  width: 90%;
  height: 14px;
}

/* 模拟正文摘要第二行（短宽） */
.skeleton-line-short {
  width: 45%;
  height: 14px;
}
</style>
