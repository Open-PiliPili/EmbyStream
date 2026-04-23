<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import hljs from "highlight.js/lib/core";
import ini from "highlight.js/lib/languages/ini";
import javascript from "highlight.js/lib/languages/javascript";
import nginx from "highlight.js/lib/languages/nginx";
import plaintext from "highlight.js/lib/languages/plaintext";
import toml from "highlight.js/lib/languages/ini";
import yaml from "highlight.js/lib/languages/yaml";
import { useI18n } from "vue-i18n";

hljs.registerLanguage("ini", ini);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("nginx", nginx);
hljs.registerLanguage("plaintext", plaintext);
hljs.registerLanguage("toml", toml);
hljs.registerLanguage("yaml", yaml);

const props = defineProps<{
  fileName: string;
  content: string;
  language: string;
  collapsed: boolean;
  downloadHref?: string;
  openLabel?: string;
  expandLabel: string;
  collapseLabel: string;
  downloadLabel: string;
  immersive?: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
  open: [];
}>();

const { t } = useI18n();
const highlighted = ref("");
const copied = ref(false);
let idleHandle: number | undefined;
let copiedTimer: number | undefined;

const shouldHighlight = computed(() => props.immersive || !props.collapsed);

watch(
  () => [props.content, props.language, shouldHighlight.value],
  () => {
    if (!shouldHighlight.value) {
      highlighted.value = "";
      return;
    }

    scheduleHighlight();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (idleHandle !== undefined && typeof window !== "undefined") {
    window.cancelIdleCallback?.(idleHandle);
    window.clearTimeout(idleHandle);
  }
  if (copiedTimer !== undefined && typeof window !== "undefined") {
    window.clearTimeout(copiedTimer);
  }
});

function scheduleHighlight() {
  if (typeof window === "undefined") {
    highlighted.value = buildHighlight();
    return;
  }

  if (idleHandle !== undefined) {
    window.cancelIdleCallback?.(idleHandle);
    window.clearTimeout(idleHandle);
  }

  const task = () => {
    highlighted.value = buildHighlight();
  };

  if (typeof window.requestIdleCallback === "function") {
    idleHandle = window.requestIdleCallback(task, {
      timeout: 180,
    }) as unknown as number;
    return;
  }

  idleHandle = window.setTimeout(task, 32);
}

function buildHighlight() {
  const language = pickHighlightLanguage(props.language);

  return hljs.highlight(props.content, {
    language,
    ignoreIllegals: true,
  }).value;
}

function pickHighlightLanguage(language: string) {
  return hljs.getLanguage(language) ? language : "plaintext";
}

async function copyContent() {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      await navigator.clipboard.writeText(props.content);
    } else if (!copyWithFallback()) {
      return;
    }

    copied.value = true;

    if (copiedTimer !== undefined) {
      window.clearTimeout(copiedTimer);
    }

    copiedTimer = window.setTimeout(() => {
      copied.value = false;
      copiedTimer = undefined;
    }, 1800);
  } catch {
    copied.value = false;
  }
}

function copyWithFallback() {
  if (typeof document === "undefined") {
    return false;
  }

  const textarea = document.createElement("textarea");
  textarea.value = props.content;
  textarea.setAttribute("readonly", "true");
  textarea.setAttribute("aria-hidden", "true");
  textarea.style.position = "fixed";
  textarea.style.top = "0";
  textarea.style.left = "-9999px";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();
  textarea.setSelectionRange(0, textarea.value.length);
  const copied = document.execCommand("copy");
  document.body.removeChild(textarea);
  return copied;
}

function triggerDownload() {
  if (typeof document === "undefined" || !props.downloadHref) {
    return;
  }

  const anchor = document.createElement("a");
  anchor.href = props.downloadHref;
  anchor.rel = "noopener noreferrer";
  anchor.download = "";
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
}

function toggleCollapsed() {
  if (props.immersive) {
    return;
  }

  emit("toggle");
}
</script>

