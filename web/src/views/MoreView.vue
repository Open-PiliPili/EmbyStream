<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import { useSessionStore } from "@/stores/session";

const { t } = useI18n();
const router = useRouter();
const sessionStore = useSessionStore();

useDocumentLocale();

const groups = computed(() => {
  const workspace = [
    {
      key: "account",
      icon: "ph:user-circle",
      label: t("nav.account"),
      action: () => router.push({ name: "account", query: { from: "more" } }),
    },
    {
      key: "docs",
      icon: "ph:book-open-text",
      label: t("nav.docs"),
      action: () => router.push({ name: "docs", query: { from: "more" } }),
    },
    {
      key: "settings",
      icon: "ph:gear-six",
      label: t("nav.settings"),
      action: () => router.push({ name: "settings", query: { from: "more" } }),
    },
  ];

  const system = sessionStore.isAdmin
    ? [
        {
          key: "logs",
          icon: "ph:receipt",
          label: t("nav.logs"),
          action: () => router.push({ name: "logs", query: { from: "more" } }),
        },
        {
          key: "users",
          icon: "ph:users-three",
          label: t("nav.users"),
          action: () => router.push({ name: "users", query: { from: "more" } }),
        },
      ]
    : [];

  const info = [
    {
      key: "about",
      icon: "ph:seal-check",
      label: t("account.aboutOpen"),
      action: () => router.push({ name: "about", query: { from: "more" } }),
    },
    {
      key: "disclaimer",
      icon: "ph:warning-diamond",
      label: t("common.disclaimer"),
      action: () => router.push({ name: "disclaimer", query: { from: "more" } }),
    },
  ];

  return [
    { key: "workspace", title: t("common.workspace"), items: workspace },
    ...(system.length
      ? [{ key: "system", title: t("common.system"), items: system }]
      : []),
    { key: "info", title: t("common.support"), items: info },
  ];
});
</script>

<template>
  <AppWorkspaceShell
    :body="t('more.body')"
    :eyebrow="t('more.eyebrow')"
    :title="t('more.title')"
  >
    <section class="more-stack">
      <section v-for="group in groups" :key="group.key" class="more-section">
        <p class="section-label">{{ group.title }}</p>

        <GlassPanel class="more-group">
          <button
            v-for="item in group.items"
            :key="item.key"
            class="more-row"
            type="button"
            @click="item.action()"
          >
            <span class="more-row__icon">
              <Icon :icon="item.icon" width="22" />
            </span>
            <span class="more-row__label">{{ item.label }}</span>
            <Icon class="more-row__chevron" icon="ph:caret-right" width="18" />
          </button>
        </GlassPanel>
      </section>
    </section>
  </AppWorkspaceShell>
</template>

<style scoped>
.more-stack {
  display: grid;
  gap: 1rem;
}

.more-section {
  display: grid;
  gap: 0.75rem;
}

.more-group {
  display: grid;
  gap: 0;
  padding: 0.3rem;
  overflow: hidden;
}

.more-row {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  min-height: 4rem;
  border: 0;
  border-radius: 1rem;
  background: transparent;
  box-shadow: none;
}

.more-row + .more-row {
  border-top: 1px solid color-mix(in srgb, var(--border-strong) 80%, transparent);
  border-top-left-radius: 0;
  border-top-right-radius: 0;
}

.more-row__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.7rem;
  height: 2.7rem;
  border-radius: 1rem;
  border: 1px solid var(--border-strong);
  background: color-mix(in srgb, var(--bg-soft) 74%, transparent);
  color: var(--text-main);
  flex-shrink: 0;
}

.more-row__label {
  flex: 1;
  text-align: left;
  color: var(--text-main);
  font-size: 1rem;
  font-weight: 700;
}

.more-row__chevron {
  color: var(--text-faint);
}
</style>
