<template>
  <svg
    aria-hidden="true"
    class="brand-mark"
    fill="none"
    viewBox="0 0 32 32"
    xmlns="http://www.w3.org/2000/svg"
  >
    <rect
      class="brand-mark__surface"
      x="1.5"
      y="1.5"
      rx="9"
      width="29"
      height="29"
    />
    <path
      class="brand-mark__shape"
      d="M9 7.5H16.9C21.5 7.5 24.1 10 24.1 14.1C24.1 18.2 21.3 20.8 16.6 20.8H13.5V24.5H9V7.5ZM13.5 17.1H16.2C18.5 17.1 19.6 16.1 19.6 14.2C19.6 12.4 18.5 11.3 16.2 11.3H13.5V17.1Z"
    />
    <path class="brand-mark__play" d="M16.8 12.8L20.9 15.3L16.8 17.8V12.8Z" />
  </svg>
</template>

<style scoped>
.brand-mark {
  width: 1.55rem;
  height: 1.55rem;
  display: block;
}

.brand-mark__surface {
  fill: color-mix(in srgb, var(--bg-surface-strong) 92%, transparent);
  stroke: color-mix(
    in srgb,
    var(--surface-highlight) 60%,
    var(--border-subtle)
  );
}

.brand-mark__shape {
  fill: var(--text-main);
}

.brand-mark__play {
  fill: var(--signal-blue);
}
</style>
