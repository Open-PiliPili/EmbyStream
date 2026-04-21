<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

import {
  ApiError,
  deleteConfigSet,
  duplicateConfigSet,
  listConfigSets,
  updateConfigSetMetadata,
} from "@/api/client";
import type { ConfigSetSummary } from "@/api/types";
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import ActionDialog from "@/components/ui/ActionDialog.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";

const { t, locale } = useI18n();
const router = useRouter();

const configSets = ref<ConfigSetSummary[]>([]);
const errorMessage = ref("");
const loading = ref(true);
const isMobile = ref(false);
const configSetToDelete = ref<ConfigSetSummary | null>(null);
const configSetToRename = ref<ConfigSetSummary | null>(null);
const renameConfigValue = ref("");

useDocumentLocale();

const configList = computed(() =>
  configSets.value.map((item) => ({
    ...item,
    updatedAt: formatDate(item.updated_at),
  })),
);

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(locale.value, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function syncMobileState() {
  if (typeof window === "undefined") {
    return;
  }

  isMobile.value = window.matchMedia("(max-width: 980px)").matches;
}

onMounted(async () => {
  syncMobileState();
  window.addEventListener("resize", syncMobileState);
  await refresh();
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", syncMobileState);
});

async function refresh() {
  loading.value = true;
  errorMessage.value = "";

  try {
    const response = await listConfigSets();
    configSets.value = response.items;
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.configsLoadFailed");
  } finally {
    loading.value = false;
  }
}

async function duplicate(configSetId: string) {
  const response = await duplicateConfigSet(configSetId);
  await router.push({ name: "wizard", query: { draftId: response.draft.id } });
}

function rename(configSet: ConfigSetSummary) {
  configSetToRename.value = configSet;
  renameConfigValue.value = configSet.name;
}

function remove(configSet: ConfigSetSummary) {
  configSetToDelete.value = configSet;
}

async function submitRenameConfigSet() {
  if (!configSetToRename.value) {
    return;
  }

  const nextName = renameConfigValue.value.trim();
  if (!nextName || nextName === configSetToRename.value.name) {
    closeRenameConfigDialog();
    return;
  }

  try {
    await updateConfigSetMetadata(configSetToRename.value.id, {
      name: nextName,
    });
    closeRenameConfigDialog();
    await refresh();
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.configsLoadFailed");
  }
}

async function confirmRemoveConfigSet() {
  if (!configSetToDelete.value) {
    return;
  }

  await deleteConfigSet(configSetToDelete.value.id);
  configSetToDelete.value = null;
  await refresh();
}

function closeRenameConfigDialog() {
  configSetToRename.value = null;
  renameConfigValue.value = "";
}

function goToDetail(configSetId: string) {
  router.push({ name: "config-set-detail", params: { configSetId } });
}

function handleItemClick(configSetId: string) {
  goToDetail(configSetId);
}
</script>

<template>
  <AppWorkspaceShell
    :body="t('configSets.body')"
    :eyebrow="t('configSets.eyebrow')"
    :title="t('configSets.title')"
  >
    <section class="config-list-view__hero">
      <div class="config-list-view__hero-copy">
        <p class="section-label">{{ t("configSets.libraryLabel") }}</p>
        <h2>{{ t("configSets.libraryTitle") }}</h2>
        <p>{{ t("configSets.libraryBody") }}</p>
      </div>
      <div class="config-list-view__toolbar">
        <button
          class="config-list-view__primary"
          type="button"
          @click="router.push({ name: 'wizard' })"
        >
          {{ t("configSets.createAction") }}
        </button>
        <button
          class="config-list-view__secondary"
          type="button"
          @click="router.push({ name: 'drafts' })"
        >
          {{ t("configSets.draftsAction") }}
        </button>
      </div>
    </section>

    <GlassPanel v-if="loading" class="config-list-view__state">
      <p class="section-label">{{ t("common.loading") }}</p>
      <h3>{{ t("configSets.loadingTitle") }}</h3>
      <p>{{ t("configSets.loadingBody") }}</p>
    </GlassPanel>

    <GlassPanel
      v-else-if="errorMessage"
      class="config-list-view__state"
      tone="warm"
    >
      <p class="section-label">{{ t("common.forbidden") }}</p>
      <h3>{{ t("configSets.errorTitle") }}</h3>
      <p>{{ errorMessage }}</p>
    </GlassPanel>

    <section v-else-if="configList.length" class="config-list">
      <GlassPanel
        v-for="configSet in configList"
        :key="configSet.id"
        class="config-list__item config-list__item--link"
        @click="handleItemClick(configSet.id)"
      >
        <div class="config-list__item-copy">
          <div class="config-list__chips">
            <span class="config-list__chip">{{
              t(`modes.${configSet.stream_mode}`)
            }}</span>
          </div>
          <h3>{{ configSet.name }}</h3>
          <p>{{ t("configSets.updatedAt", { time: configSet.updatedAt }) }}</p>
        </div>

        <div class="config-list__actions">
          <button
            v-if="!isMobile"
            class="config-list__action config-list__action--detail"
            type="button"
            @click.stop="goToDetail(configSet.id)"
          >
            <span>{{ t("configSets.detailAction") }}</span>
          </button>
          <button
            class="config-list__action"
            type="button"
            :aria-label="t('common.duplicate')"
            @click.stop="duplicate(configSet.id)"
          >
            <Icon aria-hidden="true" icon="ph:copy" width="16" />
            <span>{{ t("common.duplicate") }}</span>
          </button>
          <button
            class="config-list__action"
            type="button"
            :aria-label="t('common.rename')"
            @click.stop="rename(configSet)"
          >
            <Icon aria-hidden="true" icon="ph:pencil-simple" width="16" />
            <span>{{ t("common.rename") }}</span>
          </button>
          <button
            class="config-list__action config-list__action--danger"
            type="button"
            :aria-label="t('common.delete')"
            @click.stop="remove(configSet)"
          >
            <Icon aria-hidden="true" icon="ph:trash" width="16" />
            <span>{{ t("common.delete") }}</span>
          </button>
        </div>
      </GlassPanel>
    </section>

    <GlassPanel v-else class="config-list-view__state">
      <p class="section-label">{{ t("common.preview") }}</p>
      <h3>{{ t("configSets.emptyTitle") }}</h3>
      <p>{{ t("configSets.emptyBody") }}</p>
    </GlassPanel>

    <ActionDialog
      :open="Boolean(configSetToRename)"
      :title="t('common.rename')"
      :description="t('common.promptRenameConfigSet')"
      :confirm-label="t('common.rename')"
      :cancel-label="t('common.closePreview')"
      input-label="Name"
      :input-value="renameConfigValue"
      :input-placeholder="t('common.promptRenameConfigSet')"
      @close="closeRenameConfigDialog"
      @confirm="submitRenameConfigSet"
      @update:input-value="renameConfigValue = $event"
    />

    <ActionDialog
      :open="Boolean(configSetToDelete)"
      :title="t('common.delete')"
      :description="t('common.confirmDeleteConfigSet')"
      :confirm-label="t('common.delete')"
      :cancel-label="t('common.closePreview')"
      confirm-tone="danger"
      @close="configSetToDelete = null"
      @confirm="confirmRemoveConfigSet"
    />
  </AppWorkspaceShell>
</template>

<style scoped>
.config-list-view__hero {
  display: grid;
  gap: 1rem;
}

.config-list-view__hero-copy {
  padding: 1.2rem 1.25rem;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  box-shadow: var(--shadow-soft);
}

.config-list-view__hero-copy h2,
.config-list-view__hero-copy p,
.config-list-view__state h3,
.config-list-view__state p {
  margin: 0;
}

.config-list-view__hero-copy h2 {
  margin-top: 0.45rem;
  font-size: clamp(1.8rem, 2.4vw, 2.5rem);
  line-height: 1;
  letter-spacing: -0.05em;
}

.config-list-view__hero-copy p:last-child,
.config-list-view__state p:last-child {
  margin-top: 0.9rem;
  color: var(--text-muted);
}

.config-list-view__toolbar {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.config-list-view__primary,
.config-list-view__secondary {
  min-height: 2.7rem;
  padding-inline: 1.15rem;
  font-weight: 700;
}

.config-list-view__primary {
  border-color: transparent;
  background: var(--button-primary-bg);
  color: #fff;
}

.config-list-view__primary:hover {
  background: var(--button-primary-hover);
}

.config-list-view__secondary {
  border-color: var(--border-accent);
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 88%,
    var(--bg-accent)
  );
  color: var(--signal-blue);
  box-shadow: 0 0 0 1px
    color-mix(in srgb, var(--brand-secondary) 12%, transparent);
}

.config-list-view__secondary:hover {
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 80%,
    var(--bg-accent)
  );
}

