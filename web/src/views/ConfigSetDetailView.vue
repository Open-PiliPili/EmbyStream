<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";

import { ApiError, getArtifacts, listConfigSets } from "@/api/client";
import { CONFIG_SETS_API } from "@/api/constants";
import type { ArtifactDocument, ConfigSetSummary } from "@/api/types";
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import FilePreviewPanel from "@/components/blocks/FilePreviewPanel.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";

const { t, locale } = useI18n();
const route = useRoute();
const router = useRouter();

const configSets = ref<ConfigSetSummary[]>([]);
const selectedArtifacts = ref<ArtifactDocument[]>([]);
const loading = ref(true);
const errorMessage = ref("");
const focusedArtifact = ref<ArtifactDocument | null>(null);
const collapsedArtifacts = reactive<Record<string, boolean>>({});

useDocumentLocale();

const configSetId = computed(() => String(route.params.configSetId ?? ""));

const selectedConfigSet = computed(() =>
  configSets.value.find((item) => item.id === configSetId.value),
);

const selectedUpdatedAt = computed(() => {
  if (!selectedConfigSet.value?.updated_at) {
    return "—";
  }

  const date = new Date(selectedConfigSet.value.updated_at);
  if (Number.isNaN(date.getTime())) {
    return selectedConfigSet.value.updated_at;
  }

  return new Intl.DateTimeFormat(locale.value, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
});

function handlePreviewKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeArtifactModal();
  }
}

watch(focusedArtifact, (value) => {
  if (typeof document === "undefined") {
    return;
  }

  document.body.style.overflow = value ? "hidden" : "";

  if (value) {
    document.addEventListener("keydown", handlePreviewKeydown);
  } else {
    document.removeEventListener("keydown", handlePreviewKeydown);
  }
});

onMounted(async () => {
  await loadData();
});

onBeforeUnmount(() => {
  if (typeof document !== "undefined") {
    document.body.style.overflow = "";
    document.removeEventListener("keydown", handlePreviewKeydown);
  }
});

watch(configSetId, async (value, previousValue) => {
  if (!value || value === previousValue) {
    return;
  }

  await loadData();
});

async function loadData() {
  if (!configSetId.value) {
    errorMessage.value = t("configSets.detailMissingBody");
    loading.value = false;
    return;
  }

  loading.value = true;
  errorMessage.value = "";
  focusedArtifact.value = null;

  try {
    const [configSetResponse, artifactsResponse] = await Promise.all([
      listConfigSets(),
      getArtifacts(configSetId.value),
    ]);

    configSets.value = configSetResponse.items;
    selectedArtifacts.value = artifactsResponse.items;

    for (const key of Object.keys(collapsedArtifacts)) {
      delete collapsedArtifacts[key];
    }

    for (const artifact of artifactsResponse.items) {
      collapsedArtifacts[artifact.file_name] = true;
    }

    if (
      !configSetResponse.items.some((item) => item.id === configSetId.value)
    ) {
      errorMessage.value = t("configSets.detailMissingBody");
    }
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.configsLoadFailed");
  } finally {
    loading.value = false;
  }
}

function toggleArtifact(fileName: string) {
  collapsedArtifacts[fileName] = !collapsedArtifacts[fileName];
}

function openArtifactModal(artifact: ArtifactDocument) {
  focusedArtifact.value = artifact;
}

function closeArtifactModal() {
  focusedArtifact.value = null;
}
</script>

