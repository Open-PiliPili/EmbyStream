<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

import {
  ApiError,
  deleteDraft,
  listConfigSets,
  listDrafts,
  updateDraftMetadata,
} from "@/api/client";
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import type { DraftSummary } from "@/api/types";
import ActionDialog from "@/components/ui/ActionDialog.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import PillTabs from "@/components/ui/PillTabs.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";

const { t, locale } = useI18n();
const router = useRouter();

const drafts = ref<DraftSummary[]>([]);
const configSetCount = ref(0);
const errorMessage = ref("");
const loading = ref(true);
const draftToDelete = ref<DraftSummary | null>(null);
const draftToRename = ref<DraftSummary | null>(null);
const renameDraftValue = ref("");

useDocumentLocale();

const generatedCount = computed(
  () => drafts.value.filter((draft) => draft.status === "generated").length,
);

const statusCards = computed(() => [
  { key: "drafts", label: t("drafts.statsDrafts"), value: drafts.value.length },
  {
    key: "generated",
    label: t("drafts.statsGenerated"),
    value: generatedCount.value,
  },
  {
    key: "configs",
    label: t("drafts.statsConfigs"),
    value: configSetCount.value,
  },
]);

const formattedDrafts = computed(() =>
  drafts.value.map((draft) => ({
    ...draft,
    updatedLabel: formatTimestamp(draft.updated_at),
  })),
);

onMounted(async () => {
  try {
    await refreshLists();
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.draftsLoadFailed");
  } finally {
    loading.value = false;
  }
});

function continueDraft(draftId: string) {
  router.push({ name: "wizard", query: { draftId } });
}

function renameDraft(draft: DraftSummary) {
  draftToRename.value = draft;
  renameDraftValue.value = draft.name;
}

async function submitRenameDraft() {
  if (!draftToRename.value) {
    return;
  }

  const nextName = renameDraftValue.value.trim();
  if (!nextName || nextName === draftToRename.value.name) {
    closeRenameDraftDialog();
    return;
  }

  try {
    await updateDraftMetadata(draftToRename.value.id, {
      name: nextName,
    });
    await refreshLists();
    closeRenameDraftDialog();
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.draftsLoadFailed");
  }
}

function removeDraft(draft: DraftSummary) {
  draftToDelete.value = draft;
}

async function confirmRemoveDraft() {
  if (!draftToDelete.value) {
    return;
  }

  try {
    await deleteDraft(draftToDelete.value.id);
    await refreshLists();
    draftToDelete.value = null;
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.draftsLoadFailed");
  }
}

function closeRenameDraftDialog() {
  draftToRename.value = null;
  renameDraftValue.value = "";
}

async function refreshLists() {
  const [draftResponse, configSetResponse] = await Promise.all([
    listDrafts(),
    listConfigSets(),
  ]);
  drafts.value = draftResponse.items;
  configSetCount.value = configSetResponse.items.length;
}

