<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import {
  ApiError,
  deleteUser,
  listUsers,
  updateUserDisabled,
  updateUserPassword,
  updateUserRole,
} from "@/api/client";
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import type { AdminUserSummary } from "@/api/types";

const { t, locale } = useI18n();

const users = ref<AdminUserSummary[]>([]);
const keyword = ref("");
const loading = ref(true);
const errorMessage = ref("");
const activeUser = ref<AdminUserSummary | null>(null);
const dialogMode = ref<"password" | "delete" | null>(null);
const passwordDraft = ref("");

useDocumentLocale();

const filteredUsers = computed(() => {
  const query = keyword.value.trim().toLowerCase();
  if (!query) {
    return users.value;
  }

  return users.value.filter((user) =>
    `${user.username} ${user.email ?? ""} ${user.role}`
      .toLowerCase()
      .includes(query),
  );
});

onMounted(async () => {
  await refreshUsers();
});

async function refreshUsers() {
  loading.value = true;
  errorMessage.value = "";
  try {
    const response = await listUsers();
    users.value = response.items;
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("users.errorBody");
  } finally {
    loading.value = false;
  }
}

async function toggleRole(user: AdminUserSummary) {
  const nextRole = user.role === "admin" ? "user" : "admin";
  try {
    const response = await updateUserRole(user.id, nextRole);
    replaceUser(response.user);
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("users.errorBody");
  }
}

async function toggleDisabled(user: AdminUserSummary) {
  try {
    const response = await updateUserDisabled(user.id, !user.disabled);
    replaceUser(response.user);
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("users.errorBody");
  }
}

async function changePassword(user: AdminUserSummary) {
  activeUser.value = user;
  dialogMode.value = "password";
  passwordDraft.value = "";
}

async function submitPasswordChange() {
  if (!activeUser.value || !passwordDraft.value.trim()) {
    return;
  }

  try {
    const response = await updateUserPassword(
      activeUser.value.id,
      passwordDraft.value.trim(),
    );
    replaceUser(response.user);
    closeDialog();
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("users.errorBody");
  }
}

async function removeUser(user: AdminUserSummary) {
  activeUser.value = user;
  dialogMode.value = "delete";
}

async function confirmDeleteUser() {
  if (!activeUser.value) {
    return;
  }

  try {
    await deleteUser(activeUser.value.id);
    users.value = users.value.filter(
      (item) => item.id !== activeUser.value?.id,
    );
    closeDialog();
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("users.errorBody");
  }
}

function closeDialog() {
  dialogMode.value = null;
  activeUser.value = null;
  passwordDraft.value = "";
}

function replaceUser(nextUser: AdminUserSummary) {
  users.value = users.value.map((user) =>
    user.id === nextUser.id ? nextUser : user,
  );
}

function isBuiltinAdmin(user: AdminUserSummary) {
  return user.username === "admin" && user.role === "admin";
}

