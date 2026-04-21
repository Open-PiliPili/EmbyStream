<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";

import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import { APP_NAME } from "@/constants/app";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";

const { t } = useI18n();
const appVersion = __APP_VERSION__;
const githubUrl = __APP_GITHUB_URL__;
const changelogUrl = __APP_CHANGELOG_URL__;

useDocumentLocale();
</script>

<template>
  <AppWorkspaceShell
    :body="t('account.aboutBody')"
    :eyebrow="t('account.aboutLabel')"
    :title="t('account.aboutTitle', { app: APP_NAME })"
  >
    <GlassPanel class="about-card" tone="warm">
      <span class="about-card__version">
        {{ t("account.aboutVersion", { version: appVersion }) }}
      </span>
      <div class="about-card__links">
        <a :href="githubUrl" rel="noopener noreferrer" target="_blank">
          <Icon icon="mdi:github" width="18" />
          <span>{{ t("account.aboutGithub") }}</span>
        </a>
        <a :href="changelogUrl" rel="noopener noreferrer" target="_blank">
          <Icon icon="ph:clock-counter-clockwise" width="18" />
          <span>{{ t("account.aboutChangelog") }}</span>
        </a>
      </div>
    </GlassPanel>
  </AppWorkspaceShell>
</template>

<style scoped>
.about-card {
  display: grid;
  gap: 1rem;
  padding: 1.1rem;
}

.about-card__version {
  display: inline-flex;
  width: fit-content;
  min-height: 2rem;
  align-items: center;
  padding: 0.2rem 0.72rem;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
  color: var(--text-main);
  font-size: 0.82rem;
  font-weight: 700;
}

.about-card__links {
  display: grid;
  gap: 0.75rem;
}

.about-card__links a {
  display: inline-flex;
  align-items: center;
  gap: 0.55rem;
  width: fit-content;
  min-height: 2.45rem;
  padding: 0.55rem 0.9rem;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: var(--button-secondary-bg);
}
</style>
