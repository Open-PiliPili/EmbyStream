<script setup lang="ts">
withDefaults(
  defineProps<{
    tone?: "cool" | "warm";
  }>(),
  {
    tone: "cool",
  },
);
</script>

<template>
  <section class="panel" :class="`panel--${tone}`">
    <slot />
  </section>
</template>

<style scoped>
.panel {
  --panel-bg: var(--bg-surface);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-subtle);
  background: var(--panel-bg);
  box-shadow: var(--shadow-medium);
  transition:
    border-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    background-color 180ms var(--curve-swift);
}

.panel--warm {
  --panel-bg: color-mix(in srgb, var(--bg-surface) 96%, var(--surface-warm));
}
</style>