function formatTimestamp(value: string) {
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
    :body="t('users.body')"
    :eyebrow="t('users.eyebrow')"
    :title="t('users.title')"
  >
    <section class="users-toolbar">
      <GlassPanel class="users-toolbar__card">
        <p class="section-label">{{ t("users.filterLabel") }}</p>
        <h2>{{ t("users.filterTitle") }}</h2>
        <label class="users-toolbar__field">
          <span>{{ t("users.keywordLabel") }}</span>
          <input
            v-model="keyword"
            :placeholder="t('users.keywordPlaceholder')"
            type="text"
          />
        </label>
      </GlassPanel>
    </section>

    <section class="users-grid">
      <GlassPanel v-if="loading" class="users-card users-card--state">
        <p class="section-label">{{ t("common.loading") }}</p>
        <h3>{{ t("users.loadingTitle") }}</h3>
        <p>{{ t("users.loadingBody") }}</p>
      </GlassPanel>

      <GlassPanel
        v-else-if="errorMessage"
        class="users-card users-card--state"
        tone="warm"
      >
        <p class="section-label">{{ t("users.statusLabel") }}</p>
        <h3>{{ t("users.errorTitle") }}</h3>
        <p>{{ errorMessage }}</p>
      </GlassPanel>

      <template v-else-if="filteredUsers.length">
        <GlassPanel
          v-for="user in filteredUsers"
          :key="user.id"
          class="users-card"
        >
          <div class="users-card__head">
            <div>
              <h3>{{ user.username }}</h3>
              <p>{{ user.email || t("account.emailFallback") }}</p>
            </div>
            <div class="users-card__badges">
              <span class="users-card__badge">{{
                user.role === "admin"
                  ? t("account.roleAdmin")
                  : t("account.roleUser")
              }}</span>
              <span
                v-if="user.disabled"
                class="users-card__badge users-card__badge--muted"
              >
                {{ t("users.disabled") }}
              </span>
            </div>
          </div>

          <p class="users-card__meta">
            {{
              t("users.updatedAt", { time: formatTimestamp(user.updated_at) })
            }}
          </p>

          <div class="users-card__actions">
            <button
              v-if="!isBuiltinAdmin(user)"
              type="button"
              @click="toggleRole(user)"
            >
              {{
                user.role === "admin" ? t("users.demote") : t("users.promote")
              }}
            </button>
            <button
              v-if="!isBuiltinAdmin(user)"
              type="button"
              @click="toggleDisabled(user)"
            >
              {{ user.disabled ? t("users.enable") : t("users.disable") }}
            </button>
            <button type="button" @click="changePassword(user)">
              {{ t("users.changePassword") }}
            </button>
            <button
              v-if="!isBuiltinAdmin(user)"
              type="button"
              @click="removeUser(user)"
            >
              {{ t("common.delete") }}
            </button>
          </div>
        </GlassPanel>
      </template>

      <GlassPanel v-else class="users-card users-card--state">
        <p class="section-label">{{ t("users.emptyLabel") }}</p>
        <h3>{{ t("users.emptyTitle") }}</h3>
        <p>{{ t("users.emptyBody") }}</p>
      </GlassPanel>
    </section>

    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="dialogMode && activeUser"
          class="users-overlay"
          role="dialog"
          aria-modal="true"
          @click.self="closeDialog"
        >
          <GlassPanel class="users-dialog" tone="warm">
            <div class="users-dialog__head">
              <div>
                <p class="section-label">{{ t("users.statusLabel") }}</p>
                <h3>
                  {{
                    dialogMode === "password"
                      ? t("users.passwordDialogTitle")
                      : t("users.deleteDialogTitle")
                  }}
                </h3>
              </div>
            </div>

            <div class="users-dialog__body">
              <p v-if="dialogMode === 'password'" class="users-dialog__copy">
                {{
                  t("users.passwordDialogBody", {
                    username: activeUser.username,
                  })
                }}
              </p>
              <p v-else class="users-dialog__copy">
                {{
                  t("users.deleteConfirm", { username: activeUser.username })
                }}
              </p>

              <label
                v-if="dialogMode === 'password'"
                class="users-dialog__field"
              >
                <span>{{ t("users.passwordLabel") }}</span>
                <input
                  v-model="passwordDraft"
                  :placeholder="t('users.passwordPrompt')"
                  type="password"
                />
              </label>

              <div class="users-dialog__actions">
                <button type="button" @click="closeDialog">
                  {{ t("common.collapse") }}
                </button>
                <button
                  v-if="dialogMode === 'password'"
                  class="users-dialog__primary"
                  type="button"
                  @click="submitPasswordChange"
                >
                  {{ t("users.changePassword") }}
                </button>
                <button
                  v-else
                  class="users-dialog__danger"
                  type="button"
                  @click="confirmDeleteUser"
                >
                  {{ t("common.delete") }}
                </button>
              </div>
            </div>
          </GlassPanel>
        </div>
      </Transition>
    </Teleport>
  </AppWorkspaceShell>
</template>

<style scoped>
.users-toolbar {
  margin-bottom: 1rem;
}

.users-toolbar__card {
  display: grid;
  gap: 0.9rem;
  padding: 1.25rem;
}

.users-toolbar__card h2,
.users-toolbar__card p,
.users-toolbar__field span {
  margin: 0;
}

.users-toolbar__card h2 {
  font-size: 1.3rem;
  line-height: 1.12;
  font-weight: 500;
}

.users-toolbar__field {
  display: grid;
  gap: 0.45rem;
}

.users-toolbar__field span {
  color: var(--text-main);
  font-size: 0.88rem;
  font-weight: 600;
}

.users-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
  gap: 1rem;
}

