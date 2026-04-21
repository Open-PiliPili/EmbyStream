<script setup lang="ts">
defineProps<{
  title: string;
  steps: string[];
  currentStep: number;
  currentTitle?: string;
  currentPurpose?: string;
  currentEffect?: string;
}>();

defineEmits<{
  select: [index: number];
}>();
</script>

<template>
  <section class="stepper">
    <div class="stepper__header">
      <p class="section-label">{{ title }}</p>
      <p class="stepper__progress">
        {{ currentStep + 1 }} / {{ steps.length }}
      </p>
    </div>

    <ol class="stepper__segments">
      <li
        v-for="(step, index) in steps"
        :key="step"
        :class="{ active: currentStep === index }"
        @click="$emit('select', index)"
      >
        <span class="stepper__index">{{ index + 1 }}</span>
        <span class="stepper__label">{{ step }}</span>
      </li>
    </ol>

    <div v-if="currentTitle" class="stepper__detail">
      <p class="stepper__detail-title">{{ currentTitle }}</p>
      <p v-if="currentPurpose" class="stepper__detail-copy">
        {{ currentPurpose }}
      </p>
      <p v-if="currentEffect" class="stepper__detail-effect">
        {{ currentEffect }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.stepper {
  display: grid;
  gap: 1rem;
}

.stepper__header {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: center;
}

.stepper__progress {
  margin: 0;
  color: var(--text-faint);
  font-size: 0.84rem;
  font-weight: 600;
}

.stepper__segments {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
  gap: 0.65rem;
  margin: 0;
  list-style: none;
  padding: 0;
}

.stepper__segments li {
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: 0.75rem;
  min-height: 3.25rem;
  padding: 0.8rem 0.95rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  color: var(--text-muted);
  background: var(--bg-elevated);
  cursor: pointer;
  box-shadow: var(--shadow-soft);
}

.stepper__segments li.active {
  color: var(--text-main);
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 88%,
    var(--bg-accent)
  );
  border-color: var(--border-accent);
  box-shadow: 0 0 0 1px
    color-mix(in srgb, var(--brand-secondary) 14%, transparent);
}

@media (pointer: fine) {
  .stepper__segments li:hover {
    color: var(--text-main);
    border-color: var(--border-strong);
  }
}

.stepper__index {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  width: 1.55rem;
  height: 1.55rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--signal-blue) 18%, transparent);
  color: var(--signal-blue);
  font-size: 0.75rem;
  font-weight: 700;
}

.stepper__label {
  font-size: 0.9rem;
  font-weight: 600;
  line-height: 1.35;
}

.stepper__detail {
  padding: 1rem 1.1rem;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  box-shadow: var(--shadow-soft);
}

.stepper__detail-title,
.stepper__detail-copy,
.stepper__detail-effect {
  margin: 0;
}

.stepper__detail-title {
  color: var(--text-main);
  font-size: 1.08rem;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.stepper__detail-copy {
  margin-top: 0.5rem;
  color: var(--text-muted);
  font-size: 0.94rem;
  line-height: 1.6;
}

.stepper__detail-effect {
  margin-top: 0.6rem;
  color: var(--signal-accent);
  font-size: 0.86rem;
  font-weight: 600;
}
</style>