function formatTimestamp(value: string) {
  if (!value) {
    return "—";
  }

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
</script>

<template>
  <AppWorkspaceShell
    :body="t('drafts.body')"
    :eyebrow="t('drafts.eyebrow')"
    :title="t('drafts.title')"
  >
    <template #hero-actions>
      <button
        class="workspace-shell__hero-action workspace-shell__hero-action--secondary"
        type="button"
        @click="router.push({ name: 'config-sets' })"
      >
        <Icon icon="ph:arrow-left" width="16" />
        {{ t("drafts.backToConfigs") }}
      </button>
    </template>

    <section class="drafts-hero">
      <div class="drafts-hero__copy">
        <p class="section-label">{{ t("drafts.queueLabel") }}</p>
        <h2>{{ t("drafts.queueTitle") }}</h2>
        <p>{{ t("drafts.queueBody") }}</p>
      </div>

      <div class="drafts-hero__stats">
        <GlassPanel
          v-for="card in statusCards"
          :key="card.key"
          class="drafts-hero__stat"
        >
          <p>{{ card.label }}</p>
          <strong>{{ card.value }}</strong>
        </GlassPanel>
      </div>
    </section>

    <header class="drafts-toolbar">
      <PillTabs
        :items="[
          { key: 'new', label: t('common.newDraft') },
          {
            key: 'configs',
            label: t('configSets.countLabel', { count: configSetCount }),
          },
        ]"
        @select="
          (key) =>
            key === 'new'
              ? router.push({ name: 'wizard' })
              : router.push({ name: 'config-sets' })
        "
      />
    </header>

    <section class="drafts-grid">
      <GlassPanel v-if="loading" class="draft-card draft-card--state">
        <p class="section-label">{{ t("common.loading") }}</p>
        <h3>{{ t("drafts.loadingTitle") }}</h3>
        <p>{{ t("drafts.loadingBody") }}</p>
      </GlassPanel>

      <GlassPanel
        v-else-if="errorMessage"
        class="draft-card draft-card--state"
        tone="warm"
      >
        <p class="section-label">{{ t("common.forbidden") }}</p>
        <h3>{{ t("drafts.errorTitle") }}</h3>
        <p>{{ errorMessage }}</p>
      </GlassPanel>

      <template v-else-if="formattedDrafts.length">
        <GlassPanel
          v-for="draft in formattedDrafts"
          :key="draft.id"
          class="draft-card"
          :tone="draft.status === 'generated' ? 'warm' : 'cool'"
        >
          <div class="draft-card__head">
            <span class="draft-card__status">
              {{
                draft.status === "generated"
                  ? t("drafts.statusGenerated")
                  : t("drafts.statusDraft")
              }}
            </span>
            <span class="draft-card__mode">{{
              t(`modes.${draft.stream_mode}`)
            }}</span>
          </div>

          <div class="draft-card__body">
            <h3>{{ draft.name }}</h3>
            <p>{{ t("drafts.updatedAt", { time: draft.updatedLabel }) }}</p>
          </div>

          <div class="draft-card__actions">
            <button type="button" @click="continueDraft(draft.id)">
              {{ t("common.continue") }}
            </button>
            <button type="button" @click="renameDraft(draft)">
              {{ t("common.rename") }}
            </button>
            <button type="button" @click="removeDraft(draft)">
              {{ t("common.delete") }}
            </button>
          </div>
        </GlassPanel>
      </template>

      <GlassPanel v-else class="draft-card draft-card--state">
        <p class="section-label">{{ t("common.noDrafts") }}</p>
        <h3>{{ t("drafts.emptyTitle") }}</h3>
        <p>{{ t("drafts.emptyBody") }}</p>
        <button
          class="draft-card__primary"
          type="button"
          @click="router.push({ name: 'wizard' })"
        >
          {{ t("common.newDraft") }}
        </button>
      </GlassPanel>
    </section>

    <ActionDialog
      :open="Boolean(draftToRename)"
      :title="t('common.rename')"
      :description="t('common.promptRenameDraft')"
      :confirm-label="t('common.rename')"
      :cancel-label="t('common.closePreview')"
      input-label="Name"
      :input-value="renameDraftValue"
      :input-placeholder="t('common.promptRenameDraft')"
      @close="closeRenameDraftDialog"
      @confirm="submitRenameDraft"
      @update:input-value="renameDraftValue = $event"
    />

    <ActionDialog
      :open="Boolean(draftToDelete)"
      :title="t('common.delete')"
      :description="t('common.confirmDeleteConfigSet')"
      :confirm-label="t('common.delete')"
      :cancel-label="t('common.closePreview')"
      confirm-tone="danger"
      @close="draftToDelete = null"
      @confirm="confirmRemoveDraft"
    />
  </AppWorkspaceShell>
</template>

