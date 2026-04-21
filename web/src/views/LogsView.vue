<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";

import { ApiError, listLogs } from "@/api/client";
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import PillTabs from "@/components/ui/PillTabs.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import { useSessionStore } from "@/stores/session";

const { t } = useI18n();
const route = useRoute();
const sessionStore = useSessionStore();

const lines = ref<
  Array<{ timestamp: string; level: string; source: string; message: string }>
>([]);
const errorMessage = ref("");
const sourceFilter = ref<"stream" | "runtime" | "audit" | undefined>(undefined);
const levelFilter = ref("all");
const keywordInput = ref("");
const debouncedKeyword = ref("");
let keywordTimer: number | undefined;

useDocumentLocale();

const levelOptions = computed(() => [
  { key: "all", label: t("logs.levelAll") },
  { key: "trace", label: "TRACE" },
  { key: "debug", label: "DEBUG" },
  { key: "info", label: "INFO" },
  { key: "warn", label: "WARN" },
  { key: "error", label: "ERROR" },
]);

const sourceOptions = computed(() => [
  {
    key: "all",
    label: t("logs.filterAll"),
    title: t("logs.filterAll"),
    body: t("logs.filterAllHint"),
  },
  {
    key: "stream",
    label: t("logs.filterStream"),
    title: t("logs.filterStream"),
    body: t("logs.filterStreamHint"),
  },
  {
    key: "runtime",
    label: t("logs.filterRuntime"),
    title: t("logs.filterRuntime"),
    body: t("logs.filterRuntimeHint"),
  },
  {
    key: "audit",
    label: t("logs.filterAudit"),
    title: t("logs.filterAudit"),
    body: t("logs.filterAuditHint"),
  },
]);

const selectedSourceMeta = computed(
  () =>
    sourceOptions.value.find((item) => item.key === (sourceFilter.value ?? "all")) ??
    sourceOptions.value[0],
);

const filteredLines = computed(() => {
  const keyword = debouncedKeyword.value.trim().toLowerCase();

  return lines.value.filter((line) => {
    const matchesLevel =
      levelFilter.value === "all" ||
      line.level.toLowerCase() === levelFilter.value;

    if (!matchesLevel) {
      return false;
    }

    if (!keyword) {
      return true;
    }

    const haystack =
      `${line.timestamp} ${line.level} ${line.source} ${line.message}`.toLowerCase();
    return haystack.includes(keyword);
  });
});

watch(keywordInput, (value) => {
  if (keywordTimer !== undefined) {
    window.clearTimeout(keywordTimer);
  }

  keywordTimer = window.setTimeout(() => {
    debouncedKeyword.value = value;
  }, 160);
});

onMounted(async () => {
  await loadLogs();
});

async function loadLogs() {
  if (!sessionStore.isAdmin) {
    return;
  }

  try {
    const response = await listLogs({
      limit: 50,
      source: sourceFilter.value,
    });
    lines.value = response.items;
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.logsLoadFailed");
  }
}

function selectFilter(key: string) {
  sourceFilter.value =
    key === "all" ? undefined : (key as "stream" | "runtime" | "audit");
  loadLogs();
}
</script>