<template>
  <AppWorkspaceShell
    :body="t('configSets.detailBody')"
    :eyebrow="t('configSets.detailEyebrow')"
    :title="selectedConfigSet?.name || t('configSets.detailTitle')"
  >
    <template #hero-actions>
      <button
        class="workspace-shell__hero-action workspace-shell__hero-action--secondary"
        type="button"
        @click="router.push({ name: 'config-sets' })"
      >
        <Icon aria-hidden="true" icon="ph:arrow-left" width="16" />
        <span>{{ t("configSets.backToList") }}</span>
      </button>
    </template>

    <GlassPanel v-if="loading" class="config-detail__state">
      <p class="section-label">{{ t("common.loading") }}</p>
      <h3>{{ t("configSets.detailLoadingTitle") }}</h3>
      <p>{{ t("configSets.detailLoadingBody") }}</p>
    </GlassPanel>

    <GlassPanel
      v-else-if="errorMessage"
      class="config-detail__state"
      tone="warm"
    >
      <p class="section-label">{{ t("common.forbidden") }}</p>
      <h3>{{ t("configSets.detailMissingTitle") }}</h3>
      <p>{{ errorMessage }}</p>
    </GlassPanel>

    <template v-else-if="selectedConfigSet">
      <GlassPanel class="config-detail__summary">
        <div class="config-detail__summary-copy">
          <div class="config-detail__chips">
            <span class="config-detail__chip">{{
              t(`modes.${selectedConfigSet.stream_mode}`)
            }}</span>
            <span class="config-detail__chip config-detail__chip--muted">
              {{
                t("configSets.countLabel", { count: selectedArtifacts.length })
              }}
            </span>
          </div>
          <p>{{ t("configSets.updatedAt", { time: selectedUpdatedAt }) }}</p>
        </div>
      </GlassPanel>

      <section v-if="selectedArtifacts.length" class="artifact-list">
        <div
          v-for="artifact in selectedArtifacts"
          :key="artifact.file_name"
          class="artifact-list__item"
        >
          <FilePreviewPanel
            :collapse-label="t('common.collapse')"
            :collapsed="collapsedArtifacts[artifact.file_name]"
            :content="artifact.content"
            :download-href="
              CONFIG_SETS_API.artifactDownload(
                configSetId,
                artifact.artifact_type,
              )
            "
            :download-label="t('common.download')"
            :expand-label="t('common.expand')"
            :file-name="artifact.file_name"
            :language="artifact.language"
            :open-label="t('common.openPreview')"
            @toggle="toggleArtifact(artifact.file_name)"
            @open="openArtifactModal(artifact)"
          />
        </div>
      </section>

      <GlassPanel v-else class="config-detail__state">
        <p class="section-label">{{ t("common.preview") }}</p>
        <h3>{{ t("configSets.emptyTitle") }}</h3>
        <p>{{ t("configSets.emptyBody") }}</p>
      </GlassPanel>
    </template>

    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="focusedArtifact"
          class="config-detail__overlay"
          role="dialog"
          aria-modal="true"
          @click.self="closeArtifactModal"
        >
          <GlassPanel class="config-detail__overlay-panel">
            <div class="config-detail__overlay-head">
              <div>
                <p class="section-label">{{ selectedConfigSet?.name }}</p>
                <h3>{{ focusedArtifact.file_name }}</h3>
              </div>
              <button
                class="config-detail__overlay-close"
                type="button"
                :aria-label="t('common.close')"
                @click="closeArtifactModal"
              >
                <svg
                  aria-hidden="true"
                  class="config-detail__overlay-close-icon"
                  viewBox="0 0 24 24"
                >
                  <path
                    d="M6 6l12 12M18 6L6 18"
                    fill="none"
                    stroke="currentColor"
                    stroke-linecap="round"
                    stroke-width="2"
                  />
                </svg>
              </button>
            </div>
            <FilePreviewPanel
              :collapse-label="t('common.collapse')"
              :collapsed="false"
              :content="focusedArtifact.content"
              :download-href="
                CONFIG_SETS_API.artifactDownload(
                  configSetId,
                  focusedArtifact.artifact_type,
                )
              "
              :download-label="t('common.download')"
              :expand-label="t('common.expand')"
              :file-name="focusedArtifact.file_name"
              :immersive="true"
              :language="focusedArtifact.language"
            />
          </GlassPanel>
        </div>
      </Transition>
    </Teleport>
  </AppWorkspaceShell>
</template>

<style scoped>
.config-detail__summary,
.config-detail__state,
.config-detail__overlay-panel {
  padding: 1.25rem;
}

.config-detail__summary {
  display: grid;
  gap: 0.75rem;
}

.config-detail__summary-copy h2,
.config-detail__summary-copy p,
.config-detail__state h3,
.config-detail__state p {
  margin: 0;
}

.config-detail__summary-copy h2 {
  font-size: clamp(1.7rem, 2.2vw, 2.25rem);
  line-height: 1.05;
  letter-spacing: -0.04em;
}

.config-detail__summary-copy p,
.config-detail__state p:last-child {
  color: var(--text-muted);
}

.config-detail__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  margin-bottom: 0.75rem;
}

.config-detail__chip {
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

.config-detail__chip--muted {
  background: var(--bg-soft);
  color: var(--text-muted);
}

.artifact-list {
  display: grid;
  gap: 1rem;
  margin-top: 1rem;
}

.config-detail__state {
  display: grid;
  gap: 0.7rem;
}

.config-detail__state h3 {
  font-size: 1.25rem;
  line-height: 1.16;
  letter-spacing: -0.03em;
}

.config-detail__overlay {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(20, 20, 19, 0.34);
  overflow: auto;
}

.config-detail__overlay-panel {
  width: min(72rem, 100%);
  max-height: calc(100vh - 2rem);
  overflow: auto;
}

.config-detail__overlay-head {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: start;
  margin-bottom: 1rem;
}

.config-detail__overlay-head h3 {
  margin: 0.45rem 0 0;
  font-size: 1.4rem;
  letter-spacing: -0.03em;
}

.config-detail__overlay-close-icon {
  display: block;
  flex-shrink: 0;
}

.modal-pop-enter-active,
.modal-pop-leave-active {
  transition: opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from,
.modal-pop-leave-to {
  opacity: 0;
}

.modal-pop-enter-active .config-detail__overlay-panel,
.modal-pop-leave-active .config-detail__overlay-panel {
  transition:
    transform 320ms var(--curve-spring),
    opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from .config-detail__overlay-panel,
.modal-pop-leave-to .config-detail__overlay-panel {
  opacity: 0;
  transform: scale(0.965);
}

.config-detail__overlay-close {
  min-width: 2.7rem;
  min-height: 2.7rem;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
</style>