.users-card {
  display: grid;
  gap: 1rem;
  padding: 1.25rem;
  transition:
    background-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    transform 220ms var(--curve-buoy);
}

.users-card__head {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: start;
}

.users-card__head h3,
.users-card__head p,
.users-card__meta,
.users-card--state h3,
.users-card--state p {
  margin: 0;
}

.users-card__head h3,
.users-card--state h3 {
  font-size: 1.2rem;
  line-height: 1.14;
  font-weight: 500;
}

.users-card__head p,
.users-card__meta,
.users-card--state p {
  color: var(--text-muted);
}

.users-card__badges,
.users-card__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}

.users-card__badge {
  display: inline-flex;
  min-height: 2rem;
  align-items: center;
  padding: 0.2rem 0.72rem;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--signal-blue) 14%, transparent);
  color: var(--signal-blue);
  font-size: 0.76rem;
  font-weight: 700;
}

.users-card__badge--muted {
  background: var(--bg-soft);
  color: var(--text-muted);
}

.users-card__actions button {
  min-height: 2.35rem;
  padding-inline: 0.9rem;
}

.users-card--state {
  grid-column: 1 / -1;
}

@media (pointer: fine) {
  .users-card:hover {
    background: var(--bg-surface-strong);
    border-color: var(--border-strong);
    box-shadow: var(--card-hover-shadow);
    transform: translateY(-2px);
  }
}

.users-card:focus-within {
  background: var(--bg-surface-strong);
  border-color: var(--border-strong);
  box-shadow: var(--card-focus-shadow);
  transform: translateY(-1px);
}

.users-overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(20, 20, 19, 0.3);
}

.users-dialog {
  width: min(28rem, 100%);
  padding: 1.25rem;
}

.users-dialog__head {
  display: flex;
  justify-content: flex-start;
  gap: 1rem;
  align-items: start;
}

.users-dialog__head h3,
.users-dialog__copy,
.users-dialog__field span {
  margin: 0;
}

.users-dialog__head h3 {
  margin-top: 0.45rem;
  font-size: 1.3rem;
  line-height: 1.14;
  font-weight: 500;
}

.users-dialog__body {
  display: grid;
  gap: 1rem;
  margin-top: 1rem;
}

.users-dialog__copy {
  color: var(--text-muted);
  line-height: 1.7;
}

.users-dialog__field {
  display: grid;
  gap: 0.45rem;
}

.users-dialog__field span {
  color: var(--text-main);
  font-size: 0.88rem;
  font-weight: 600;
}

.users-dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.users-dialog__primary,
.users-dialog__danger {
  min-height: 2.45rem;
  padding-inline: 1rem;
}

.users-dialog__primary {
  border-color: transparent;
  background: var(--button-primary-bg);
  color: #fff;
}

.users-dialog__primary:hover {
  background: var(--button-primary-hover);
}

.users-dialog__danger {
  border-color: transparent;
  background: #b53333;
  color: #fff;
}

.modal-pop-enter-active,
.modal-pop-leave-active {
  transition: opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from,
.modal-pop-leave-to {
  opacity: 0;
}

.modal-pop-enter-active .users-dialog,
.modal-pop-leave-active .users-dialog {
  transition:
    transform 320ms var(--curve-spring),
    opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from .users-dialog,
.modal-pop-leave-to .users-dialog {
  opacity: 0;
  transform: scale(0.96);
}
</style>
