<script setup lang="ts">
defineProps<{
  items: Array<{
    key: string;
    label: string;
  }>;
  activeKey?: string;
}>();

defineEmits<{
  select: [key: string];
}>();
</script>

<template>
  <div class="pill-tabs">
    <button
      v-for="item in items"
      :key="item.key"
      :class="{ active: activeKey === item.key }"
      type="button"
      @click="$emit('select', item.key)"
    >
      {{ item.label }}
    </button>
  </div>
</template>

<style scoped>
.pill-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
}

.pill-tabs button {
  min-height: 2.5rem;
  border-radius: var(--radius-pill);
  padding: 0.58rem 0.95rem;
  background: var(--button-secondary-bg);
  color: var(--text-muted);
  font-size: 0.88rem;
  font-weight: 600;
  box-shadow: var(--shadow-soft);
  -webkit-tap-highlight-color: transparent;
}

.pill-tabs button:focus-visible {
  outline: none;
  border-color: var(--border-accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--focus-ring) 22%, transparent);
}

@media (pointer: fine) {
  .pill-tabs button:hover {
    color: var(--text-main);
    border-color: var(--border-strong);
  }
}

.pill-tabs button.active {
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 88%,
    var(--bg-accent)
  );
  color: var(--signal-blue);
  border-color: var(--border-accent);
  box-shadow: 0 0 0 1px
    color-mix(in srgb, var(--brand-secondary) 14%, transparent);
}
</style>
