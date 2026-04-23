<script setup lang="ts">
import { UI_LABELS } from "@/constants/app";

const _props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description: string;
    confirmLabel: string;
    cancelLabel: string;
    showClose?: boolean;
    confirmTone?: "primary" | "danger";
    inputLabel?: string;
    inputValue?: string;
    inputPlaceholder?: string;
  }>(),
  {
    showClose: true,
    confirmTone: "primary",
    inputLabel: "",
    inputValue: "",
    inputPlaceholder: "",
  },
);

const _emit = defineEmits<{
  close: [];
  confirm: [];
  "update:inputValue": [value: string];
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="modal-pop">
      <div
        v-if="open"
        class="action-dialog__overlay"
        role="dialog"
        aria-modal="true"
        @click.self="$emit('close')"
      >
        <div class="action-dialog">
          <div class="action-dialog__head">
            <div>
              <p class="section-label">{{ UI_LABELS.dialogEyebrow }}</p>
              <h3>{{ title }}</h3>
            </div>
            <button
              v-if="showClose"
              class="action-dialog__close"
              type="button"
              :aria-label="cancelLabel"
              @click="$emit('close')"
            >
              <svg
                aria-hidden="true"
                class="action-dialog__close-icon"
                viewBox="0 0 24 24"
              >
                <path
                  d="M6 6l12 12M18 6L6 18"
                  fill="none"
                  stroke="currentColor"
                  stroke-linecap="round"
                  stroke-width="2"
                />
              </svg>
            </button>
          </div>

          <div class="action-dialog__body">
            <p>{{ description }}</p>

            <label v-if="inputLabel" class="action-dialog__field">
              <span>{{ inputLabel }}</span>
              <input
                :placeholder="inputPlaceholder"
                :value="inputValue"
                type="text"
                @input="
                  $emit(
                    'update:inputValue',
                    ($event.target as HTMLInputElement).value,
                  )
                "
              />
            </label>

            <div class="action-dialog__actions">
              <button type="button" @click="$emit('close')">
                {{ cancelLabel }}
              </button>
              <button
                class="action-dialog__confirm"
                :class="`action-dialog__confirm--${confirmTone}`"
                type="button"
                @click="$emit('confirm')"
              >
                {{ confirmLabel }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.action-dialog__overlay {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(20, 20, 19, 0.3);
}

.action-dialog {
  width: min(28rem, 100%);
  padding: 1.25rem;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface-strong);
  box-shadow: var(--shadow-medium);
}

.action-dialog__head {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: start;
}

.action-dialog__head > button {
  border-color: var(--border-strong);
  box-shadow: none;
}

.action-dialog__head h3,
.action-dialog__body p,
.action-dialog__field span {
  margin: 0;
}

.action-dialog__head h3 {
  margin-top: 0.45rem;
  font-size: 1.3rem;
  line-height: 1.14;
  font-weight: 500;
}

.action-dialog__body {
  display: grid;
  gap: 1rem;
  margin-top: 1rem;
}

.action-dialog__body p {
  color: var(--text-muted);
  line-height: 1.7;
}

.action-dialog__field {
  display: grid;
  gap: 0.45rem;
}

.action-dialog__field span {
  color: var(--text-main);
  font-size: 0.88rem;
  font-weight: 600;
}

.action-dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.action-dialog__confirm {
  min-height: 2.45rem;
  padding-inline: 1rem;
  border-color: transparent;
  color: #fff;
  box-shadow: none;
}

.action-dialog__confirm--primary {
  background: var(--button-primary-bg);
}

.action-dialog__confirm--primary:hover {
  background: var(--button-primary-hover);
}

.action-dialog__confirm--danger {
  background: #b53333;
}

.action-dialog__actions button:not(.action-dialog__confirm) {
  min-width: 6.25rem;
  border-color: var(--border-subtle);
  box-shadow: none;
}

.modal-pop-enter-active,
.modal-pop-leave-active {
  transition: opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from,
.modal-pop-leave-to {
  opacity: 0;
}

.modal-pop-enter-active .action-dialog,
.modal-pop-leave-active .action-dialog {
  transition:
    transform 320ms var(--curve-spring),
    opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from .action-dialog,
.modal-pop-leave-to .action-dialog {
  opacity: 0;
  transform: scale(0.96);
}

.action-dialog__close {
  min-width: 2.7rem;
  min-height: 2.7rem;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.action-dialog__close-icon {
  display: block;
}
</style>
