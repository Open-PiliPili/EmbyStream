<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";

import { ApiError, buildLogsStreamUrl, listLogs } from "@/api/client";
import type { LogStreamMessage } from "@/api/types";
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
const logsConsoleRef = ref<HTMLElement | null>(null);
const liveStatus = ref<"connecting" | "live" | "reconnecting" | "offline">(
  "connecting",
);
const autoFollow = ref(true);
const pendingEntriesCount = ref(0);
let keywordTimer: number | undefined;
let liveSocket: WebSocket | null = null;
let reconnectTimer: number | undefined;
let reconnectAttempts = 0;
let intentionalSocketClose = false;

const MAX_LIVE_LOGS = 300;

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
    sourceOptions.value.find(
      (item) => item.key === (sourceFilter.value ?? "all"),
    ) ?? sourceOptions.value[0],
);

const liveStatusMeta = computed(() => {
  switch (liveStatus.value) {
    case "live":
      return {
        label: t("logs.statusLive"),
        body: t("logs.statusLiveBody"),
        className: "logs-toolbar__status--live",
      };
    case "reconnecting":
      return {
        label: t("logs.statusReconnecting"),
        body: t("logs.statusReconnectingBody"),
        className: "logs-toolbar__status--reconnecting",
      };
    case "offline":
      return {
        label: t("logs.statusOffline"),
        body: t("logs.statusOfflineBody"),
        className: "logs-toolbar__status--offline",
      };
    default:
      return {
        label: t("logs.statusConnecting"),
        body: t("logs.statusConnectingBody"),
        className: "logs-toolbar__status--connecting",
      };
  }
});

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

watch(sourceFilter, async () => {
  await loadLogs();
  reconnectLiveLogs();
});

onMounted(async () => {
  await loadLogs();
  connectLiveLogs();
});

onBeforeUnmount(() => {
  closeLiveLogs(true);

  if (keywordTimer !== undefined) {
    window.clearTimeout(keywordTimer);
  }
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
    replaceLogEntries(response.items);
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.logsLoadFailed");
  }
}

function selectFilter(key: string) {
  sourceFilter.value =
    key === "all" ? undefined : (key as "stream" | "runtime" | "audit");
}

function connectLiveLogs() {
  if (
    typeof window === "undefined" ||
    !sessionStore.isAdmin ||
    route.query.access === "forbidden"
  ) {
    liveStatus.value = "offline";
    return;
  }

  intentionalSocketClose = false;
  liveStatus.value = reconnectAttempts > 0 ? "reconnecting" : "connecting";
  liveSocket = new WebSocket(
    buildLogsStreamUrl({
      source: sourceFilter.value,
      limit: 50,
    }),
  );

  liveSocket.onopen = () => {
    reconnectAttempts = 0;
    liveStatus.value = "live";
  };

  liveSocket.onmessage = (event) => {
    handleLiveMessage(event.data);
  };

  liveSocket.onerror = () => {
    liveStatus.value = "reconnecting";
    liveSocket?.close();
  };

  liveSocket.onclose = () => {
    liveSocket = null;
    if (intentionalSocketClose) {
      liveStatus.value = "offline";
      return;
    }

    scheduleReconnect();
  };
}

function reconnectLiveLogs() {
  closeLiveLogs(true);
  connectLiveLogs();
}

function closeLiveLogs(intentional: boolean) {
  intentionalSocketClose = intentional;
  if (intentional) {
    liveStatus.value = "offline";
  }

  if (reconnectTimer !== undefined) {
    window.clearTimeout(reconnectTimer);
    reconnectTimer = undefined;
  }

  if (
    liveSocket &&
    (liveSocket.readyState === WebSocket.OPEN ||
      liveSocket.readyState === WebSocket.CONNECTING)
  ) {
    liveSocket.close();
  }

  liveSocket = null;
}