.config-list {
  display: grid;
  gap: 1rem;
  margin-top: 1rem;
}

.config-list__item {
  display: flex;
  justify-content: space-between;
  gap: 1.25rem;
  align-items: flex-start;
  padding: 1.2rem 1.25rem;
  cursor: pointer;
  transition:
    background-color 180ms var(--curve-swift),
    border-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    transform 220ms var(--curve-buoy);
}

.config-list__item-copy h3,
.config-list__item-copy p {
  margin: 0;
}

.config-list__item-copy h3 {
  font-size: clamp(1.15rem, 2vw, 1.45rem);
  line-height: 1.15;
}

.config-list__item-copy p {
  margin-top: 0.6rem;
  color: var(--text-muted);
}

.config-list__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  margin-bottom: 0.85rem;
}

.config-list__chip {
  display: inline-flex;
  align-items: center;
  min-height: 2rem;
  padding: 0.2rem 0.72rem;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--signal-blue) 14%, transparent);
  color: var(--signal-blue);
  font-size: 0.77rem;
  font-weight: 700;
}

.config-list__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.65rem;
}

.config-list__action {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-height: 2.45rem;
  padding-inline: 0.9rem;
  border-radius: var(--radius-pill);
}

.config-list__action--detail {
  border-color: var(--border-accent);
  color: var(--signal-blue);
}

.config-list__action--danger {
  color: var(--signal-red);
}

.config-list-view__state {
  display: grid;
  gap: 0.7rem;
  padding: 1.25rem;
  margin-top: 1rem;
}

.config-list-view__state h3 {
  font-size: 1.25rem;
  line-height: 1.16;
  letter-spacing: -0.03em;
}

@media (pointer: fine) {
  .config-list__item:hover {
    background: var(--bg-surface-strong);
    border-color: var(--border-strong);
    box-shadow: var(--card-hover-shadow);
    transform: translateY(-2px);
  }
}

.config-list__item:focus-within {
  background: var(--bg-surface-strong);
  border-color: var(--border-strong);
  box-shadow: var(--card-focus-shadow);
}

@media (max-width: 980px) {
  .config-list__item {
    flex-direction: column;
  }

  .config-list__actions {
    width: 100%;
    justify-content: flex-start;
  }

  .config-list__action {
    width: 2.6rem;
    min-width: 2.6rem;
    min-height: 2.6rem;
    justify-content: center;
    padding: 0;
  }

  .config-list__action span {
    display: none;
  }
}
</style>
