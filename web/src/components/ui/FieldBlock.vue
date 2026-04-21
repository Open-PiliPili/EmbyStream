<script setup lang="ts">
defineProps<{
  label: string;
  hint?: string;
}>();
</script>

<template>
  <label class="field">
    <span class="field__label">{{ label }}</span>
    <slot />
    <span class="field__hint" :class="{ 'field__hint--empty': !hint }">
      {{ hint ?? "" }}
    </span>
  </label>
</template>

<style scoped>
.field {
  display: grid;
  gap: 0.6rem;
  height: 100%;
  align-content: start;
}

.field__label {
  color: var(--text-main);
  font-size: 0.9rem;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.field__hint {
  color: var(--text-faint);
  font-size: 0.84rem;
  line-height: 1.55;
  min-height: 2.6rem;
}

.field__hint--empty {
  visibility: hidden;
}
</style>
