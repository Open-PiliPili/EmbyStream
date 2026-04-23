<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";

import {
  ApiError,
  getLoginBackground,
  getRegistrationSettings,
} from "@/api/client";
import type { BackgroundItem } from "@/api/types";
import AuthStageShell from "@/components/blocks/AuthStageShell.vue";
import { STORAGE_KEYS } from "@/constants/app";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import { useTheme } from "@/composables/useTheme";
import { useSessionStore } from "@/stores/session";

const { t, tm } = useI18n();
const route = useRoute();
const router = useRouter();
const sessionStore = useSessionStore();
const { theme, themePreference, cycleThemePreference } = useTheme();

const form = reactive({
  login: "",
  password: "",
});
const pending = ref(false);
const errorMessage = ref("");
const backgroundItems = ref<BackgroundItem[]>([]);
const registrationEnabled = ref(false);
const toastMessage = ref("");
let toastTimer: number | undefined;

useDocumentLocale();

const nextThemePreferenceLabel = computed(() => {
  if (themePreference.value === "system") {
    return t("common.themeLight");
  }

  if (themePreference.value === "light") {
    return t("common.themeDark");
  }

  return t("common.themeSystem");
});

const accentLabel = computed(() =>
  t("common.themeSwitchTo", { mode: nextThemePreferenceLabel.value }),
);

const submitDisabled = computed(
  () => pending.value || !form.login.trim() || !form.password.trim(),
);
const authSignals = computed(() => tm("auth.signals") as string[]);

async function loadLoginArtwork() {
  try {
    const response = await getLoginBackground();
    backgroundItems.value = pickRotatedArtwork(response.items);
  } catch {
    backgroundItems.value = [];
  }
}

function pickRotatedArtwork(items: BackgroundItem[]) {
  if (items.length === 0) {
    return [];
  }

  const previousImage = window.sessionStorage.getItem(
    STORAGE_KEYS.loginArtwork,
  );
  const candidates = items.filter((item) => item.image_url !== previousImage);
  const pool = candidates.length > 0 ? candidates : items;
  const nextItem = pool[Math.floor(Math.random() * pool.length)];

  if (nextItem) {
    window.sessionStorage.setItem(
      STORAGE_KEYS.loginArtwork,
      nextItem.image_url,
    );
    return [nextItem];
  }

  return items.slice(0, 1);
}

watch(
  () => route.fullPath,
  () => {
    loadLoginArtwork();
  },
  { immediate: true },
);

watch(
  () => route.query.notice,
  async (notice) => {
    if (notice !== "registration_closed") {
      return;
    }

    showToast(t("auth.registerClosedToast"));
    const nextQuery = { ...route.query };
    delete nextQuery.notice;
    await router.replace({ name: "login", query: nextQuery });
  },
  { immediate: true },
);

onMounted(async () => {
  await loadRegistrationAvailability();
});

onBeforeUnmount(() => {
  if (toastTimer !== undefined) {
    window.clearTimeout(toastTimer);
  }
});

async function loadRegistrationAvailability() {
  try {
    const response = await getRegistrationSettings();
    registrationEnabled.value = response.registration_enabled;
  } catch {
    registrationEnabled.value = false;
  }
}

function showToast(message: string) {
  toastMessage.value = message;
  if (toastTimer !== undefined) {
    window.clearTimeout(toastTimer);
  }
  toastTimer = window.setTimeout(() => {
    toastMessage.value = "";
    toastTimer = undefined;
  }, 2800);
}

async function handleSwitch() {
  if (!registrationEnabled.value) {
    showToast(t("auth.registerClosedToast"));
    return;
  }

  await router.push({ name: "register" });
}

async function submit() {
  if (submitDisabled.value) {
    return;
  }

  pending.value = true;
  errorMessage.value = "";

  try {
    await sessionStore.signIn(form);
    await router.push({ name: "drafts" });
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.signInFailed");
  } finally {
    pending.value = false;
  }
}
</script>

<template>
  <AuthStageShell
    :background-alt="backgroundItems[0]?.title"
    :background-image="backgroundItems[0]?.image_url"
    :body="t('auth.login.body')"
    :eyebrow="t('auth.login.eyebrow')"
    :panel-body="t('auth.panelBody')"
    :panel-title="t('auth.panelTitle')"
    :signals="authSignals"
    :story-body="t('auth.wallBody')"
    :story-label="t('auth.storyLabel')"
    :story-title="t('auth.wallTitle')"
    :switch-label="t('auth.login.switch')"
    switch-to="/register"
    :theme-label="accentLabel"
    :theme-mode="theme"
    :theme-preference="themePreference"
    :toast-message="toastMessage"
    :title="t('auth.login.title')"
    @switch="handleSwitch"
    @toggle-theme="cycleThemePreference"
  >
    <form class="auth-form" @submit.prevent="submit">
      <label>
        <span>{{ t("auth.login.loginLabel") }}</span>
        <input v-model="form.login" autocomplete="username" type="text" />
      </label>
      <label>
        <span>{{ t("auth.login.passwordLabel") }}</span>
        <input
          v-model="form.password"
          autocomplete="current-password"
          type="password"
        />
      </label>

      <p class="auth-form__hint">{{ t("auth.login.helper") }}</p>
      <p
        v-if="errorMessage"
        class="auth-form__feedback auth-form__feedback--error"
      >
        {{ errorMessage }}
      </p>

      <button
        :disabled="submitDisabled"
        class="auth-form__submit"
        type="submit"
      >
        {{ pending ? t("common.loading") : t("auth.login.submit") }}
      </button>
    </form>
  </AuthStageShell>
</template>

<style scoped>
.auth-form {
  display: grid;
  gap: 0.95rem;
}

.auth-form label {
  display: grid;
  gap: 0.5rem;
}

.auth-form span {
  color: var(--text-main);
  font-size: 0.88rem;
  font-weight: 600;
}

.auth-form input {
  border-color: var(--field-border);
  background: var(--field-bg);
}

.auth-form__hint,
.auth-form__feedback {
  margin: 0;
  font-size: 0.88rem;
  line-height: 1.55;
}

.auth-form__hint {
  color: var(--text-faint);
}

.auth-form__feedback--error {
  color: var(--signal-red);
}

.auth-form__submit {
  min-height: 2.95rem;
  border-color: transparent;
  background: var(--button-primary-bg);
  color: #ffffff;
  font-weight: 700;
}

.auth-form__submit:hover:not(:disabled) {
  background: var(--button-primary-hover);
  border-color: transparent;
}

.auth-form__submit:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}
</style>
