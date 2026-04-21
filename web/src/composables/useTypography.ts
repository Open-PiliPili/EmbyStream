import { computed, ref, watchEffect } from "vue";

import { STORAGE_KEYS } from "@/constants/app";

export type ChineseFontKey = "source-han-sans" | "lxgw-wenkai" | "pingfang";
export type EnglishFontKey =
  | "source-serif"
  | "source-sans"
  | "inter"
  | "plus-jakarta-sans";
export type CodeFontKey = "fira-code" | "menlo" | "source-code-pro";
export type RenderFontKey = "menlo" | "source-sans" | "source-serif";
export type FontWeightKey = "400" | "500" | "600" | "700";

const chineseFont = ref<ChineseFontKey>("source-han-sans");
const englishFont = ref<EnglishFontKey>("source-serif");
const codeFont = ref<CodeFontKey>("fira-code");
const renderFont = ref<RenderFontKey>("menlo");
const renderWeight = ref<FontWeightKey>("700");
const chineseWeight = ref<FontWeightKey>("600");
const englishWeight = ref<FontWeightKey>("700");
const codeWeight = ref<FontWeightKey>("600");
let initialized = false;

const chineseFontStacks: Record<ChineseFontKey, string[]> = {
  "source-han-sans": [
    '"Source Han Sans SC"',
    '"Noto Sans SC"',
    '"PingFang SC"',
    '"Hiragino Sans GB"',
    '"Microsoft YaHei"',
  ],
  "lxgw-wenkai": ['"LXGW WenKai"', '"Kaiti SC"', '"STKaiti"', '"KaiTi"'],
  pingfang: ['"PingFang SC"', '"Hiragino Sans GB"', '"Microsoft YaHei"'],
};

const englishFontStacks: Record<EnglishFontKey, string[]> = {
  "source-serif": [
    '"Source Serif 4"',
    '"Iowan Old Style"',
    '"Palatino Linotype"',
  ],
  "source-sans": ['"Source Sans 3"', '"Avenir Next"', '"Helvetica Neue"'],
  inter: ["Inter", '"Helvetica Neue"', '"Avenir Next"'],
  "plus-jakarta-sans": ['"Plus Jakarta Sans"', "Inter", '"Helvetica Neue"'],
};

const codeFontStacks: Record<CodeFontKey, string[]> = {
  "fira-code": [
    '"Fira Code"',
    "Menlo",
    '"Source Code Pro"',
    '"SFMono-Regular"',
  ],
  "source-code-pro": ['"Source Code Pro"', '"SFMono-Regular"', '"Monaco"'],
  menlo: ["Menlo", '"SFMono-Regular"', '"Monaco"', '"Source Code Pro"'],
};

const renderFontStacks: Record<RenderFontKey, string[]> = {
  menlo: ["Menlo", '"Avenir Next"', '"Helvetica Neue"'],
  "source-sans": ['"Source Sans 3"', '"Avenir Next"', '"Helvetica Neue"'],
  "source-serif": [
    '"Source Serif 4"',
    '"Iowan Old Style"',
    '"Palatino Linotype"',
  ],
};

function getInitialChineseFontSetting(): ChineseFontKey {
  if (typeof window === "undefined") {
    return "source-han-sans";
  }

  const stored = window.localStorage.getItem(STORAGE_KEYS.chineseFont);
  if (stored && stored in chineseFontStacks) {
    return stored as ChineseFontKey;
  }

  return "source-han-sans";
}

function getInitialEnglishFontSetting(): EnglishFontKey {
  if (typeof window === "undefined") {
    return "source-serif";
  }

  const stored = window.localStorage.getItem(STORAGE_KEYS.englishFont);
  if (stored && stored in englishFontStacks) {
    return stored as EnglishFontKey;
  }

  return "source-serif";
}

function getInitialCodeFontSetting(): CodeFontKey {
  if (typeof window === "undefined") {
    return "fira-code";
  }

  const stored = window.localStorage.getItem(STORAGE_KEYS.codeFont);
  if (stored && stored in codeFontStacks) {
    return stored as CodeFontKey;
  }

  return "fira-code";
}