function scheduleReconnect() {
  if (typeof window === "undefined") {
    return;
  }

  if (reconnectTimer !== undefined) {
    window.clearTimeout(reconnectTimer);
  }

  const delay = Math.min(1000 * 2 ** reconnectAttempts, 10000);
  reconnectAttempts += 1;
  liveStatus.value = "reconnecting";
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = undefined;
    connectLiveLogs();
  }, delay);
}

function handleLiveMessage(raw: string) {
  let payload: LogStreamMessage;

  try {
    payload = JSON.parse(raw) as LogStreamMessage;
  } catch {
    return;
  }

  if (!isLogStreamMessage(payload)) {
    return;
  }

  if (payload.kind === "replay") {
    replaceLogEntries(payload.items);
    return;
  }

  prependLogEntries([payload.item]);
}

function replaceLogEntries(nextEntries: typeof lines.value) {
  lines.value = normalizeLogEntries(nextEntries);
  if (autoFollow.value) {
    pendingEntriesCount.value = 0;
    scrollConsoleToLatest();
  }
}

function prependLogEntries(nextEntries: typeof lines.value) {
  const shouldStickTop = autoFollow.value && isConsolePinnedToTop();
  lines.value = normalizeLogEntries([...nextEntries, ...lines.value]);

  if (shouldStickTop) {
    pendingEntriesCount.value = 0;
    scrollConsoleToLatest();
  } else {
    pendingEntriesCount.value = Math.min(
      pendingEntriesCount.value + nextEntries.length,
      MAX_LIVE_LOGS,
    );
  }
}

function normalizeLogEntries(nextEntries: typeof lines.value) {
  const deduped = new Map<string, (typeof lines.value)[number]>();

  for (const entry of nextEntries) {
    deduped.set(
      `${entry.timestamp}|${entry.level}|${entry.source}|${entry.message}`,
      entry,
    );
  }

  return Array.from(deduped.values())
    .sort((left, right) => right.timestamp.localeCompare(left.timestamp))
    .slice(0, MAX_LIVE_LOGS);
}

function isConsolePinnedToTop() {
  if (!logsConsoleRef.value) {
    return true;
  }

  return logsConsoleRef.value.scrollTop <= 24;
}

function handleConsoleScroll() {
  if (!logsConsoleRef.value) {
    return;
  }

  const pinnedToTop = isConsolePinnedToTop();
  autoFollow.value = pinnedToTop;

  if (pinnedToTop) {
    pendingEntriesCount.value = 0;
  }
}

function pauseAutoFollow() {
  autoFollow.value = false;
}

function resumeAutoFollow() {
  autoFollow.value = true;
  pendingEntriesCount.value = 0;
  scrollConsoleToLatest();
}

function scrollConsoleToLatest() {
  nextTick(() => {
    if (logsConsoleRef.value) {
      logsConsoleRef.value.scrollTop = 0;
    }
  });
}

function isLogEntryPayload(
  value: unknown,
): value is (typeof lines.value)[number] {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.timestamp === "string" &&
    typeof candidate.level === "string" &&
    typeof candidate.source === "string" &&
    typeof candidate.message === "string"
  );
}

