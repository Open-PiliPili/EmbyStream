<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import { ApiError, changeOwnPassword } from "@/api/client";
import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import { useSessionStore } from "@/stores/session";
import { useRouter } from "vue-router";

const { t } = useI18n();
const router = useRouter();
const sessionStore = useSessionStore();
const dialogOpen = ref(false);
const pending = ref(false);
const errorMessage = ref("");
const currentPassword = ref("");
const nextPassword = ref("");
const confirmPassword = ref("");

useDocumentLocale();

const roleCopy = computed(() =>
  sessionStore.isAdmin ? t("account.roleAdminBody") : t("account.roleUserBody"),
);

watch(dialogOpen, (value) => {
  if (!value) {
    return;
  }

  errorMessage.value = "";
});

function handleDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && dialogOpen.value) {
    closePasswordDialog();
  }
}

onMounted(() => {
  document.addEventListener("keydown", handleDialogKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", handleDialogKeydown);
});

async function submitPasswordChange() {
  if (pending.value) {
    return;
  }

  errorMessage.value = "";
  if (!currentPassword.value.trim()) {
    errorMessage.value = t("account.passwordCurrentRequired");
    return;
  }
  if (!nextPassword.value.trim()) {
    errorMessage.value = t("account.passwordNewRequired");
    return;
  }
  if (nextPassword.value !== confirmPassword.value) {
    errorMessage.value = t("account.passwordRepeatMismatch");
    return;
  }

  pending.value = true;
  try {
    await changeOwnPassword({
      current_password: currentPassword.value,
      new_password: nextPassword.value,
    });
    dialogOpen.value = false;
    currentPassword.value = "";
    nextPassword.value = "";
    confirmPassword.value = "";
    await sessionStore.signOut();
    await router.push({
      name: "login",
      query: { refresh: String(Date.now()) },
    });
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.signInFailed");
  } finally {
    pending.value = false;
  }
}

function openPasswordDialog() {
  dialogOpen.value = true;
  errorMessage.value = "";
  currentPassword.value = "";
  nextPassword.value = "";
  confirmPassword.value = "";
}

function closePasswordDialog() {
  dialogOpen.value = false;
  pending.value = false;
  errorMessage.value = "";
  currentPassword.value = "";
  nextPassword.value = "";
  confirmPassword.value = "";
}
</script>

<template>
  <AppWorkspaceShell
    :body="t('account.body')"
    :eyebrow="t('account.eyebrow')"
    :title="t('account.title')"
  >
    <section class="account-grid">
      <GlassPanel class="account-card account-card--primary" tone="warm">
        <div class="account-card__section-head">
          <div>
            <p class="section-label">{{ t("account.securityTitle") }}</p>
            <h3>{{ t("account.passwordCardTitle") }}</h3>
          </div>
          <button type="button" @click="openPasswordDialog">
            {{ t("users.changePassword") }}
          </button>
        </div>

        <div class="account-card__identity">
          <div class="account-card__avatar">
            {{ sessionStore.user?.username?.slice(0, 1)?.toUpperCase() ?? "U" }}
          </div>
          <div>
            <p class="section-label">{{ t("account.username") }}</p>
            <h2>{{ sessionStore.user?.username }}</h2>
            <p>{{ sessionStore.user?.email || t("account.emailFallback") }}</p>
          </div>
        </div>
        <div class="account-card__status">
          <span class="account-card__badge">
            {{
              sessionStore.isAdmin
                ? t("account.roleAdmin")
                : t("account.roleUser")
            }}
          </span>
          <p>{{ roleCopy }}</p>
        </div>
      </GlassPanel>
    </section>

    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="dialogOpen"
          class="account-overlay"
          role="dialog"
          aria-modal="true"
          @click.self="closePasswordDialog"
        >
          <GlassPanel class="account-dialog" tone="warm">
            <div class="account-dialog__head">
              <div>
                <p class="section-label">{{ t("account.securityTitle") }}</p>
                <h3>{{ t("users.passwordDialogTitle") }}</h3>
              </div>
            </div>

            <div class="account-dialog__body">
              <p class="account-dialog__copy">
                {{ t("account.passwordDialogBody") }}
              </p>

              <label class="account-dialog__field">
                <span>{{ t("account.passwordCurrentLabel") }}</span>
                <input
                  v-model="currentPassword"
                  :placeholder="t('account.passwordCurrentPlaceholder')"
                  autocomplete="current-password"
                  type="password"
                />
              </label>

              <label class="account-dialog__field">
                <span>{{ t("account.passwordNewLabel") }}</span>
                <input
                  v-model="nextPassword"
                  :placeholder="t('account.passwordNewPlaceholder')"
                  autocomplete="new-password"
                  type="password"
                />
              </label>

              <label class="account-dialog__field">
                <span>{{ t("account.passwordRepeatLabel") }}</span>
                <input
                  v-model="confirmPassword"
                  :placeholder="t('account.passwordRepeatPlaceholder')"
                  autocomplete="new-password"
                  type="password"
                />
              </label>

              <p v-if="errorMessage" class="account-dialog__error">
                {{ errorMessage }}
              </p>

              <div class="account-dialog__actions">
                <button
                  class="account-dialog__primary"
                  type="button"
                  @click="submitPasswordChange"
                >
                  {{
                    pending ? t("common.loading") : t("users.changePassword")
                  }}
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
.account-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.account-card {
  display: grid;
  gap: 1rem;
  padding: 1.25rem;
  transition:
    background-color 180ms var(--curve-swift),
    border-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    transform 220ms var(--curve-buoy);
}

.account-card__section-head {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-start;
}

.account-card__section-head h3,
.account-card__section-head p,
.account-card__body {
  margin: 0;
}

.account-card__identity {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.account-card__avatar {
  display: inline-grid;
  place-items: center;
  width: 4rem;
  height: 4rem;
  border-radius: 1.25rem;
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 84%,
    var(--bg-accent)
  );
  border: 1px solid var(--border-strong);
  color: var(--text-main);
  font-size: 1.4rem;
  font-weight: 500;
  flex-shrink: 0;
}

.account-card__identity h2,
.account-card__identity p,
.account-card__head h3,
.account-card__body,
.account-card__status p {
  margin: 0;
}

.account-card__identity h2 {
  margin-top: 0.4rem;
  font-size: clamp(1.6rem, 2vw, 2rem);
  line-height: 1.12;
  font-weight: 500;
}

.account-card__identity p:last-child,
.account-card__body,
.account-card__status p {
  color: var(--text-muted);
}

.account-card__status {
  display: grid;
  gap: 0.7rem;
}

.account-card__badge {
  display: inline-flex;
  width: fit-content;
  min-height: 2rem;
  align-items: center;
  padding: 0.2rem 0.72rem;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--signal-blue) 14%, transparent);
  color: var(--signal-blue);
  font-size: 0.78rem;
  font-weight: 700;
}

