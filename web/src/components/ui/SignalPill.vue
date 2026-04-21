<script setup lang="ts">
defineProps<{
  label: string;
}>();
</script>

<template>
  <span class="signal-pill">{{ label }}</span>
</template>

<style scoped>
.signal-pill {
  display: inline-flex;
  align-items: center;
  min-height: 2rem;
  padding: 0.35rem 0.72rem;
  border-radius: var(--radius-pill);
  border: 1px solid
    color-mix(in srgb, var(--signal-blue) 16%, var(--border-subtle));
  background: color-mix(in srgb, var(--bg-app) 40%, var(--bg-surface-strong));
  color: var(--signal-accent);
  font-size: 0.76rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  box-shadow: 0 0 0 1px
    color-mix(in srgb, var(--border-subtle) 72%, transparent);
}
</style>
