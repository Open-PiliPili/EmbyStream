<script setup lang="ts">
import { Icon } from "@iconify/vue";

import BrandMark from "@/components/ui/BrandMark.vue";
import GlassPanel from "@/components/ui/GlassPanel.vue";
import SignalPill from "@/components/ui/SignalPill.vue";
import { APP_NAME } from "@/constants/app";

withDefaults(
  defineProps<{
    eyebrow: string;
    title: string;
    body: string;
    switchLabel: string;
    switchTo: string;
    themeLabel: string;
    themeMode: "light" | "dark";
    themePreference: "light" | "dark" | "system";
    storyLabel: string;
    storyTitle: string;
    storyBody: string;
    signals: string[];
    panelTitle: string;
    panelBody: string;
    backgroundImage?: string;
    backgroundAlt?: string;
  }>(),
  {
    backgroundImage: "",
    backgroundAlt: "",
  },
);

defineEmits<{
  toggleTheme: [];
}>();
</script>

<template>
  <main class="auth-stage" :class="{ 'auth-stage--artwork': backgroundImage }">
    <div
      v-if="backgroundImage"
      aria-hidden="true"
      class="auth-stage__background"
      :style="{ backgroundImage: `url(${backgroundImage})` }"
    ></div>

    <section class="auth-stage__frame">
      <header class="auth-stage__bar">
        <span class="auth-stage__brand">
          <BrandMark class="auth-stage__brand-mark" />
          <span>{{ APP_NAME }}</span>
        </span>
        <button
          class="auth-stage__theme-switch"
          type="button"
          :aria-label="themeLabel"
          @click="$emit('toggleTheme')"
        >
          <Icon
            :icon="
              themePreference === 'system'
                ? 'ph:desktop'
                : themeMode === 'dark'
                  ? 'ph:sun'
                  : 'ph:moon'
            "
            width="18"
          />
        </button>
      </header>

      <div class="auth-stage__content">
        <section class="auth-stage__story">
          <h2>{{ storyTitle }}</h2>
          <p class="auth-stage__story-copy">{{ storyBody }}</p>

          <div class="auth-stage__signals">
            <SignalPill
              v-for="signal in signals"
              :key="signal"
              :label="signal"
            />
          </div>

          <div class="auth-stage__story-panel">
            <p class="auth-stage__story-panel-label">{{ panelTitle }}</p>
            <p class="auth-stage__story-panel-copy">{{ panelBody }}</p>
          </div>
        </section>

        <GlassPanel class="auth-stage__panel" tone="warm">
          <div class="auth-stage__hero">
            <p class="eyebrow">{{ eyebrow }}</p>
            <h1>{{ title }}</h1>
            <p class="lede">{{ body }}</p>
          </div>

          <slot />

          <RouterLink class="auth-stage__switch" :to="switchTo">
            {{ switchLabel }}
          </RouterLink>
        </GlassPanel>
      </div>
    </section>
  </main>
</template>

<style scoped>
.auth-stage {
  position: relative;
  min-height: 100vh;
  min-height: 100dvh;
  padding: clamp(1rem, 2vw, 1.5rem);
  background: var(--bg-app);
  overflow: hidden;
}

.auth-stage__background {
  position: absolute;
  inset: 0;
  z-index: 0;
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  transform: scale(1.04);
  filter: saturate(0.86) contrast(0.96) brightness(0.7);
}

.auth-stage__background::after {
  content: "";
  position: absolute;
  inset: 0;
  background:
    linear-gradient(180deg, rgba(15, 15, 14, 0.26), rgba(15, 15, 14, 0.48)),
    linear-gradient(135deg, rgba(201, 100, 66, 0.2), rgba(0, 0, 0, 0.22));
}

.auth-stage__frame {
  position: relative;
  z-index: 1;
  display: grid;
  gap: clamp(1.05rem, 2.4vw, 2rem);
  max-width: min(1440px, calc(100vw - 2rem));
  min-height: calc(100vh - clamp(2rem, 4vw, 3rem));
  min-height: calc(100dvh - clamp(2rem, 4vw, 3rem));
  margin: 0 auto;
  padding: clamp(1rem, 2vw, 1.35rem);
  border: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
  border-radius: 32px;
  background: color-mix(in srgb, var(--bg-app) 42%, transparent);
  box-shadow: 0 24px 70px rgba(0, 0, 0, 0.16);
  backdrop-filter: blur(22px) saturate(120%);
  -webkit-backdrop-filter: blur(22px) saturate(120%);
}

.auth-stage__bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  min-height: 2.45rem;
}

.auth-stage__brand {
  display: inline-flex;
  align-items: center;
  gap: 0.7rem;
  color: var(--text-main);
  font-size: 0.96rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  margin-top: -0.18rem;
}

