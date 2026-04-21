<script setup lang="ts">
import { computed, ref } from "vue";

const _props = withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    autocomplete?: string;
    multiline?: boolean;
    rows?: number;
  }>(),
  {
    placeholder: "",
    autocomplete: "off",
    multiline: false,
    rows: 4,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const visible = ref(false);

const inputType = computed(() => (visible.value ? "text" : "password"));

function updateValue(event: Event) {
  const target = event.target as HTMLInputElement | HTMLTextAreaElement;
  emit("update:modelValue", target.value);
}
</script>

<template>
  <div class="sensitive-input">
    <input
      v-if="!multiline"
      :autocomplete="autocomplete"
      class="sensitive-input__control sensitive-input__control--single"
      :placeholder="placeholder"
      :type="inputType"
      :value="modelValue"
      @input="updateValue"
    />
    <textarea
      v-else
      :autocomplete="autocomplete"
      class="sensitive-input__control sensitive-input__control--multi"
      :class="{ 'sensitive-input__control--masked': !visible }"
      :placeholder="placeholder"
      :rows="rows"
      :value="modelValue"
      @input="updateValue"
    ></textarea>

    <button
      class="sensitive-input__toggle"
      type="button"
      @click="visible = !visible"
    >
      <svg
        v-if="!visible"
        aria-hidden="true"
        class="sensitive-input__icon"
        viewBox="0 0 24 24"
      >
        <path
          d="M12 5C6.5 5 2.1 8.5 1 12c1.1 3.5 5.5 7 11 7s9.9-3.5 11-7c-1.1-3.5-5.5-7-11-7Z"
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.8"
        />
        <circle
          cx="12"
          cy="12"
          r="3.2"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
        />
      </svg>
      <svg
        v-else
        aria-hidden="true"
        class="sensitive-input__icon"
        viewBox="0 0 24 24"
      >
        <path
          d="M3 3l18 18"
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-width="1.8"
        />
        <path
          d="M10.6 6.2A11.8 11.8 0 0 1 12 6c5.5 0 9.9 3.5 11 7a11.9 11.9 0 0 1-4.1 5.4"
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.8"
        />
        <path
          d="M6.2 6.8A12.1 12.1 0 0 0 1 12c1.1 3.5 5.5 7 11 7 1.4 0 2.7-.2 3.9-.6"
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.8"
        />
        <path
          d="M9.9 9.9a3.2 3.2 0 0 0 4.2 4.2"
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.8"
        />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.sensitive-input {
  position: relative;
  min-height: 2.95rem;
}

.sensitive-input__control {
  width: 100%;
  min-height: 2.95rem;
  padding-right: 3.35rem;
}

.sensitive-input__control--multi {
  min-height: 8rem;
}

.sensitive-input__control--masked {
  -webkit-text-security: disc;
  filter: blur(0.16rem);
}

.sensitive-input__toggle {
  appearance: none;
  position: absolute;
  top: 0.5rem;
  right: 0.55rem;
  z-index: 2;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.2rem;
  height: 2.2rem;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: transparent;
  box-shadow: none;
  color: var(--text-muted);
}

.sensitive-input__icon {
  width: 17px;
  height: 17px;
  display: block;
}

@media (pointer: fine) {
  .sensitive-input__toggle:hover {
    color: var(--text-main);
  }
}
</style>
