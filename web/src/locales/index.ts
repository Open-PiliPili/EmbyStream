import { createI18n } from "vue-i18n";

import { STORAGE_KEYS } from "@/constants/app";

type AppLocale = "zh-CN" | "zh-TW" | "en";
type LocaleMessages = Record<string, unknown>;

const DEFAULT_LOCALE: AppLocale = "zh-CN";

const localeLoaders: Record<AppLocale, () => Promise<LocaleMessages>> = {
  "zh-CN": () => import("./zh-CN.json").then((module) => module.default),
  "zh-TW": () => import("./zh-TW.json").then((module) => module.default),
  en: () => import("./en.json").then((module) => module.default),
};

const loadedLocales = new Set<AppLocale>();

export const i18n = createI18n({
  legacy: false,
  locale: DEFAULT_LOCALE,
  fallbackLocale: "en",
  messages: {},
});

function normalizeLocaleValue(value?: string | null): AppLocale {
  if (value === "zh-CN" || value === "zh-TW" || value === "en") {
    return value;
  }
  return DEFAULT_LOCALE;
}

export async function loadLocaleMessages(locale: AppLocale) {
  if (loadedLocales.has(locale)) {
    return;
  }

  const messages = await localeLoaders[locale]();
  i18n.global.setLocaleMessage(locale, messages);
  loadedLocales.add(locale);
}

export async function setAppLocale(locale: AppLocale) {
  const normalizedLocale = normalizeLocaleValue(locale);
  await loadLocaleMessages(normalizedLocale);
  i18n.global.locale.value = normalizedLocale;
  if (typeof window !== "undefined") {
    window.localStorage.setItem(STORAGE_KEYS.locale, normalizedLocale);
  }
}

export function getInitialLocaleSetting(): AppLocale {
  if (typeof window === "undefined") {
    return DEFAULT_LOCALE;
  }

  return normalizeLocaleValue(window.localStorage.getItem(STORAGE_KEYS.locale));
}

export default i18n;
