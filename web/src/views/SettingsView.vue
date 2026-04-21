<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";

import AppWorkspaceShell from "@/components/blocks/AppWorkspaceShell.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import { useDocumentLocale } from "@/composables/useDocumentLocale";
import { useTheme } from "@/composables/useTheme";
import { useTypography } from "@/composables/useTypography";
import { setAppLocale } from "@/locales";

const { t, locale } = useI18n();
const isMobile = ref(false);
const {
  chineseFont,
  englishFont,
  codeFont,
  renderFont,
  renderWeight,
  chineseWeight,
  englishWeight,
  codeWeight,
  previewReadingFont,
} = useTypography();
const { theme, themePreference, setThemePreference } = useTheme();

useDocumentLocale();

const localeOptions = computed(() => [
  { value: "zh-CN", label: t("settings.localeZhCn") },
  { value: "zh-TW", label: t("settings.localeZhTw") },
  { value: "en", label: t("settings.localeEn") },
]);

const themeOptions = computed(() => [
  { value: "system", label: t("common.themeSystem") },
  { value: "light", label: t("common.themeLight") },
  { value: "dark", label: t("common.themeDark") },
]);

const chineseFontOptions = computed(() => [
  { value: "source-han-sans", label: t("account.fontZhSourceHanSans") },
  { value: "lxgw-wenkai", label: t("account.fontZhWenkai") },
  { value: "pingfang", label: t("account.fontZhPingfang") },
]);

const englishFontOptions = computed(() => [
  { value: "source-serif", label: t("account.fontEnSourceSerif") },
  { value: "source-sans", label: t("account.fontEnSourceSans") },
  { value: "inter", label: t("account.fontEnInter") },
  { value: "plus-jakarta-sans", label: t("account.fontEnPlusJakartaSans") },
]);

const codeFontOptions = computed(() => [
  { value: "fira-code", label: t("account.fontCodeFiraCode") },
  { value: "menlo", label: t("account.fontCodeMenlo") },
  { value: "source-code-pro", label: t("account.fontEnSourceCodePro") },
]);

const renderFontOptions = computed(() => [
  { value: "menlo", label: t("settings.renderFontMenlo") },
  { value: "source-sans", label: t("settings.renderFontSans") },
  { value: "source-serif", label: t("settings.renderFontSerif") },
]);

const weightOptions = computed(() => [
  { value: "400", label: t("settings.renderWeightNormal") },
  { value: "500", label: t("settings.renderWeightRegular") },
  { value: "600", label: t("settings.renderWeightSemibold") },
  { value: "700", label: t("settings.renderWeightBold") },
]);

async function updateLocale(nextLocale: string) {
  await setAppLocale(nextLocale as "zh-CN" | "zh-TW" | "en");
  locale.value = nextLocale;
}

function updateThemePreference(nextPreference: string) {
  setThemePreference(nextPreference as "light" | "dark" | "system");
}

function syncMobileState() {
  if (typeof window === "undefined") {
    return;
  }

  isMobile.value = window.matchMedia("(max-width: 980px)").matches;
}

onMounted(() => {
  syncMobileState();
  window.addEventListener("resize", syncMobileState);
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", syncMobileState);
});
</script>