function getInitialRenderFontSetting(): RenderFontKey {
  if (typeof window === "undefined") {
    return "menlo";
  }

  const stored = window.localStorage.getItem(STORAGE_KEYS.renderFont);
  if (stored && stored in renderFontStacks) {
    return stored as RenderFontKey;
  }

  return "menlo";
}

function getStoredWeight(
  storageKey: string,
  fallback: FontWeightKey,
): FontWeightKey {
  if (typeof window === "undefined") {
    return fallback;
  }

  const stored = window.localStorage.getItem(storageKey);
  if (
    stored === "400" ||
    stored === "500" ||
    stored === "600" ||
    stored === "700"
  ) {
    return stored;
  }

  return fallback;
}

function buildReadingFamily(
  englishKey: EnglishFontKey,
  chineseKey: ChineseFontKey,
) {
  return [
    ...englishFontStacks[englishKey],
    ...chineseFontStacks[chineseKey],
    "serif",
  ].join(", ");
}

function buildUiFamily(renderKey: RenderFontKey, chineseKey: ChineseFontKey) {
  return [
    ...renderFontStacks[renderKey],
    ...chineseFontStacks[chineseKey],
    "sans-serif",
  ].join(", ");
}

function buildMonoFamily(codeKey: CodeFontKey) {
  return [
    ...codeFontStacks[codeKey],
    '"SFMono-Regular"',
    "Menlo",
    "monospace",
  ].join(", ");
}

export function useTypography() {
  if (!initialized) {
    initialized = true;
    chineseFont.value = getInitialChineseFontSetting();
    englishFont.value = getInitialEnglishFontSetting();
    codeFont.value = getInitialCodeFontSetting();
    renderFont.value = getInitialRenderFontSetting();
    renderWeight.value = getStoredWeight(STORAGE_KEYS.renderWeight, "700");
    chineseWeight.value = getStoredWeight(STORAGE_KEYS.chineseWeight, "600");
    englishWeight.value = getStoredWeight(STORAGE_KEYS.englishWeight, "700");
    codeWeight.value = getStoredWeight(STORAGE_KEYS.codeWeight, "600");

    watchEffect(() => {
      if (typeof document === "undefined") {
        return;
      }

      const root = document.documentElement;
      root.style.setProperty(
        "--font-reading-family",
        buildReadingFamily(englishFont.value, chineseFont.value),
      );
      root.style.setProperty(
        "--font-display-family",
        buildReadingFamily(englishFont.value, chineseFont.value),
      );
      root.style.setProperty(
        "--font-ui-family",
        buildUiFamily(renderFont.value, chineseFont.value),
      );
      root.style.setProperty("--font-ui-weight", renderWeight.value);
      root.style.setProperty("--font-zh-weight", chineseWeight.value);
      root.style.setProperty("--font-en-weight", englishWeight.value);
      root.style.setProperty("--font-code-weight", codeWeight.value);
      root.style.setProperty(
        "--font-mono-family",
        buildMonoFamily(codeFont.value),
      );

      window.localStorage.setItem(STORAGE_KEYS.chineseFont, chineseFont.value);
      window.localStorage.setItem(STORAGE_KEYS.englishFont, englishFont.value);
      window.localStorage.setItem(STORAGE_KEYS.codeFont, codeFont.value);
      window.localStorage.setItem(STORAGE_KEYS.renderFont, renderFont.value);
      window.localStorage.setItem(
        STORAGE_KEYS.renderWeight,
        renderWeight.value,
      );
      window.localStorage.setItem(
        STORAGE_KEYS.chineseWeight,
        chineseWeight.value,
      );
      window.localStorage.setItem(
        STORAGE_KEYS.englishWeight,
        englishWeight.value,
      );
      window.localStorage.setItem(STORAGE_KEYS.codeWeight, codeWeight.value);
    });
  }

  const previewReadingFont = computed(() =>
    buildReadingFamily(englishFont.value, chineseFont.value),
  );

  return {
    chineseFont,
    englishFont,
    codeFont,
    renderFont,
    renderWeight,
    chineseWeight,
    englishWeight,
    codeWeight,
    previewReadingFont,
  };
}