<template>
  <AppWorkspaceShell
    :body="t('logs.body')"
    :eyebrow="t('logs.eyebrow')"
    :title="t('logs.title')"
  >
    <section class="logs-view">
      <GlassPanel class="logs-toolbar">
        <div class="logs-toolbar__head">
          <div class="logs-toolbar__copy">
            <p class="section-label">{{ t("logs.consoleLabel") }}</p>
            <h2>{{ t("logs.consoleTitle") }}</h2>
            <p>{{ t("logs.consoleBody") }}</p>
          </div>

          <span class="logs-toolbar__badge">{{ t("common.adminOnly") }}</span>
        </div>

        <div
          v-if="route.query.access !== 'forbidden' && sessionStore.isAdmin"
          class="logs-toolbar__filters"
        >
          <div class="logs-toolbar__tabs">
            <PillTabs
              :active-key="sourceFilter ?? 'all'"
              :items="sourceOptions.map(({ key, label }) => ({ key, label }))"
              @select="selectFilter"
            />
          </div>
          <div class="logs-toolbar__source-note">
            <p class="section-label">{{ t("logs.sourceLabel") }}</p>
            <strong>{{ selectedSourceMeta.title }}</strong>
            <p>{{ selectedSourceMeta.body }}</p>
          </div>

          <label class="logs-toolbar__field">
            <span>{{ t("logs.levelLabel") }}</span>
            <select v-model="levelFilter">
              <option
                v-for="option in levelOptions"
                :key="option.key"
                :value="option.key"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="logs-toolbar__field logs-toolbar__field--keyword">
            <span>{{ t("logs.keywordLabel") }}</span>
            <input
              v-model="keywordInput"
              :placeholder="t('logs.keywordPlaceholder')"
              type="text"
            />
          </label>
        </div>
      </GlassPanel>

      <GlassPanel
        v-if="route.query.access === 'forbidden' || !sessionStore.isAdmin"
        class="logs-state"
        tone="warm"
      >
        <p class="section-label">{{ t("common.forbidden") }}</p>
        <h3>{{ t("logs.forbiddenTitle") }}</h3>
        <p>{{ t("logs.forbiddenBody") }}</p>
      </GlassPanel>

      <GlassPanel v-else-if="errorMessage" class="logs-state" tone="warm">
        <p class="section-label">{{ t("common.forbidden") }}</p>
        <h3>{{ t("logs.errorTitle") }}</h3>
        <p>{{ errorMessage }}</p>
      </GlassPanel>

      <GlassPanel v-else-if="filteredLines.length" class="logs-console-panel">
        <div class="logs-console" role="log" aria-live="polite">
          <div
            v-for="(line, index) in filteredLines"
            :key="`${line.timestamp}-${line.source}-${index}`"
            class="logs-console__line"
          >
            <span class="logs-console__timestamp">{{ line.timestamp }}</span>
            <span
              class="logs-console__level"
              :class="`logs-console__level--${line.level.toLowerCase()}`"
            >
              {{ line.level.toUpperCase() }}
            </span>
            <span class="logs-console__source">{{ line.source }}</span>
            <span class="logs-console__message">{{ line.message }}</span>
          </div>
        </div>
      </GlassPanel>

      <GlassPanel v-else class="logs-state">
        <p class="section-label">
          {{
            keywordInput || levelFilter !== "all"
              ? t("logs.filteredLabel")
              : t("common.noLogs")
          }}
        </p>
        <h3>
          {{
            keywordInput || levelFilter !== "all"
              ? t("logs.filteredTitle")
              : t("logs.emptyTitle")
          }}
        </h3>
        <p>
          {{
            keywordInput || levelFilter !== "all"
              ? t("logs.filteredBody")
              : t("logs.emptyBody")
          }}
        </p>
      </GlassPanel>
    </section>
  </AppWorkspaceShell>
</template>

<style scoped>
.logs-view {
  display: grid;
  gap: 1rem;
  min-width: 0;
}

.logs-toolbar,
.logs-state,
.logs-console-panel {
  padding: 1.2rem;
  min-width: 0;
}

.logs-toolbar {
  display: grid;
  gap: 1.1rem;
}

.logs-toolbar__head {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: start;
}

.logs-toolbar__copy h2,
.logs-toolbar__copy p,
.logs-state h3,
.logs-state p {
  margin: 0;
  overflow-wrap: anywhere;
}

.logs-toolbar__copy h2 {
  margin-top: 0.45rem;
  font-size: clamp(1.8rem, 2.3vw, 2.4rem);
  line-height: 1.12;
  font-weight: 500;
}

.logs-toolbar__copy p:last-child,
.logs-state p:last-child {
  margin-top: 0.85rem;
  color: var(--text-muted);
  line-height: 1.7;
}

