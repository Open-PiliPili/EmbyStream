<script setup lang="ts">
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import { DOC_SNIPPETS, EXTERNAL_LINKS } from "@/constants/app";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

useDocumentLocale();

const readmeUrl = EXTERNAL_LINKS.readme;
const nginxGuideUrl = EXTERNAL_LINKS.issueNew;
const nginxConfig = DOC_SNIPPETS.nginxProxyExample;
</script>

<template>
  <AppWorkspaceShell
    :body="t('docs.body')"
    :eyebrow="t('docs.eyebrow')"
    :title="t('docs.title')"
  >
    <section class="docs-grid">
      <GlassPanel class="docs-card">
        <p class="section-label">{{ t("docs.readmeLabel") }}</p>
        <h2>{{ t("docs.readmeTitle") }}</h2>
        <p>{{ t("docs.readmeBody") }}</p>
        <a :href="readmeUrl" rel="noopener noreferrer" target="_blank">{{
          t("docs.openLink")
        }}</a>
      </GlassPanel>

      <GlassPanel class="docs-card docs-card--wide">
        <p class="section-label">{{ t("docs.nginxLabel") }}</p>
        <h2>{{ t("docs.nginxTitle") }}</h2>
        <p>{{ t("docs.nginxBody") }}</p>
        <pre class="docs-card__code"><code>{{ nginxConfig }}</code></pre>
        <a :href="nginxGuideUrl" rel="noopener noreferrer" target="_blank">{{
          t("docs.issueLink")
        }}</a>
      </GlassPanel>
    </section>
  </AppWorkspaceShell>
</template>

<style scoped>
.docs-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
}

.docs-card {
  display: grid;
  gap: 1rem;
  padding: 1.25rem;
}

.docs-card--wide {
  grid-column: span 2;
}

.docs-card h2,
.docs-card p {
  margin: 0;
}

.docs-card h2 {
  font-size: 1.35rem;
  line-height: 1.14;
  font-weight: 500;
}

.docs-card p {
  color: var(--text-muted);
  line-height: 1.7;
}

.docs-card a {
  display: inline-flex;
  width: fit-content;
  min-height: 2.4rem;
  align-items: center;
  padding: 0.5rem 0.9rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-pill);
  background: var(--button-secondary-bg);
  box-shadow: var(--shadow-soft);
}

.docs-card__code {
  margin: 0;
  padding: 1rem;
  border: 1px solid
    color-mix(in srgb, var(--code-accent) 14%, var(--border-subtle));
  border-radius: var(--radius-md);
  background: var(--code-bg);
  color: var(--code-fg);
  font-family: var(--mono-font);
  white-space: pre-wrap;
}

@media (max-width: 900px) {
  .docs-grid {
    grid-template-columns: 1fr;
  }

  .docs-card--wide {
    grid-column: auto;
  }
}
</style>