<template>
  <div
    class="file-preview"
    :class="{
      'file-preview--immersive': immersive,
      'file-preview--collapsed': collapsed,
    }"
  >
    <div
      class="file-preview__head"
      :role="immersive ? undefined : 'button'"
      :tabindex="immersive ? undefined : 0"
      @click="toggleCollapsed"
      @keydown.enter.prevent="toggleCollapsed"
      @keydown.space.prevent="toggleCollapsed"
    >
      <div class="file-preview__meta">
        <span class="file-preview__name">{{ fileName }}</span>
        <span class="file-preview__language">{{ language }}</span>
      </div>
      <div class="file-preview__actions">
        <button
          :aria-label="copied ? t('common.copied') : t('common.copy')"
          type="button"
          @click.stop="copyContent"
        >
          <Icon :icon="copied ? 'ph:check' : 'ph:copy'" width="16" />
        </button>
        <button
          v-if="downloadHref"
          :aria-label="downloadLabel"
          type="button"
          @click.stop="triggerDownload"
        >
          <Icon icon="ph:download-simple" width="16" />
        </button>
        <button
          v-if="openLabel && !immersive"
          :aria-label="openLabel"
          type="button"
          @click.stop="emit('open')"
        >
          <Icon icon="ph:arrows-out-simple" width="16" />
        </button>
        <button
          v-if="!immersive"
          :aria-label="collapsed ? expandLabel : collapseLabel"
          type="button"
          @click.stop="emit('toggle')"
        >
          <svg
            aria-hidden="true"
            class="file-preview__toggle-icon"
            :class="{ 'file-preview__toggle-icon--collapsed': collapsed }"
            fill="none"
            viewBox="0 0 16 16"
          >
            <rect
              class="file-preview__toggle-line"
              x="3"
              y="3.2"
              width="10"
              height="1.6"
              rx="0.8"
            />
            <rect
              class="file-preview__toggle-line"
              x="3"
              y="11.2"
              width="10"
              height="1.6"
              rx="0.8"
            />
            <rect
              class="file-preview__toggle-line file-preview__toggle-line--panel"
              x="5"
              y="6.4"
              width="6"
              height="3.2"
              rx="1.1"
            />
          </svg>
        </button>
      </div>
    </div>
    <!-- eslint-disable-next-line vue/no-v-html -->
    <!-- highlight.js output is escaped and language-validated before rendering -->
    <pre v-if="!collapsed"><code v-html="highlighted"></code></pre>
  </div>
</template>

<style scoped>
.file-preview {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 0;
  overflow: hidden;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  box-shadow: var(--shadow-soft);
  content-visibility: auto;
}

.file-preview__head {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: center;
  padding: 0.95rem 1rem;
  border-bottom: 1px solid var(--border-subtle);
  cursor: pointer;
  transition:
    background-color 180ms var(--curve-swift),
    border-color 180ms var(--curve-swift);
}

.file-preview__head:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: -2px;
}

.file-preview__meta {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
}

.file-preview__name,
.file-preview__language {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-preview__name {
  color: var(--text-main);
  font-size: 0.93rem;
  font-weight: 600;
}

.file-preview__language {
  color: var(--text-faint);
  font-size: 0.76rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.file-preview__actions {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.file-preview__actions button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.15rem;
  height: 2.15rem;
  padding: 0;
  border: 1px solid var(--border-subtle);
  border-radius: 999px;
  background: var(--button-secondary-bg);
  color: var(--button-secondary-text);
  box-shadow: var(--shadow-soft);
}

@media (pointer: fine) {
  .file-preview__head:hover {
    background: color-mix(in srgb, var(--bg-surface-strong) 82%, transparent);
  }

  .file-preview__actions button:hover {
    background: var(--button-secondary-hover);
  }
}

.file-preview--collapsed .file-preview__head {
  border-bottom-color: transparent;
}

.file-preview__toggle-icon {
  width: 16px;
  height: 16px;
  display: block;
}

.file-preview__toggle-line {
  fill: currentColor;
  transition:
    transform 180ms var(--curve-swift),
    opacity 180ms var(--curve-swift);
}

.file-preview__toggle-line--panel {
  transform-origin: center;
}

.file-preview__toggle-icon--collapsed .file-preview__toggle-line--panel {
  transform: translateY(2px) scaleY(0.45);
}

.file-preview pre {
  margin: 0;
  padding: 1rem 1rem 1.1rem;
  color: var(--code-fg);
  background: var(--code-bg);
  min-height: 0;
  max-height: clamp(14rem, 36vh, 18rem);
  overflow: auto;
  white-space: pre;
  font-family: var(--mono-font);
}

.file-preview--immersive pre {
  min-height: 24rem;
  max-height: calc(100vh - 12rem);
}

.file-preview :deep(.hljs-comment),
.file-preview :deep(.hljs-quote) {
  color: var(--code-muted);
}

.file-preview :deep(.hljs-attr),
.file-preview :deep(.hljs-attribute),
.file-preview :deep(.hljs-keyword),
.file-preview :deep(.hljs-selector-tag) {
  color: var(--code-keyword);
}

.file-preview :deep(.hljs-string),
.file-preview :deep(.hljs-number),
.file-preview :deep(.hljs-literal) {
  color: var(--code-string);
}

.file-preview :deep(.hljs-number),
.file-preview :deep(.hljs-literal) {
  color: var(--code-number);
}

.file-preview :deep(.hljs-title),
.file-preview :deep(.hljs-section),
.file-preview :deep(.hljs-selector-id),
.file-preview :deep(.hljs-selector-class) {
  color: var(--code-title);
}

.file-preview :deep(.hljs-built_in),
.file-preview :deep(.hljs-type),
.file-preview :deep(.hljs-variable),
.file-preview :deep(.hljs-template-variable) {
  color: var(--code-builtins);
}

.file-preview :deep(.hljs-params),
.file-preview :deep(.hljs-symbol),
.file-preview :deep(.hljs-bullet),
.file-preview :deep(.hljs-link) {
  color: var(--code-accent);
}
</style>