function isLogStreamMessage(value: unknown): value is LogStreamMessage {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Record<string, unknown>;

  if (candidate.kind === "replay" && Array.isArray(candidate.items)) {
    return candidate.items.every((item) => isLogEntryPayload(item));
  }

  if (candidate.kind === "entry") {
    return isLogEntryPayload(candidate.item);
  }

  return false;
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

          <div class="logs-toolbar__meta-badges">
            <div class="logs-toolbar__status" :class="liveStatusMeta.className">
              <strong>{{ liveStatusMeta.label }}</strong>
              <span>{{ liveStatusMeta.body }}</span>
              <button
                v-if="liveStatus !== 'live'"
                class="logs-toolbar__status-action"
                type="button"
                @click="reconnectLiveLogs"
              >
                {{ t("logs.reconnectNow") }}
              </button>
            </div>
            <span class="logs-toolbar__badge">{{ t("common.adminOnly") }}</span>
          </div>
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
        <div class="logs-console-panel__bar">
          <div
            class="logs-console-panel__follow"
            :class="{
              'logs-console-panel__follow--paused': !autoFollow,
            }"
          >
            <strong>{{
              autoFollow ? t("logs.followLive") : t("logs.followPaused")
            }}</strong>
            <span>
              {{
                autoFollow
                  ? t("logs.followLiveBody")
                  : pendingEntriesCount > 0
                    ? t("logs.followPausedCount", { count: pendingEntriesCount })
                    : t("logs.followPausedBody")
              }}
            </span>
          </div>
          <div class="logs-console-panel__actions">
            <button
              v-if="autoFollow"
              class="logs-console-panel__action"
              type="button"
              @click="pauseAutoFollow"
            >
              {{ t("logs.pauseFollow") }}
            </button>
            <button
              v-else
              class="logs-console-panel__action logs-console-panel__action--primary"
              type="button"
              @click="resumeAutoFollow"
            >
              {{ t("logs.resumeFollow") }}
            </button>
          </div>
        </div>
        <div
          ref="logsConsoleRef"
          class="logs-console"
          role="log"
          aria-live="polite"
          @scroll="handleConsoleScroll"
        >
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

.logs-toolbar__meta-badges {
  display: grid;
  justify-items: end;
  gap: 0.65rem;
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

.logs-toolbar__status {
  display: grid;
  gap: 0.18rem;
  width: min(18rem, 100%);
  padding: 0.75rem 0.85rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-surface-strong) 82%, transparent);
}

.logs-toolbar__status strong,
.logs-toolbar__status span {
  margin: 0;
}

.logs-toolbar__status strong {
  font-size: 0.86rem;
}

.logs-toolbar__status span {
  color: var(--text-muted);
  font-size: 0.8rem;
  line-height: 1.45;
}

.logs-toolbar__status-action {
  width: fit-content;
  min-height: 2.15rem;
  padding-inline: 0.8rem;
  border-color: transparent;
  background: color-mix(in srgb, var(--bg-accent) 68%, transparent);
  color: var(--signal-blue);
  box-shadow: none;
}

.logs-toolbar__status--live strong {
  color: var(--signal-green);
}

.logs-toolbar__status--connecting strong {
  color: var(--signal-blue);
}

.logs-toolbar__status--reconnecting strong {
  color: var(--signal-warm);
}

.logs-toolbar__status--offline strong {
  color: var(--signal-red);
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

.logs-console-panel__bar {
  display: flex;
  justify-content: space-between;
  gap: 0.9rem;
  align-items: start;
  margin-bottom: 0.9rem;
}

.logs-console-panel__follow {
  display: grid;
  gap: 0.16rem;
  min-width: 0;
  padding: 0.75rem 0.85rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-surface-strong) 78%, transparent);
}

.logs-console-panel__follow strong,
.logs-console-panel__follow span {
  margin: 0;
}

.logs-console-panel__follow strong {
  color: var(--signal-green);
  font-size: 0.86rem;
}

.logs-console-panel__follow span {
  color: var(--text-muted);
  font-size: 0.8rem;
  line-height: 1.45;
}

.logs-console-panel__follow--paused strong {
  color: var(--signal-warm);
}

.logs-console-panel__actions {
  display: flex;
  gap: 0.6rem;
  flex-shrink: 0;
}

.logs-console-panel__action {
  min-height: 2.45rem;
  padding-inline: 0.95rem;
  white-space: nowrap;
}

.logs-console-panel__action--primary {
  border-color: transparent;
  background: var(--button-primary-bg);
  color: #fff;
  box-shadow: none;
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

  .logs-toolbar__meta-badges {
    width: 100%;
    justify-items: stretch;
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

  .logs-console-panel__bar {
    flex-direction: column;
  }

  .logs-console-panel__actions {
    width: 100%;
  }

  .logs-console-panel__action {
    flex: 1;
  }

  .logs-console__line {
    grid-template-columns: 1fr;
    gap: 0.28rem;
  }
}
</style>
