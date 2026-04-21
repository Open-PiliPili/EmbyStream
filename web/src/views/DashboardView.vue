<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";

import { ApiError, getSystemMetrics } from "@/api/client";
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import type { SystemMetricsResponse } from "@/api/types";

const { t } = useI18n();

const metrics = ref<SystemMetricsResponse | null>(null);
const loading = ref(true);
const refreshing = ref(false);
const errorMessage = ref("");
const lastUpdatedAt = ref<Date | null>(null);
let refreshTimer: number | undefined;

useDocumentLocale();

const cards = computed(() => {
  if (!metrics.value) {
    return [];
  }

  return [
    {
      key: "cpu",
      icon: "ph:cpu",
      label: t("dashboard.cpu"),
      value: `${metrics.value.cpu_usage_percent.toFixed(2)}%`,
      meta: t("dashboard.cpuMeta", { count: metrics.value.cpu_core_count }),
    },
    {
      key: "memory",
      icon: "ph:memory",
      label: t("dashboard.memory"),
      value: `${metrics.value.memory_usage_percent.toFixed(2)}%`,
      meta: t("dashboard.memoryMeta", {
        total: formatSingleBytes(metrics.value.memory_total_bytes),
      }),
    },
    {
      key: "disk",
      icon: "ph:hard-drives",
      label: t("dashboard.disk"),
      value: `${metrics.value.disk_usage_percent.toFixed(2)}%`,
      meta: t("dashboard.diskMeta", {
        total: formatSingleBytes(metrics.value.disk_total_bytes),
      }),
    },
    {
      key: "uptime",
      icon: "ph:timer",
      label: t("dashboard.uptime"),
      value: formatUptime(metrics.value.uptime_seconds),
    },
  ];
});

onMounted(async () => {
  await fetchMetrics(true);
  refreshTimer = window.setInterval(() => {
    fetchMetrics();
  }, 15_000);
});

onBeforeUnmount(() => {
  if (refreshTimer !== undefined) {
    window.clearInterval(refreshTimer);
  }
});

const lastUpdatedLabel = computed(() => {
  if (!lastUpdatedAt.value) {
    return "";
  }

  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(lastUpdatedAt.value);
});

async function fetchMetrics(initial = false) {
  if (initial) {
    loading.value = true;
  } else {
    refreshing.value = true;
  }

  try {
    metrics.value = await getSystemMetrics();
    lastUpdatedAt.value = new Date();
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("dashboard.errorBody");
  } finally {
    loading.value = false;
    refreshing.value = false;
  }
}

function formatSingleBytes(value: number) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let nextValue = value;
  let unitIndex = 0;

  while (nextValue >= 1024 && unitIndex < units.length - 1) {
    nextValue /= 1024;
    unitIndex += 1;
  }

  const digits = unitIndex === 0 ? 0 : 1;
  return `${nextValue.toFixed(digits)}${units[unitIndex]}`;
}

function formatUptime(seconds: number) {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h`;
  }

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  return `${minutes}m`;
}
</script>

<template>
  <AppWorkspaceShell
    :body="t('dashboard.body')"
    :eyebrow="t('dashboard.eyebrow')"
    :title="t('dashboard.title')"
  >
    <section class="dashboard-toolbar">
      <GlassPanel class="dashboard-toolbar__card">
        <div>
          <p class="section-label">{{ t("dashboard.statusLabel") }}</p>
          <h2>{{ t("dashboard.refreshTitle") }}</h2>
          <p>
            {{ t("dashboard.refreshBody", { time: lastUpdatedLabel || "—" }) }}
          </p>
          <p class="dashboard-toolbar__hint">
            {{ t("dashboard.refreshHint") }}
          </p>
        </div>
        <button class="dashboard-toolbar__refresh" type="button" @click="fetchMetrics()">
          <Icon
            :icon="refreshing ? 'ph:spinner-gap' : 'ph:arrow-clockwise'"
            width="18"
          />
        </button>
      </GlassPanel>
    </section>

    <section class="dashboard-grid">
      <GlassPanel v-if="loading" class="dashboard-card dashboard-card--state">
        <p class="section-label">{{ t("common.loading") }}</p>
        <h2>{{ t("dashboard.loadingTitle") }}</h2>
        <p>{{ t("dashboard.loadingBody") }}</p>
      </GlassPanel>

      <GlassPanel
        v-else-if="errorMessage"
        class="dashboard-card dashboard-card--state"
        tone="warm"
      >
        <p class="section-label">{{ t("dashboard.statusLabel") }}</p>
        <h2>{{ t("dashboard.errorTitle") }}</h2>
        <p>{{ errorMessage }}</p>
      </GlassPanel>

      <GlassPanel
        v-for="card in cards"
        v-else
        :key="card.key"
        class="dashboard-card"
      >
        <div class="dashboard-card__head">
          <div class="dashboard-card__head-main">
            <Icon :icon="card.icon" width="20" />
            <span>{{ card.label }}</span>
          </div>
          <span v-if="card.meta" class="dashboard-card__head-meta">{{
            card.meta
          }}</span>
        </div>
        <strong>{{ card.value }}</strong>
      </GlassPanel>
    </section>
  </AppWorkspaceShell>
</template>

<style scoped>
.dashboard-toolbar {
  margin-bottom: 1rem;
}

.dashboard-toolbar__card {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: start;
  padding: 1.25rem;
}

.dashboard-toolbar__card h2,
.dashboard-toolbar__card p {
  margin: 0;
}

.dashboard-toolbar__card h2 {
  margin-top: 0.45rem;
  font-size: 1.25rem;
  line-height: 1.14;
  font-weight: 500;
}

.dashboard-toolbar__refresh {
  min-width: 2.9rem;
  min-height: 2.9rem;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.dashboard-toolbar__card p:last-child {
  margin-top: 0.7rem;
  color: var(--text-muted);
}

.dashboard-toolbar__hint {
  margin-top: 0.7rem;
  color: var(--text-faint);
  font-size: 0.88rem;
  line-height: 1.6;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
  gap: 1rem;
}

.dashboard-card {
  display: grid;
  gap: 0.9rem;
  padding: 1.25rem;
  transition:
    background-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    transform 220ms var(--curve-buoy);
}

.dashboard-card__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.6rem;
  min-height: 1.5rem;
  color: var(--text-muted);
  font-size: 0.88rem;
  font-weight: 600;
}

.dashboard-card__head-main {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.dashboard-card__head-meta {
  text-align: right;
  color: var(--text-faint);
  font-size: 0.8rem;
  font-weight: 600;
}

.dashboard-card strong,
.dashboard-card p,
.dashboard-card h2 {
  margin: 0;
}

.dashboard-card strong {
  color: var(--text-main);
  font-size: clamp(1.9rem, 3vw, 2.5rem);
  line-height: 1;
  font-weight: 700;
}

.dashboard-card p {
  color: var(--text-muted);
}

.dashboard-card--state {
  grid-column: 1 / -1;
}

@media (pointer: fine) {
  .dashboard-card:hover {
    background: var(--bg-surface-strong);
    border-color: var(--border-strong);
    box-shadow: var(--card-hover-shadow);
    transform: translateY(-2px);
  }
}

.dashboard-card:focus-within {
  background: var(--bg-surface-strong);
  border-color: var(--border-strong);
  box-shadow: var(--card-focus-shadow);
  transform: translateY(-1px);
}
</style>