.account-card__head {
  display: flex;
  gap: 0.8rem;
  align-items: start;
}

.account-card__head h3 {
  margin-top: 0.45rem;
  font-size: 1.18rem;
  line-height: 1.2;
  font-weight: 500;
}

@media (pointer: fine) {
  .account-card:hover {
    background: var(--bg-surface-strong);
    border-color: var(--border-strong);
    box-shadow: var(--card-hover-shadow);
    transform: translateY(-2px);
  }
}

.account-card:focus-within {
  background: var(--bg-surface-strong);
  border-color: var(--border-strong);
  box-shadow: var(--card-focus-shadow);
  transform: translateY(-1px);
}

.account-overlay {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(20, 20, 19, 0.34);
}

.account-dialog {
  width: min(32rem, 100%);
  display: grid;
  gap: 1rem;
  padding: 1rem;
}

.account-dialog__head,
.account-dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  align-items: center;
}

.account-dialog__head h3,
.account-dialog__copy,
.account-dialog__error,
.account-dialog__field span {
  margin: 0;
}

.account-dialog__body {
  display: grid;
  gap: 0.9rem;
}

.account-dialog__copy {
  color: var(--text-muted);
  line-height: 1.6;
}

.account-dialog__field {
  display: grid;
  gap: 0.45rem;
}

.account-dialog__field span {
  color: var(--text-main);
  font-size: 0.88rem;
  font-weight: 600;
}

.account-dialog__error {
  color: var(--signal-red);
  font-size: 0.9rem;
}

.account-dialog__primary {
  border-color: transparent;
  background: var(--button-primary-bg);
  color: #fff;
  font-weight: 700;
}
</style>