<template>
  <AppWorkspaceShell
    :body="t('settings.body')"
    :eyebrow="t('settings.eyebrow')"
    :title="t('settings.title')"
  >
    <section class="settings-grid">
      <GlassPanel class="settings-card">
        <div class="settings-card__head">
          <Icon icon="ph:moon-stars" width="20" />
          <div>
            <p class="section-label">{{ t("settings.themeLabel") }}</p>
            <h2>{{ t("settings.themeTitle") }}</h2>
          </div>
        </div>
        <p class="settings-card__body">{{ t("settings.themeBody") }}</p>
        <label class="settings-card__field">
          <span>{{ t("settings.themeLabel") }}</span>
          <select
            :value="themePreference"
            @change="
              updateThemePreference(
                ($event.target as HTMLSelectElement).value,
              )
            "
          >
            <option
              v-for="option in themeOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </option>
          </select>
        </label>
        <p class="settings-card__hint">
          {{ t("settings.themeCurrent", {
            mode: theme === "dark" ? t("common.themeDark") : t("common.themeLight"),
          }) }}
        </p>
      </GlassPanel>

      <GlassPanel class="settings-card">
        <div class="settings-card__head">
          <Icon icon="ph:globe-hemisphere-west" width="20" />
          <div>
            <p class="section-label">{{ t("settings.languageLabel") }}</p>
            <h2>{{ t("settings.languageTitle") }}</h2>
          </div>
        </div>
        <p class="settings-card__body">{{ t("settings.languageBody") }}</p>
        <label class="settings-card__field">
          <span>{{ t("common.language") }}</span>
          <select
            :value="locale"
            @change="updateLocale(($event.target as HTMLSelectElement).value)"
          >
            <option
              v-for="option in localeOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </option>
          </select>
        </label>
      </GlassPanel>

      <GlassPanel
        v-if="!isMobile"
        class="settings-card settings-card--typography"
      >
        <div class="settings-card__head">
          <Icon icon="ph:text-aa" width="20" />
          <div>
            <p class="section-label">{{ t("account.fontsLabel") }}</p>
            <h2>{{ t("account.fontsTitle") }}</h2>
          </div>
        </div>
        <p class="settings-card__body">{{ t("account.fontsBody") }}</p>

        <div class="settings-card__font-grid">
          <label class="settings-card__field">
            <span>{{ t("settings.renderFontLabel") }}</span>
            <select v-model="renderFont">
              <option
                v-for="option in renderFontOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="settings-card__field">
            <span>{{ t("settings.renderWeightLabel") }}</span>
            <select v-model="renderWeight">
              <option
                v-for="option in weightOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="settings-card__field">
            <span>{{ t("account.fontZhLabel") }}</span>
            <select v-model="chineseFont">
              <option
                v-for="option in chineseFontOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="settings-card__field">
            <span>{{ t("settings.zhWeightLabel") }}</span>
            <select v-model="chineseWeight">
              <option
                v-for="option in weightOptions"
                :key="`zh-${option.value}`"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="settings-card__field">
            <span>{{ t("account.fontEnLabel") }}</span>
            <select v-model="englishFont">
              <option
                v-for="option in englishFontOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="settings-card__field">
            <span>{{ t("settings.enWeightLabel") }}</span>
            <select v-model="englishWeight">
              <option
                v-for="option in weightOptions"
                :key="`en-${option.value}`"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="settings-card__field">
            <span>{{ t("account.fontCodeLabel") }}</span>
            <select v-model="codeFont">
              <option
                v-for="option in codeFontOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="settings-card__field">
            <span>{{ t("settings.codeWeightLabel") }}</span>
            <select v-model="codeWeight">
              <option
                v-for="option in weightOptions"
                :key="`code-${option.value}`"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>
        </div>

        <div
          class="settings-card__preview"
          :style="{ fontFamily: previewReadingFont }"
        >
          <p class="settings-card__preview-zh">
            {{ t("account.fontPreviewZh") }}
          </p>
          <p class="settings-card__preview-en">
            {{ t("account.fontPreviewEn") }}
          </p>
        </div>

        <pre
          class="settings-card__code-preview"
        ><code>{{ t("account.fontPreviewCode") }}</code></pre>
      </GlassPanel>
    </section>
  </AppWorkspaceShell>
</template>

<style scoped>
.settings-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.settings-card {
  display: grid;
  gap: 1rem;
  padding: 1.25rem;
  background: var(--bg-surface);
  transition:
    background-color 180ms var(--curve-swift),
    border-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    transform 220ms var(--curve-buoy);
}

.settings-card--typography {
  border-color: color-mix(
    in srgb,
    var(--brand-secondary) 14%,
    var(--border-subtle)
  );
  background: color-mix(in srgb, var(--bg-surface) 94%, var(--surface-warm));
}

.settings-card__head {
  display: flex;
  gap: 0.8rem;
  align-items: start;
}

.settings-card__head h2,
.settings-card__body,
.settings-card__preview p,
.settings-card__code-preview {
  margin: 0;
}

.settings-card__head h2 {
  margin-top: 0.45rem;
  font-size: 1.18rem;
  line-height: 1.2;
  font-weight: 500;
}

.settings-card__body {
  color: var(--text-muted);
}

.settings-card__hint {
  margin: 0;
  color: var(--text-faint);
  font-size: 0.85rem;
}

.settings-card__field {
  display: grid;
  gap: 0.45rem;
}

.settings-card__field span {
  color: var(--text-main);
  font-size: 0.88rem;
  font-weight: 600;
}

.settings-card__font-grid {
  display: grid;
  grid-template-columns: repeat(8, minmax(0, 1fr));
  gap: 0.9rem;
}

.settings-card__preview {
  display: grid;
  gap: 0.55rem;
  padding: 1rem 1.05rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

.settings-card__preview-zh {
  color: var(--text-main);
  font-size: 1.05rem;
  line-height: 1.7;
}

.settings-card__preview-en {
  color: var(--text-muted);
  font-size: 0.95rem;
  line-height: 1.65;
}

.settings-card__code-preview {
  padding: 1rem 1.05rem;
  border: 1px solid
    color-mix(in srgb, var(--code-accent) 14%, var(--border-subtle));
  border-radius: var(--radius-md);
  background: var(--code-bg);
  color: var(--code-fg);
  font-family: var(--mono-font);
  font-size: 0.9rem;
  line-height: 1.7;
  white-space: pre-wrap;
}

@media (pointer: fine) {
  .settings-card:hover {
    background: color-mix(
      in srgb,
      var(--bg-surface-strong) 94%,
      var(--surface-warm)
    );
    border-color: var(--border-strong);
    box-shadow: var(--card-hover-shadow);
    transform: translateY(-2px);
  }
}

.settings-card:focus-within {
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 94%,
    var(--surface-warm)
  );
  border-color: var(--border-strong);
  box-shadow: var(--card-focus-shadow);
  transform: translateY(-1px);
}

@media (max-width: 1024px) {
  .settings-card__font-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .settings-card__font-grid {
    grid-template-columns: 1fr;
  }
}
</style>
