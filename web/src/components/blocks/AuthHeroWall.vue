<script setup lang="ts">
import SignalPill from "@/components/ui/SignalPill.vue";

defineProps<{
  title: string;
  body: string;
  signals: string[];
}>();
</script>

<template>
  <section class="hero-wall">
    <div class="hero-wall__stack">
      <div class="hero-wall__poster hero-wall__poster--main"></div>
      <div class="hero-wall__poster hero-wall__poster--top"></div>
      <div class="hero-wall__poster hero-wall__poster--side"></div>
    </div>
    <div class="hero-wall__copy">
      <h1>{{ title }}</h1>
      <p>{{ body }}</p>
      <div class="hero-wall__signals">
        <SignalPill v-for="signal in signals" :key="signal" :label="signal" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.hero-wall {
  display: grid;
  gap: 2rem;
  align-items: center;
}

.hero-wall__stack {
  position: relative;
  min-height: 380px;
}

.hero-wall__poster {
  position: absolute;
  border-radius: 16px;
  border: 1px solid var(--border-subtle);
  background:
    linear-gradient(180deg, var(--bg-soft), transparent 72%),
    linear-gradient(135deg, var(--bg-accent), transparent 55%),
    var(--bg-surface);
  box-shadow: var(--shadow-medium);
  overflow: hidden;
}

.hero-wall__poster::after {
  content: "";
  position: absolute;
  inset: 0;
  background:
    linear-gradient(180deg, transparent 0%, rgba(0, 0, 0, 0.08) 100%),
    repeating-linear-gradient(
      90deg,
      color-mix(in srgb, var(--signal-blue) 10%, transparent) 0 1px,
      transparent 1px 36px
    );
}

.hero-wall__poster--main {
  inset: 2.5rem 3rem 0 0;
}

.hero-wall__poster--top {
  top: 0;
  right: 7rem;
  width: 9rem;
  height: 11rem;
  transform: rotate(-7deg);
}

.hero-wall__poster--side {
  right: 0;
  bottom: 2rem;
  width: 10rem;
  height: 14rem;
  transform: rotate(9deg);
}

.hero-wall__copy h1 {
  margin: 0;
  font-size: clamp(2.8rem, 6vw, 5.6rem);
  line-height: 0.92;
  letter-spacing: -0.05em;
}

.hero-wall__copy p {
  max-width: 32rem;
  margin: 1.1rem 0 0;
  color: var(--text-muted);
  font-size: 1rem;
}

.hero-wall__signals {
  display: flex;
  flex-wrap: wrap;
  gap: 0.7rem;
  margin-top: 1.4rem;
}

@media (max-width: 960px) {
  .hero-wall__stack {
    min-height: 260px;
  }

  .hero-wall__poster--top,
  .hero-wall__poster--side {
    width: 7rem;
    height: 9rem;
  }
}
</style>
