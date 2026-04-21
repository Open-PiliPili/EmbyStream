import { computed, ref, watchEffect } from "vue";

import { STORAGE_KEYS } from "@/constants/app";

export type ThemeMode = "light" | "dark";
export type ThemePreference = ThemeMode | "system";

const themePreference = ref<ThemePreference>("system");
const systemTheme = ref<ThemeMode>("light");
const theme = computed<ThemeMode>(() =>
  themePreference.value === "system" ? systemTheme.value : themePreference.value,
);
let initialized = false;
let hydrated = false;
let previousTheme: ThemeMode | null = null;
let colorSchemeMediaQuery: MediaQueryList | null = null;

function ensureThemeMeta(name: string) {
  let tag = document.querySelector<HTMLMetaElement>(`meta[name="${name}"]`);
  if (!tag) {
    tag = document.createElement("meta");
    tag.name = name;
    document.head.appendChild(tag);
  }
  return tag;
}

function refreshThemeMeta(name: string, content: string) {
  const tag = ensureThemeMeta(name);
  tag.content = content;

  // iOS standalone mode can be sticky about status-bar related meta updates.
  // Re-appending the node helps force the shell to pick up the latest value.
  if (name === "apple-mobile-web-app-status-bar-style" && tag.parentNode) {
    tag.parentNode.removeChild(tag);
    document.head.appendChild(tag);
  }
}

function getInitialThemePreference(): ThemePreference {
  if (typeof window === "undefined") {
    return "system";
  }

  const stored = window.localStorage.getItem(STORAGE_KEYS.themePreference);
  if (stored === "light" || stored === "dark" || stored === "system") {
    return stored;
  }

  return "system";
}

function readSystemTheme(): ThemeMode {
  if (typeof window === "undefined") {
    return "light";
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function syncSystemTheme() {
  systemTheme.value = readSystemTheme();
}

function ensureSystemThemeObserver() {
  if (typeof window === "undefined" || colorSchemeMediaQuery) {
    return;
  }

  colorSchemeMediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  syncSystemTheme();

  const handleChange = () => {
    syncSystemTheme();
  };

  if (typeof colorSchemeMediaQuery.addEventListener === "function") {
    colorSchemeMediaQuery.addEventListener("change", handleChange);
    return;
  }

  colorSchemeMediaQuery.addListener(handleChange);
}

function isStandaloneIosPwa() {
  if (typeof window === "undefined" || typeof navigator === "undefined") {
    return false;
  }

  const standalone = window.matchMedia("(display-mode: standalone)").matches ||
    (window.navigator as Navigator & { standalone?: boolean }).standalone === true;
  const isiOS = /iPhone|iPad|iPod/i.test(navigator.userAgent);

  return standalone && isiOS;
}

export function useTheme() {
  if (!initialized) {
    initialized = true;
    themePreference.value = getInitialThemePreference();
    ensureSystemThemeObserver();

    watchEffect(() => {
      if (typeof document === "undefined") {
        return;
      }

      const activeTheme = theme.value;

      document.documentElement.dataset.theme = activeTheme;
      document.documentElement.dataset.themePreference = themePreference.value;
      document.documentElement.style.colorScheme = activeTheme;
      window.localStorage.setItem(
        STORAGE_KEYS.themePreference,
        themePreference.value,
      );

      const themeColor = activeTheme === "dark" ? "#1b1a18" : "#f5f4ed";
      document.documentElement.style.backgroundColor = themeColor;

      if (document.body) {
        document.body.style.backgroundColor = themeColor;
      }

      refreshThemeMeta("theme-color", themeColor);
      refreshThemeMeta(
        "apple-mobile-web-app-status-bar-style",
        activeTheme === "dark" ? "black-translucent" : "default",
      );

      if (
        hydrated &&
        previousTheme !== null &&
        previousTheme !== activeTheme &&
        isStandaloneIosPwa()
      ) {
        window.setTimeout(() => {
          window.location.reload();
        }, 40);
      }

      previousTheme = activeTheme;
      hydrated = true;
    });
  }

  function cycleThemePreference() {
    if (themePreference.value === "system") {
      themePreference.value = "light";
      return;
    }

    if (themePreference.value === "light") {
      themePreference.value = "dark";
      return;
    }

    themePreference.value = "system";
  }

  function setTheme(nextTheme: ThemeMode) {
    themePreference.value = nextTheme;
  }

  function setThemePreference(nextTheme: ThemePreference) {
    themePreference.value = nextTheme;
  }

  return {
    theme,
    themePreference,
    toggleTheme: cycleThemePreference,
    cycleThemePreference,
    setTheme,
    setThemePreference,
  };
}
