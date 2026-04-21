<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";

import { CONSOLE_BRANDING, STORAGE_KEYS } from "@/constants/app";
import { useTheme } from "@/composables/useTheme";
import { useTypography } from "@/composables/useTypography";

const route = useRoute();

useTheme();
useTypography();

const routeTransitionName = computed(() =>
  route.meta.requiresAuth ? "workspace-shift" : "auth-shift",
);

const routeTransitionKey = computed(() => route.fullPath);

onMounted(() => {
  if (typeof window === "undefined") {
    return;
  }

  if (window.sessionStorage.getItem(STORAGE_KEYS.consoleSignature) === "1") {
    return;
  }

  window.sessionStorage.setItem(STORAGE_KEYS.consoleSignature, "1");
  console.log(
    `%c${CONSOLE_BRANDING.title}%c`,
    [
      "padding: 6px 10px",
      "border-radius: 999px",
      "background: #c96442",
      "color: #faf9f5",
      "font-weight: 700",
      "font-family: SF Pro Display, Avenir Next, sans-serif",
    ].join(";"),
    [
      "color: #5e5d59",
      "font-weight: 600",
      "font-family: SF Pro Text, Avenir Next, sans-serif",
      "line-height: 1.8",
    ].join(";"),
  );
  console.log(
    `%cHint:%c ${CONSOLE_BRANDING.hint.replace(/^Hint:\s*/, "")}`,
    "color:#c96442;font-weight:700",
    "color:#5e5d59",
  );
});
</script>

<template>
  <div class="app-shell">
    <div class="app-shell__viewport">
      <router-view v-slot="{ Component }">
        <Transition :name="routeTransitionName">
          <component
            :is="Component"
            :key="routeTransitionKey"
            class="app-shell__page"
          />
        </Transition>
      </router-view>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
  background: var(--bg-app);
}

.app-shell__viewport {
  position: relative;
  min-height: 100vh;
  overflow-x: clip;
}

.app-shell__page {
  width: 100%;
  min-height: 100vh;
}
</style>

<style>
.workspace-shift-enter-active,
.workspace-shift-leave-active,
.auth-shift-enter-active,
.auth-shift-leave-active {
  transition:
    opacity 320ms var(--curve-swift),
    transform 320ms var(--curve-swift);
}

.workspace-shift-enter-active,
.workspace-shift-leave-active,
.auth-shift-enter-active,
.auth-shift-leave-active {
  position: absolute;
  inset: 0;
  width: 100%;
  pointer-events: none;
}

.workspace-shift-enter-from {
  opacity: 0;
  transform: scale(0.992);
}

.workspace-shift-leave-to {
  opacity: 0;
  transform: scale(1.008);
}

.auth-shift-enter-from {
  opacity: 0;
  transform: scale(0.992);
}

.auth-shift-leave-to {
  opacity: 0;
  transform: scale(1.008);
}
</style>
