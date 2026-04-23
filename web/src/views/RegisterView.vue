<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

import { ApiError, getRegistrationSettings } from "@/api/client";
import AuthStageShell from "@/components/blocks/AuthStageShell.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import { useTheme } from "@/composables/useTheme";
import { useSessionStore } from "@/stores/session";

const { t, tm } = useI18n();
const router = useRouter();
const sessionStore = useSessionStore();
const { theme, themePreference, cycleThemePreference } = useTheme();

const form = reactive({
  username: "",
  email: "",
  password: "",
});
const pending = ref(false);
const errorMessage = ref("");
const registrationEnabled = ref(true);

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
  () =>
    pending.value ||
    !registrationEnabled.value ||
    !form.username.trim() ||
    !form.password.trim() ||
    !form.email.trim(),
);
const authSignals = computed(() => tm("auth.signals") as string[]);

onMounted(async () => {
  try {
    const response = await getRegistrationSettings();
    registrationEnabled.value = response.registration_enabled;
  } catch {
    registrationEnabled.value = false;
  }

  if (!registrationEnabled.value) {
    await router.replace({
      name: "login",
      query: { notice: "registration_closed" },
    });
  }
});

async function handleSwitch() {
  await router.push({ name: "login" });
}

async function submit() {
  if (submitDisabled.value) {
    return;
  }

  pending.value = true;
  errorMessage.value = "";

  try {
    await sessionStore.signUp({
      username: form.username,
      email: form.email,
      password: form.password,
    });
    await router.push({ name: "login" });
  } catch (error) {
    errorMessage.value =
      error instanceof ApiError ? error.message : t("errors.registerFailed");
  } finally {
    pending.value = false;
  }
}
</script>

<template>
  <AuthStageShell
    :body="t('auth.register.body')"
    :eyebrow="t('auth.register.eyebrow')"
    :panel-body="t('auth.panelBody')"
    :panel-title="t('auth.panelTitle')"
    :signals="authSignals"
    :story-body="t('auth.wallBody')"
    :story-label="t('auth.storyLabel')"
    :story-title="t('auth.wallTitle')"
    :switch-label="t('auth.register.switch')"
    switch-to="/login"
    :theme-label="accentLabel"
    :theme-mode="theme"
    :theme-preference="themePreference"
    :title="t('auth.register.title')"
    @switch="handleSwitch"
    @toggle-theme="cycleThemePreference"
  >
    <form class="auth-form" @submit.prevent="submit">
      <label>
        <span>{{ t("auth.register.usernameLabel") }}</span>
        <input v-model="form.username" autocomplete="username" type="text" />
      </label>
      <label>
        <span>{{ t("auth.register.emailLabel") }}</span>
        <input v-model="form.email" autocomplete="email" type="email" />
      </label>
      <label>
        <span>{{ t("auth.register.passwordLabel") }}</span>
        <input
          v-model="form.password"
          autocomplete="new-password"
          type="password"
        />
      </label>

      <p class="auth-form__hint">{{ t("auth.register.helper") }}</p>
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
        {{ pending ? t("common.loading") : t("auth.register.submit") }}
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