.logs-toolbar__filters {
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(14rem, 0.6fr) minmax(
      16rem,
      0.85fr
    );
  gap: 0.9rem 1rem;
  align-items: start;
}

.logs-toolbar__tabs {
  grid-column: 1 / -1;
  min-width: 0;
}

.logs-toolbar__source-note {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
  min-height: 100%;
  padding: 0.85rem 0.95rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-surface-strong) 84%, transparent);
}

.logs-toolbar__source-note strong,
.logs-toolbar__source-note p {
  margin: 0;
}

.logs-toolbar__source-note strong {
  color: var(--text-main);
  font-size: 0.92rem;
}

.logs-toolbar__source-note p:last-child {
  color: var(--text-muted);
  line-height: 1.6;
}

.logs-toolbar__field {
  display: grid;
  gap: 0.4rem;
  min-width: 0;
}

.logs-toolbar__field--keyword {
  min-width: 0;
}

.logs-toolbar__field span {
  color: var(--text-faint);
  font-size: 0.78rem;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.logs-toolbar__badge {
  display: inline-flex;
  min-height: 2rem;
  align-items: center;
  padding: 0.2rem 0.72rem;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--signal-warm) 14%, transparent);
  color: var(--signal-warm);
  font-size: 0.78rem;
  font-weight: 700;
}

.logs-state {
  display: grid;
  gap: 0.6rem;
  min-height: 12rem;
  align-content: center;
}

.logs-state h3 {
  font-size: 1.3rem;
  line-height: 1.12;
  letter-spacing: -0.03em;
}

.logs-console {
  width: 100%;
  min-height: 34rem;
  max-height: calc(100vh - 18rem);
  margin: 0;
  overflow: auto;
  padding: 1.1rem;
  border-radius: var(--radius-md);
  background: var(--code-bg);
  color: var(--code-fg);
  font-family: var(--mono-font);
  font-size: 0.9rem;
  overflow-wrap: anywhere;
  word-break: break-word;
  line-height: 1.64;
  border: 1px solid
    color-mix(in srgb, var(--code-accent) 14%, var(--border-subtle));
}

.logs-console__line {
  display: grid;
  grid-template-columns: auto auto auto minmax(0, 1fr);
  gap: 0.7rem;
  align-items: start;
}

.logs-console__line + .logs-console__line {
  margin-top: 0.5rem;
}

.logs-console__timestamp {
  color: var(--code-muted);
}

.logs-console__level {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 4.5rem;
  min-height: 1.7rem;
  padding-inline: 0.55rem;
  border-radius: 999px;
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.logs-console__level--trace {
  background: color-mix(in srgb, var(--code-muted) 18%, transparent);
  color: var(--code-muted);
}

.logs-console__level--debug {
  background: color-mix(in srgb, var(--code-title) 18%, transparent);
  color: var(--code-title);
}

.logs-console__level--info {
  background: color-mix(in srgb, var(--signal-green) 18%, transparent);
  color: var(--signal-green);
}

.logs-console__level--warn {
  background: color-mix(in srgb, var(--signal-warm) 18%, transparent);
  color: var(--signal-warm);
}

.logs-console__level--error {
  background: color-mix(in srgb, var(--signal-red) 18%, transparent);
  color: var(--signal-red);
}

.logs-console__source {
  color: var(--code-accent);
}

.logs-console__message {
  min-width: 0;
  color: var(--code-fg);
}

@media (max-width: 920px) {
  .logs-toolbar,
  .logs-state,
  .logs-console-panel {
    padding: 1rem;
  }

  .logs-toolbar__head {
    flex-direction: column;
  }

  .logs-toolbar__filters {
    grid-template-columns: 1fr;
    width: 100%;
  }

  .logs-toolbar__tabs {
    grid-column: auto;
  }

  .logs-console {
    min-height: 22rem;
    max-height: calc(100vh - 15rem);
    padding: 0.95rem;
    font-size: 0.82rem;
  }

  .logs-console__line {
    grid-template-columns: 1fr;
    gap: 0.28rem;
  }
}
</style>