<style scoped>
.drafts-hero {
  display: grid;
  grid-template-columns: minmax(0, 1.3fr) minmax(18rem, 0.95fr);
  gap: 1rem;
  align-items: stretch;
}

.drafts-hero__copy,
.drafts-hero__stat {
  padding: 1.25rem;
  border-radius: var(--radius-lg);
}

.drafts-hero__copy {
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  box-shadow: var(--shadow-soft);
}

.drafts-hero__copy h2,
.drafts-hero__copy p,
.drafts-hero__stat p,
.drafts-hero__stat strong {
  margin: 0;
}

.drafts-hero__copy h2 {
  margin-top: 0.5rem;
  font-size: clamp(1.8rem, 2.4vw, 2.4rem);
  line-height: 1;
  letter-spacing: -0.05em;
}

.drafts-hero__copy p:last-child {
  margin-top: 0.9rem;
  color: var(--text-muted);
  line-height: 1.7;
}

.drafts-hero__stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.9rem;
}

.drafts-hero__stat {
  display: grid;
  gap: 0.7rem;
}

.drafts-hero__stat p {
  color: var(--text-faint);
  font-size: 0.82rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.drafts-hero__stat strong {
  font-size: clamp(1.6rem, 3vw, 2.2rem);
  line-height: 1;
  letter-spacing: -0.05em;
}

.drafts-toolbar {
  margin: 1.4rem 0 1rem;
}

.drafts-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
  gap: 1rem;
}

.draft-card {
  display: grid;
  gap: 1rem;
  padding: 1.25rem;
  transition:
    background-color 180ms var(--curve-swift),
    border-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    transform 220ms var(--curve-buoy);
}

.draft-card--state {
  place-items: start;
}

.draft-card--state h3,
.draft-card--state p,
.draft-card__body h3,
.draft-card__body p {
  margin: 0;
}

.draft-card--state h3,
.draft-card__body h3 {
  font-size: 1.3rem;
  line-height: 1.15;
  letter-spacing: -0.03em;
}

.draft-card--state p,
.draft-card__body p {
  color: var(--text-muted);
}

.draft-card__head,
.draft-card__actions {
  display: flex;
  justify-content: space-between;
  gap: 0.7rem;
  align-items: center;
  flex-wrap: wrap;
}

.draft-card__status,
.draft-card__mode {
  display: inline-flex;
  align-items: center;
  min-height: 2rem;
  padding: 0.2rem 0.72rem;
  border-radius: var(--radius-pill);
  font-size: 0.76rem;
  font-weight: 700;
}

.draft-card__status {
  background: color-mix(in srgb, var(--signal-blue) 14%, transparent);
  color: var(--signal-blue);
}

.draft-card__mode {
  background: var(--bg-soft);
  color: var(--text-muted);
}

.draft-card__body {
  display: grid;
  gap: 0.45rem;
}

.draft-card__actions {
  justify-content: flex-start;
}

.draft-card__actions button,
.draft-card__primary {
  min-height: 2.45rem;
  padding-inline: 0.95rem;
}

.draft-card__primary {
  border-color: transparent;
  background: var(--button-primary-bg);
  color: white;
  font-weight: 700;
}

.draft-card__primary:hover {
  background: var(--button-primary-hover);
}

@media (pointer: fine) {
  .draft-card:hover {
    background: var(--bg-surface-strong);
    border-color: var(--border-strong);
    box-shadow: var(--card-hover-shadow);
    transform: translateY(-2px);
  }
}

.draft-card:focus-within {
  background: var(--bg-surface-strong);
  border-color: var(--border-strong);
  box-shadow: var(--card-focus-shadow);
  transform: translateY(-1px);
}

@media (max-width: 920px) {
  .drafts-hero {
    grid-template-columns: 1fr;
  }

  .drafts-hero__stats {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 640px) {
  .drafts-hero__stats {
    grid-template-columns: 1fr;
  }
}
</style>