.auth-stage__brand-mark {
  flex-shrink: 0;
}

.auth-stage__theme-switch {
  min-height: 2.5rem;
  padding-inline: 1rem;
  margin-top: -0.2rem;
}

.auth-stage__content {
  display: grid;
  grid-template-columns: minmax(0, 1.18fr) minmax(24rem, 32rem);
  gap: clamp(1.25rem, 2.4vw, 2rem);
  align-items: stretch;
  flex: 1;
  padding: 0.5rem clamp(0.7rem, 2vw, 1.65rem) clamp(1rem, 2vw, 1.5rem);
  transform: translateY(-0.65rem);
}

.auth-stage__story {
  display: grid;
  align-content: start;
  gap: 1.1rem;
  padding: clamp(1.5rem, 3.6vw, 2.8rem) clamp(0.9rem, 1.8vw, 1.45rem)
    clamp(1.35rem, 3vw, 2.35rem) clamp(0.55rem, 1.4vw, 1rem);
}

.auth-stage__story-copy,
.auth-stage__story-panel-label,
.auth-stage__story-panel-copy {
  margin: 0;
}

.auth-stage__story h2 {
  margin: 0;
  max-width: 12ch;
  color: var(--text-main);
  font-size: clamp(2.9rem, 6vw, 5.1rem);
  font-weight: 500;
  line-height: 1.06;
}

.auth-stage__story-copy {
  max-width: 34rem;
  color: color-mix(in srgb, var(--text-main) 78%, white 22%);
  font-size: 1.04rem;
  line-height: 1.72;
}

.auth-stage__signals {
  display: flex;
  flex-wrap: wrap;
  gap: 0.7rem;
  margin-top: 0.2rem;
}

.auth-stage__story-panel {
  width: min(100%, 26rem);
  padding: 1rem 1.05rem 1.1rem;
  border: 1px solid color-mix(in srgb, var(--border-strong) 84%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.05);
}

.auth-stage__story-panel-label {
  color: var(--text-main);
  font-size: 0.9rem;
  font-weight: 600;
}

.auth-stage__story-panel-copy {
  margin-top: 0.45rem;
  color: color-mix(in srgb, var(--text-main) 72%, white 28%);
  font-size: 0.92rem;
  line-height: 1.6;
}

.auth-stage__panel {
  display: grid;
  align-content: start;
  gap: 1.3rem;
  min-height: min(42rem, 100%);
  padding: clamp(1.4rem, 3vw, 2rem);
  align-self: start;
  margin: clamp(1.25rem, 2.6vw, 2.2rem) clamp(0.35rem, 1vw, 0.85rem)
    clamp(1rem, 2.1vw, 1.8rem) clamp(0.15rem, 0.8vw, 0.5rem);
  background: color-mix(in srgb, var(--bg-surface-strong) 84%, transparent);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--border-subtle) 72%, transparent),
    0 18px 40px rgba(0, 0, 0, 0.12);
}

.auth-stage__hero {
  display: grid;
  gap: 0.8rem;
}

.auth-stage__hero h1,
.auth-stage__hero .lede {
  margin: 0;
}

.auth-stage__hero h1 {
  font-size: clamp(2.2rem, 5vw, 3.3rem);
  line-height: 1.08;
  font-weight: 500;
}

.auth-stage__switch {
  width: fit-content;
  color: var(--signal-accent);
  font-size: 0.92rem;
  font-weight: 500;
}

.auth-stage__switch:hover {
  color: var(--text-main);
}

@media (max-width: 960px) {
  .auth-stage__content {
    grid-template-columns: 1fr;
    padding: 0;
    transform: none;
  }

  .auth-stage__story {
    padding: 1rem 0.2rem 0;
  }

  .auth-stage__story h2 {
    max-width: none;
    font-size: clamp(2.3rem, 10vw, 4rem);
  }

  .auth-stage__panel {
    min-height: auto;
    margin: 0;
  }
}

@media (max-width: 640px) {
  .auth-stage {
    padding:
      calc(0.9rem + env(safe-area-inset-top, 0px))
      calc(0.9rem + env(safe-area-inset-right, 0px))
      calc(0.9rem + env(safe-area-inset-bottom, 0px))
      calc(0.9rem + env(safe-area-inset-left, 0px));
  }

  .auth-stage__frame {
    max-width: 100%;
    min-height: calc(
      100dvh - env(safe-area-inset-top, 0px) - env(safe-area-inset-bottom, 0px)
    );
    padding: 0.95rem;
    border-radius: 26px;
  }

  .auth-stage__bar {
    align-items: flex-start;
  }

  .auth-stage__theme-switch {
    min-height: 2.5rem;
    padding-inline: 0.9rem;
  }
}
</style>
