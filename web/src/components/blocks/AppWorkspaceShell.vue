<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  ref,
  useSlots,
  watch,
} from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";

import BrandMark from "@/components/ui/BrandMark.vue";
import ActionDialog from "@/components/ui/ActionDialog.vue";
import { APP_NAME, EXTERNAL_LINKS, STORAGE_KEYS } from "@/constants/app";
import { useTheme } from "@/composables/useTheme";
import { useSessionStore } from "@/stores/session";

const props = defineProps<{
  eyebrow?: string;
  title?: string;
  body?: string;
  variant?: "default" | "onboarding";
}>();

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const slots = useSlots();
const sessionStore = useSessionStore();
const { theme, themePreference, cycleThemePreference } = useTheme();
const accountMenuOpen = ref(false);
const aboutOpen = ref(false);
const disclaimerOpen = ref(false);
const logoutDialogOpen = ref(false);
const mobileNavOpen = ref(false);
const sidebarCollapsed = ref(false);
const accountMenuRef = ref<HTMLElement | null>(null);
const tabbarStretch = ref<"left" | "right" | null>(null);
const tabbarAnimatedIndex = ref(0);
let tabbarStretchTimer: number | undefined;

if (typeof window !== "undefined") {
  sidebarCollapsed.value =
    window.localStorage.getItem(STORAGE_KEYS.sidebarCollapsed) === "1";
}

watch(
  sidebarCollapsed,
  (value) => {
    if (typeof window === "undefined") {
      return;
    }

    window.localStorage.setItem(
      STORAGE_KEYS.sidebarCollapsed,
      value ? "1" : "0",
    );
  },
  { immediate: true },
);

const roleLabel = computed(() =>
  sessionStore.isAdmin ? t("account.roleAdmin") : t("account.roleUser"),
);

const onboardingBellLabel = computed(() =>
  sessionStore.isAdmin ? t("nav.logs") : t("common.notifications"),
);

const navItems = computed(() => {
  const items = [];

  items.push({
    icon: "ph:gauge",
    name: "dashboard",
    label: t("nav.dashboard"),
  });

  items.push({
    icon: "ph:stack",
    name: "config-sets",
    label: t("nav.configSets"),
  });

  if (sessionStore.isAdmin) {
    items.push({
      icon: "ph:receipt",
      name: "logs",
      label: t("nav.logs"),
    });
    items.push({
      icon: "ph:users-three",
      name: "users",
      label: t("nav.users"),
    });
  }

  items.push({
    icon: "ph:gear-six",
    name: "settings",
    label: t("nav.settings"),
  });

  return items;
});

const mobilePrimaryNavItems = computed(() => [
  {
    icon: "ph:house-simple",
    key: "dashboard",
    label: t("nav.home"),
    name: "dashboard",
  },
  {
    icon: "ph:stack",
    key: "config-sets",
    label: t("nav.configSets"),
    name: "config-sets",
  },
  {
    icon: "ph:dots-three-outline",
    key: "more",
    label: t("common.more"),
    name: "more",
  },
]);

const mobileTabRouteName = computed(() => {
  const routeName = String(route.name ?? "");

  if (
    routeName === "dashboard" ||
    routeName === "config-sets" ||
    routeName === "config-set-detail" ||
    routeName === "more"
  ) {
    if (routeName === "config-set-detail") {
      return "config-sets";
    }

    return routeName;
  }

  if (route.query.from === "more") {
    return "more";
  }

  return "dashboard";
});

const mobileBackRoutes = new Set([
  "docs",
  "account",
  "settings",
  "about",
  "disclaimer",
  "logs",
  "users",
]);

const showMobileBackButton = computed(() => {
  const name = String(route.name ?? "");
  if (!mobileBackRoutes.has(name)) {
    return false;
  }

  return route.query.from === "more";
});

const activeMobileTabIndex = computed(() =>
  Math.max(
    mobilePrimaryNavItems.value.findIndex(
      (item) => item.name === mobileTabRouteName.value,
    ),
    0,
  ),
);

const nextThemePreference = computed(() => {
  if (themePreference.value === "system") {
    return "light";
  }

  if (themePreference.value === "light") {
    return "dark";
  }

  return "system";
});

const themePreferenceLabel = computed(() => {
  if (themePreference.value === "system") {
    return t("common.themeSystem");
  }

  return themePreference.value === "dark"
    ? t("common.themeDark")
    : t("common.themeLight");
});

const themeButtonLabel = computed(() => {
  const nextLabel =
    nextThemePreference.value === "system"
      ? t("common.themeSystem")
      : nextThemePreference.value === "dark"
        ? t("common.themeDark")
        : t("common.themeLight");

  return t("common.themeSwitchTo", { mode: nextLabel });
});

const themeButtonIcon = computed(() => {
  if (themePreference.value === "system") {
    return "ph:desktop";
  }

  return theme.value === "dark" ? "ph:sun" : "ph:moon";
});

function animateTabbarTo(nextIndex: number, previousIndex: number) {
  if (nextIndex === previousIndex) {
    tabbarAnimatedIndex.value = nextIndex;
    return;
  }

  tabbarStretch.value = nextIndex > previousIndex ? "right" : "left";
  tabbarAnimatedIndex.value = nextIndex;

  if (tabbarStretchTimer !== undefined) {
    window.clearTimeout(tabbarStretchTimer);
  }

  tabbarStretchTimer = window.setTimeout(() => {
    tabbarStretch.value = null;
    tabbarStretchTimer = undefined;
  }, 260);
}

watch(activeMobileTabIndex, (value, previousValue) => {
  if (typeof window === "undefined") {
    tabbarAnimatedIndex.value = value;
    return;
  }

  window.sessionStorage.setItem(STORAGE_KEYS.mobileTabIndex, String(value));

  if (previousValue === undefined) {
    return;
  }

  animateTabbarTo(value, previousValue);
});

function closeHeaderMenus() {
  accountMenuOpen.value = false;
  mobileNavOpen.value = false;
}

function closeAboutDialog() {
  aboutOpen.value = false;
}

function closeDisclaimerDialog() {
  disclaimerOpen.value = false;
}

function handleDocumentPointerDown(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) {
    return;
  }

  if (accountMenuRef.value?.contains(target)) {
    return;
  }

  closeHeaderMenus();
}

function handleDocumentKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeHeaderMenus();
    closeAboutDialog();
    closeDisclaimerDialog();
  }
}

async function signOut() {
  closeHeaderMenus();
  await sessionStore.signOut();
  await router.push({
    name: "login",
    query: { refresh: String(Date.now()) },
  });
}

function requestSignOut() {
  closeHeaderMenus();
  logoutDialogOpen.value = true;
}

async function confirmSignOut() {
  logoutDialogOpen.value = false;
  await signOut();
}

function goTo(name: string) {
  closeHeaderMenus();
  router.push({ name });
}

const appVersion = __APP_VERSION__;
const githubUrl = __APP_GITHUB_URL__;
const changelogUrl = __APP_CHANGELOG_URL__;
const claudeUrl = EXTERNAL_LINKS.claudeCode;
const codexUrl = EXTERNAL_LINKS.codex;

function isActive(name: string) {
  if (name === "config-sets" && route.name === "config-set-detail") {
    return true;
  }

  return route.name === name;
}

function isMobileTabActive(name: string) {
  return mobileTabRouteName.value === name;
}

function goToOnboardingHelp() {
  goTo("settings");
}

function goToOnboardingActivity() {
  if (sessionStore.isAdmin) {
    goTo("logs");
    return;
  }

  goTo("drafts");
}

function toggleAccountMenu() {
  const nextValue = !accountMenuOpen.value;
  accountMenuOpen.value = nextValue;
}

function toggleMobileNav() {
  mobileNavOpen.value = !mobileNavOpen.value;
  accountMenuOpen.value = false;
}

function toggleThemeAction() {
  closeHeaderMenus();
  cycleThemePreference();
}

function goBackFromMobile() {
  closeHeaderMenus();
  router.push({ name: "more" });
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value;
}

onMounted(() => {
  document.addEventListener("pointerdown", handleDocumentPointerDown);
  document.addEventListener("keydown", handleDocumentKeydown);

  if (typeof window !== "undefined") {
    const stored = Number(
      window.sessionStorage.getItem(STORAGE_KEYS.mobileTabIndex),
    );
    const previousIndex = Number.isFinite(stored)
      ? stored
      : activeMobileTabIndex.value;
    tabbarAnimatedIndex.value = previousIndex;
    window.sessionStorage.setItem(
      STORAGE_KEYS.mobileTabIndex,
      String(activeMobileTabIndex.value),
    );

    window.requestAnimationFrame(() => {
      animateTabbarTo(activeMobileTabIndex.value, previousIndex);
    });
  }
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  document.removeEventListener("keydown", handleDocumentKeydown);
  if (tabbarStretchTimer !== undefined) {
    window.clearTimeout(tabbarStretchTimer);
  }
});
</script>

<template>
  <main
    class="workspace-shell"
    :class="{ 'workspace-shell--collapsed': sidebarCollapsed }"
  >
    <aside
      class="workspace-shell__sidebar"
      :class="[
        `workspace-shell__sidebar--${variant ?? 'default'}`,
        { 'workspace-shell__sidebar--collapsed': sidebarCollapsed },
      ]"
    >
      <div class="workspace-shell__logo-row">
        <BrandMark v-if="!sidebarCollapsed" class="workspace-shell__logo" />
        <span v-if="!sidebarCollapsed" class="workspace-shell__wordmark">{{
          APP_NAME
        }}</span>
        <button
          class="workspace-shell__sidebar-toggle"
          type="button"
          :aria-label="
            sidebarCollapsed
              ? t('common.expandSidebar')
              : t('common.collapseSidebar')
          "
          @click="toggleSidebar"
        >
          <Icon
            :icon="
              sidebarCollapsed
                ? 'lucide:panel-left-open'
                : 'lucide:panel-left-close'
            "
            aria-hidden="true"
            class="workspace-shell__sidebar-toggle-icon"
            width="30"
          />
        </button>
      </div>

      <nav v-if="variant !== 'onboarding'" class="workspace-shell__nav">
        <button
          v-for="item in navItems"
          :key="item.name"
          :class="{ active: isActive(item.name) }"
          type="button"
          @click="goTo(item.name)"
        >
          <Icon :icon="item.icon" width="18" />
          <span v-if="!sidebarCollapsed">{{ item.label }}</span>
        </button>
      </nav>

      <nav v-else class="workspace-shell__nav workspace-shell__nav--onboarding">
        <button class="active" type="button" @click="goTo('drafts')">
          <Icon icon="ph:house-simple" width="18" />
          <span v-if="!sidebarCollapsed">{{ t("nav.home") }}</span>
        </button>
      </nav>

      <div
        v-if="variant === 'onboarding'"
        class="workspace-shell__sidebar-footer"
      >
        <button type="button" @click="goToOnboardingHelp">
          <Icon icon="ph:question" width="16" />
          <span v-if="!sidebarCollapsed">{{ t("common.help") }}</span>
        </button>
        <button type="button" @click="goToOnboardingActivity">
          <Icon icon="ph:bell" width="16" />
          <span v-if="!sidebarCollapsed">{{ onboardingBellLabel }}</span>
        </button>
      </div>
    </aside>

    <section class="workspace-shell__main">
      <header
        class="workspace-shell__topbar"
        :class="{
          'workspace-shell__topbar--overlay': props.variant === 'onboarding',
        }"
      >
        <div v-if="eyebrow || title || body" class="workspace-shell__hero">
          <p v-if="eyebrow" class="eyebrow">{{ eyebrow }}</p>
          <h1 v-if="title">{{ title }}</h1>
          <p v-if="body" class="lede">{{ body }}</p>
          <div
            v-if="slots['hero-actions']"
            class="workspace-shell__hero-actions"
          >
            <slot name="hero-actions" />
          </div>
        </div>
        <div
          v-else
          class="workspace-shell__hero workspace-shell__hero--empty"
        ></div>

        <div class="workspace-shell__actions">
          <button
            v-if="showMobileBackButton"
            class="workspace-shell__utility workspace-shell__mobile-back"
            type="button"
            :aria-label="t('common.previous')"
            @click="goBackFromMobile"
          >
            <Icon icon="ph:arrow-left" width="18" />
          </button>
          <button
            class="workspace-shell__utility workspace-shell__mobile-toggle"
            type="button"
            :aria-label="
              mobileNavOpen ? t('common.closeMenu') : t('common.openMenu')
            "
            @click="toggleMobileNav"
          >
            <Icon :icon="mobileNavOpen ? 'ph:x' : 'ph:list'" width="18" />
          </button>
          <button
            class="workspace-shell__utility"
            type="button"
            :aria-label="themeButtonLabel"
            :title="t('common.themeCurrent', { mode: themePreferenceLabel })"
            @click="toggleThemeAction"
          >
            <Icon :icon="themeButtonIcon" width="16" />
          </button>

          <div ref="accountMenuRef" class="workspace-shell__account">
            <button type="button" @click="toggleAccountMenu">
              <Icon icon="ph:user-circle" width="18" />
              <span>{{ sessionStore.user?.username ?? "user" }}</span>
            </button>
            <Transition name="menu-pop">
              <div v-if="accountMenuOpen" class="workspace-shell__dropdown">
                <p class="workspace-shell__user">
                  {{ sessionStore.user?.username }}
                </p>
                <p class="workspace-shell__meta">{{ roleLabel }}</p>
                <button type="button" @click="goTo('account')">
                  {{ t("nav.account") }}
                </button>
                <button
                  type="button"
                  @click="
                    closeHeaderMenus();
                    disclaimerOpen = true;
                  "
                >
                  {{ t("common.disclaimer") }}
                </button>
                <button type="button" @click="goTo('docs')">
                  {{ t("nav.docs") }}
                </button>
                <button
                  type="button"
                  @click="
                    closeHeaderMenus();
                    aboutOpen = true;
                  "
                >
                  {{ t("account.aboutOpen") }}
                </button>
                <button
                  class="workspace-shell__dropdown-danger"
                  type="button"
                  @click="requestSignOut"
                >
                  {{ t("common.logout") }}
                </button>
              </div>
            </Transition>
          </div>
        </div>
      </header>

      <section class="workspace-shell__content">
        <slot />
      </section>
    </section>

    <nav class="workspace-shell__tabbar">
      <div class="workspace-shell__tabbar-shell">
        <div
          class="workspace-shell__tabbar-active-pill"
          :class="{
            'workspace-shell__tabbar-active-pill--stretch-left':
              tabbarStretch === 'left',
            'workspace-shell__tabbar-active-pill--stretch-right':
              tabbarStretch === 'right',
          }"
          :style="{
            left: `calc(var(--tabbar-pad) + ${tabbarAnimatedIndex} * (var(--tabbar-pill-width) + var(--tabbar-gap)))`,
          }"
        ></div>
        <button
          v-for="item in mobilePrimaryNavItems"
          :key="item.key"
          class="workspace-shell__tabbar-item"
          :class="{ active: isMobileTabActive(item.name) }"
          type="button"
          @click="goTo(item.name)"
        >
          <div class="workspace-shell__tabbar-glass">
            <div class="workspace-shell__tabbar-item-inner">
              <Icon :icon="item.icon" width="22" />
              <span>{{ item.label }}</span>
            </div>
          </div>
        </button>
      </div>
    </nav>

    <Transition name="mobile-nav-pop">
      <div
        v-if="mobileNavOpen"
        class="workspace-shell__mobile-overlay"
        @click.self="closeHeaderMenus"
      >
        <div class="workspace-shell__mobile-panel">
          <div class="workspace-shell__mobile-head">
            <span class="workspace-shell__wordmark">{{ APP_NAME }}</span>
            <button
              class="workspace-shell__utility"
              type="button"
              :aria-label="t('common.closeMenu')"
              @click="closeHeaderMenus"
            >
              <Icon icon="ph:x" width="18" />
            </button>
          </div>

          <nav class="workspace-shell__mobile-nav">
            <button
              v-for="item in navItems"
              :key="item.name"
              :class="{ active: isActive(item.name) }"
              type="button"
              @click="goTo(item.name)"
            >
              <Icon :icon="item.icon" width="18" />
              <span>{{ item.label }}</span>
            </button>
          </nav>
        </div>
      </div>
    </Transition>

    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="aboutOpen"
          class="workspace-shell__about-overlay"
          role="dialog"
          aria-modal="true"
          @click.self="closeAboutDialog"
        >
          <div class="workspace-shell__about-dialog">
            <div class="workspace-shell__about-head">
              <div>
                <p class="section-label">{{ t("account.aboutLabel") }}</p>
                <h3>{{ t("account.aboutTitle") }}</h3>
              </div>
              <button
                class="workspace-shell__about-close"
                type="button"
                :aria-label="t('common.close')"
                @click="closeAboutDialog"
              >
                <svg
                  aria-hidden="true"
                  class="workspace-shell__about-close-icon"
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

            <div class="workspace-shell__about-body">
              <p>{{ t("account.aboutBody") }}</p>
              <span class="workspace-shell__about-version">
                {{ t("account.aboutVersion", { version: appVersion }) }}
              </span>
              <div class="workspace-shell__about-links">
                <a :href="githubUrl" rel="noopener noreferrer" target="_blank">
                  <Icon icon="mdi:github" width="18" />
                  <span>{{ t("account.aboutGithub") }}</span>
                </a>
                <a
                  :href="changelogUrl"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  <Icon icon="ph:clock-counter-clockwise" width="18" />
                  <span>{{ t("account.aboutChangelog") }}</span>
                </a>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <Teleport to="body">
      <Transition name="modal-pop">
        <div
          v-if="disclaimerOpen"
          class="workspace-shell__about-overlay"
          role="dialog"
          aria-modal="true"
          @click.self="closeDisclaimerDialog"
        >
          <div class="workspace-shell__about-dialog">
            <div class="workspace-shell__about-head">
              <div>
                <p class="section-label">{{ t("common.disclaimer") }}</p>
                <h3>{{ t("disclaimer.title") }}</h3>
              </div>
              <button
                class="workspace-shell__about-close"
                type="button"
                :aria-label="t('common.close')"
                @click="closeDisclaimerDialog"
              >
                <svg
                  aria-hidden="true"
                  class="workspace-shell__about-close-icon"
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

            <div class="workspace-shell__about-body">
              <p>{{ t("disclaimer.body") }}</p>
              <div class="workspace-shell__about-links">
                <a :href="claudeUrl" rel="noopener noreferrer" target="_blank">
                  <svg
                    aria-hidden="true"
                    class="workspace-shell__brand-icon"
                    viewBox="0 0 24 24"
                  >
                    <path
                      clip-rule="evenodd"
                      d="M20.998 10.949H24v3.102h-3v3.028h-1.487V20H18v-2.921h-1.487V20H15v-2.921H9V20H7.488v-2.921H6V20H4.487v-2.921H3V14.05H0V10.95h3V5h17.998v5.949zM6 10.949h1.488V8.102H6v2.847zm10.51 0H18V8.102h-1.49v2.847z"
                      fill="currentColor"
                      fill-rule="evenodd"
                    />
                  </svg>
                  <span>{{ t("disclaimer.claude") }}</span>
                </a>
                <a :href="codexUrl" rel="noopener noreferrer" target="_blank">
                  <svg
                    aria-hidden="true"
                    class="workspace-shell__brand-icon"
                    viewBox="0 0 24 24"
                  >
                    <path
                      clip-rule="evenodd"
                      d="M8.086.457a6.105 6.105 0 013.046-.415c1.333.153 2.521.72 3.564 1.7a.117.117 0 00.107.029c1.408-.346 2.762-.224 4.061.366l.063.03.154.076c1.357.703 2.33 1.77 2.918 3.198.278.679.418 1.388.421 2.126a5.655 5.655 0 01-.18 1.631.167.167 0 00.04.155 5.982 5.982 0 011.578 2.891c.385 1.901-.01 3.615-1.183 5.14l-.182.22a6.063 6.063 0 01-2.934 1.851.162.162 0 00-.108.102c-.255.736-.511 1.364-.987 1.992-1.199 1.582-2.962 2.462-4.948 2.451-1.583-.008-2.986-.587-4.21-1.736a.145.145 0 00-.14-.032c-.518.167-1.04.191-1.604.185a5.924 5.924 0 01-2.595-.622 6.058 6.058 0 01-2.146-1.781c-.203-.269-.404-.522-.551-.821a7.74 7.74 0 01-.495-1.283 6.11 6.11 0 01-.017-3.064.166.166 0 00.008-.074.115.115 0 00-.037-.064 5.958 5.958 0 01-1.38-2.202 5.196 5.196 0 01-.333-1.589 6.915 6.915 0 01.188-2.132c.45-1.484 1.309-2.648 2.577-3.493.282-.188.55-.334.802-.438.286-.12.573-.22.861-.304a.129.129 0 00.087-.087A6.016 6.016 0 015.635 2.31C6.315 1.464 7.132.846 8.086.457zm-.804 7.85a.848.848 0 00-1.473.842l1.694 2.965-1.688 2.848a.849.849 0 001.46.864l1.94-3.272a.849.849 0 00.007-.854l-1.94-3.393zm5.446 6.24a.849.849 0 000 1.695h4.848a.849.849 0 000-1.696h-4.848z"
                      fill="currentColor"
                      fill-rule="evenodd"
                    />
                  </svg>
                  <span>{{ t("disclaimer.codex") }}</span>
                </a>
                <a :href="githubUrl" rel="noopener noreferrer" target="_blank">
                  <Icon icon="mdi:github" width="18" />
                  <span>{{ t("disclaimer.github") }}</span>
                </a>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <ActionDialog
      :open="logoutDialogOpen"
      :title="t('common.logout')"
      :description="t('common.confirmLogout')"
      :confirm-label="t('common.logout')"
      :cancel-label="t('common.cancel')"
      :show-close="false"
      confirm-tone="danger"
      @close="logoutDialogOpen = false"
      @confirm="confirmSignOut"
    />
  </main>
</template>

<style scoped>
.workspace-shell {
  display: grid;
  grid-template-columns: 17rem minmax(0, 1fr);
  min-height: 100vh;
  overflow-x: hidden;
  background: var(--bg-app);
  transition: grid-template-columns 220ms var(--curve-swift);
}

.workspace-shell--collapsed {
  grid-template-columns: 5.75rem minmax(0, 1fr);
}

.workspace-shell__sidebar {
  position: sticky;
  top: 0;
  min-height: 100vh;
  box-sizing: border-box;
  display: grid;
  align-content: start;
  gap: 1.15rem;
  min-width: 0;
  overflow: hidden;
  padding: 1rem 0.8rem 1.1rem;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-subtle);
}

.workspace-shell__sidebar--onboarding {
  padding-inline: 0.6rem;
}

.workspace-shell__sidebar--collapsed {
  padding-inline: 0.55rem;
}

.workspace-shell__logo-row {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 0.7rem;
  min-height: 2.4rem;
  min-width: 0;
  width: 100%;
  padding: 0 0.4rem;
}

.workspace-shell__sidebar--collapsed .workspace-shell__logo-row {
  justify-content: center;
  padding-inline: 0;
}

.workspace-shell__logo {
  flex-shrink: 0;
}

.workspace-shell__wordmark {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-main);
  font-size: 0.98rem;
  font-weight: 700;
}

.workspace-shell__sidebar-toggle {
  flex: 0 0 3.25rem;
  width: 3rem;
  height: 3rem;
  align-items: center;
  justify-content: center;
  margin-left: auto;
  padding: 0;
}

.workspace-shell__sidebar--collapsed .workspace-shell__sidebar-toggle {
  margin-left: 0;
}

.workspace-shell__nav,
.workspace-shell__sidebar-footer {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
}

.workspace-shell__nav--onboarding {
  margin-top: 0.35rem;
}

.workspace-shell__nav button,
.workspace-shell__sidebar-footer button,
.workspace-shell__utility,
.workspace-shell__account > button,
.workspace-shell__dropdown button,
.workspace-shell__sidebar-toggle {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 0.6rem;
  min-width: 0;
  max-width: 100%;
  width: 100%;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  padding: 0.72rem 0.85rem;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  text-align: left;
  font-size: 0.92rem;
  font-weight: 600;
  box-shadow: none;
}

.workspace-shell__sidebar-toggle,
.workspace-shell__utility,
.workspace-shell__account > button {
  width: auto;
  background: var(--bg-surface);
  border-color: var(--border-subtle);
  box-shadow: var(--shadow-soft);
}

.workspace-shell__sidebar-toggle {
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  border-color: var(--border-subtle);
  box-shadow: var(--shadow-soft);
}

.workspace-shell__sidebar-toggle-icon {
  width: 30px;
  height: 30px;
  display: block;
}

.workspace-shell__nav button.active {
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 88%,
    var(--bg-accent)
  );
  border-color: var(--border-strong);
  color: var(--text-main);
  box-shadow: none;
}

@media (pointer: fine) {
  .workspace-shell__nav button:hover,
  .workspace-shell__sidebar-footer button:hover,
  .workspace-shell__utility:hover,
  .workspace-shell__account > button:hover,
  .workspace-shell__dropdown button:hover,
  .workspace-shell__sidebar-toggle:hover {
    background: var(--bg-soft);
    border-color: var(--border-strong);
    color: var(--text-main);
  }
}

.workspace-shell__sidebar--collapsed .workspace-shell__nav button,
.workspace-shell__sidebar--collapsed .workspace-shell__sidebar-footer button {
  justify-content: center;
  padding-inline: 0.55rem;
}

.workspace-shell__nav button span,
.workspace-shell__sidebar-footer button span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-shell__placeholder {
  display: grid;
  gap: 0.55rem;
  padding: 0.4rem 0.3rem;
}

.workspace-shell__placeholder span {
  height: 0.7rem;
  border-radius: 999px;
  background: var(--bg-active);
}

.workspace-shell__placeholder span:nth-child(1) {
  width: 72%;
}

.workspace-shell__placeholder span:nth-child(2) {
  width: 52%;
}

.workspace-shell__placeholder span:nth-child(3) {
  width: 78%;
}

.workspace-shell__placeholder span:nth-child(4) {
  width: 46%;
}

.workspace-shell__placeholder--long span:nth-child(1) {
  width: 58%;
}

.workspace-shell__placeholder--long span:nth-child(2) {
  width: 82%;
}

.workspace-shell__placeholder--long span:nth-child(3) {
  width: 90%;
}

.workspace-shell__placeholder--long span:nth-child(4) {
  width: 76%;
}

.workspace-shell__section {
  margin: 0;
  padding: 0 0.35rem;
  color: var(--text-faint);
  font-size: 0.75rem;
  font-weight: 500;
}

.workspace-shell__main {
  min-width: 0;
  position: relative;
  overflow-x: hidden;
  overflow-y: visible;
}

.workspace-shell__sidebar--onboarding
  + .workspace-shell__main
  .workspace-shell__content {
  width: auto;
  max-width: none;
  margin-inline: 1.5rem;
  padding-top: 0.8rem;
}

.workspace-shell__topbar {
  display: flex;
  justify-content: space-between;
  gap: 1.5rem;
  align-items: start;
  position: sticky;
  top: 0;
  z-index: 30;
  width: auto;
  max-width: none;
  margin-inline: 1.5rem;
  padding: 1.35rem 0 0.9rem;
  background: var(--bg-app);
}

.workspace-shell__topbar--overlay {
  position: absolute;
  inset: 1.5rem 1.5rem auto auto;
  z-index: 40;
  width: auto;
  margin: 0;
  padding: 0;
  pointer-events: none;
  background: transparent;
}

.workspace-shell__hero {
  max-width: 42rem;
  padding-top: 0.15rem;
  min-width: 0;
}

.workspace-shell__hero h1 {
  margin: 0.4rem 0 0;
  font-size: clamp(2.15rem, 3vw, 3.1rem);
  font-weight: 500;
  line-height: 1.12;
}

.workspace-shell__hero .lede {
  margin-top: 0.95rem;
  max-width: 52rem;
}

.workspace-shell__hero-actions {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
  margin-top: 0.9rem;
}

.workspace-shell__hero--empty {
  min-height: 1px;
}

.workspace-shell__hero-actions :slotted(.workspace-shell__hero-action) {
  min-height: 2.7rem;
  padding-inline: 1.05rem;
  font-weight: 700;
}

.workspace-shell__hero-actions
  :slotted(.workspace-shell__hero-action--secondary) {
  border-color: var(--border-accent);
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 88%,
    var(--bg-accent)
  );
  color: var(--signal-blue);
  box-shadow: 0 0 0 1px
    color-mix(in srgb, var(--brand-secondary) 12%, transparent);
}

@media (pointer: fine) {
  .workspace-shell__hero-actions
    :slotted(.workspace-shell__hero-action--secondary:hover) {
    background: color-mix(
      in srgb,
      var(--bg-surface-strong) 80%,
      var(--bg-accent)
    );
  }
}

.workspace-shell__actions {
  display: flex;
  gap: 0.6rem;
  align-items: center;
  pointer-events: auto;
  flex-shrink: 0;
}

.workspace-shell__account {
  position: relative;
  pointer-events: auto;
  flex-shrink: 0;
}

.workspace-shell__dropdown {
  position: absolute;
  right: 0;
  top: calc(100% + 0.5rem);
  width: 12.5rem;
  display: grid;
  gap: 0.2rem;
  padding: 0.6rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface-strong);
  box-shadow: var(--shadow-medium);
  z-index: 20;
}

.workspace-shell__dropdown button {
  border-color: transparent;
  box-shadow: none;
}

.workspace-shell__dropdown button.workspace-shell__dropdown-danger {
  color: var(--signal-red);
}

@media (pointer: fine) {
  .workspace-shell__dropdown button.workspace-shell__dropdown-danger:hover {
    color: var(--signal-red);
    background: color-mix(in srgb, var(--signal-red) 10%, transparent);
    border-color: transparent;
  }
}

.menu-pop-enter-active,
.menu-pop-leave-active {
  transition:
    opacity 240ms var(--curve-swift),
    transform 300ms var(--curve-spring);
}

.menu-pop-enter-from,
.menu-pop-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.96);
}

.workspace-shell__user,
.workspace-shell__meta {
  margin: 0;
  padding: 0 0.55rem;
}

.workspace-shell__user {
  color: var(--text-main);
  font-size: 0.92rem;
  font-weight: 600;
}

.workspace-shell__meta {
  color: var(--text-faint);
  font-size: 0.76rem;
}

.workspace-shell__content {
  width: auto;
  max-width: none;
  margin-inline: 1.5rem;
  padding: 0.8rem 0 3rem;
  min-width: 0;
}

.workspace-shell__mobile-toggle {
  display: none;
}

.workspace-shell__mobile-back {
  display: none;
}

.workspace-shell__mobile-overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  background: rgba(20, 20, 19, 0.28);
}

.workspace-shell__mobile-panel {
  width: min(22rem, calc(100vw - 1.5rem));
  height: 100%;
  padding: 1rem;
  background: var(--bg-app);
  border-right: 1px solid var(--border-subtle);
  box-shadow: var(--shadow-medium);
}

.mobile-nav-pop-enter-active,
.mobile-nav-pop-leave-active {
  transition: opacity 240ms var(--curve-swift);
}

.mobile-nav-pop-enter-from,
.mobile-nav-pop-leave-to {
  opacity: 0;
}

.mobile-nav-pop-enter-active .workspace-shell__mobile-panel,
.mobile-nav-pop-leave-active .workspace-shell__mobile-panel {
  transition:
    transform 340ms var(--curve-spring),
    opacity 240ms var(--curve-swift);
}

.mobile-nav-pop-enter-from .workspace-shell__mobile-panel,
.mobile-nav-pop-leave-to .workspace-shell__mobile-panel {
  opacity: 0;
  transform: translateX(-18px) scale(0.985);
}

.workspace-shell__mobile-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
}

.workspace-shell__mobile-nav {
  display: grid;
  gap: 0.35rem;
}

.workspace-shell__mobile-nav button {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  width: 100%;
  min-height: 2.85rem;
  padding: 0.72rem 0.85rem;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  text-align: left;
}

.workspace-shell__mobile-nav button.active {
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 88%,
    var(--bg-accent)
  );
  border-color: var(--border-accent);
  color: var(--text-main);
  box-shadow: 0 0 0 1px
    color-mix(in srgb, var(--brand-secondary) 14%, transparent);
}

.workspace-shell__about-overlay {
  position: fixed;
  inset: 0;
  z-index: 75;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(20, 20, 19, 0.3);
}

.workspace-shell__about-dialog {
  width: min(32rem, 100%);
  padding: 1.25rem;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface-strong);
  box-shadow: var(--shadow-medium);
}

.workspace-shell__about-head {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: start;
}

.workspace-shell__about-head > button {
  border-color: var(--border-strong);
  box-shadow: none;
}

.workspace-shell__about-close-icon {
  display: block;
  flex-shrink: 0;
}

.workspace-shell__about-close {
  min-width: 2.7rem;
  min-height: 2.7rem;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.workspace-shell__about-head h3,
.workspace-shell__about-body p {
  margin: 0;
}

.workspace-shell__about-head h3 {
  margin-top: 0.45rem;
  font-size: 1.45rem;
  line-height: 1.16;
  font-weight: 500;
}

.workspace-shell__about-body {
  display: grid;
  gap: 1rem;
  margin-top: 1rem;
}

.workspace-shell__about-body p {
  color: var(--text-muted);
  line-height: 1.7;
}

.workspace-shell__about-version {
  display: inline-flex;
  width: fit-content;
  min-height: 2rem;
  align-items: center;
  padding: 0.2rem 0.72rem;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--signal-blue) 12%, transparent);
  color: var(--signal-blue);
  font-size: 0.82rem;
  font-weight: 700;
}

.workspace-shell__about-links {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.workspace-shell__about-links a {
  display: inline-flex;
  align-items: center;
  gap: 0.55rem;
  min-height: 2.45rem;
  padding: 0.55rem 0.9rem;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: var(--button-secondary-bg);
  color: var(--text-main);
  box-shadow: var(--shadow-soft);
  transition:
    background-color 180ms var(--curve-swift),
    border-color 180ms var(--curve-swift),
    box-shadow 180ms var(--curve-swift),
    transform 220ms var(--curve-buoy);
}

.workspace-shell__brand-icon {
  width: 18px;
  height: 18px;
  display: block;
}

@media (pointer: fine) {
  .workspace-shell__about-links a:hover {
    background: color-mix(
      in srgb,
      var(--bg-surface-strong) 88%,
      var(--bg-accent)
    );
    border-color: var(--border-strong);
    box-shadow: var(--shadow-soft);
    transform: translateY(-2px);
  }
}

.workspace-shell__about-links a:focus-visible {
  background: color-mix(
    in srgb,
    var(--bg-surface-strong) 88%,
    var(--bg-accent)
  );
  border-color: var(--border-strong);
  box-shadow: var(--shadow-soft);
  transform: translateY(-1px);
}

.modal-pop-enter-active,
.modal-pop-leave-active {
  transition: opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from,
.modal-pop-leave-to {
  opacity: 0;
}

.modal-pop-enter-active .workspace-shell__about-dialog,
.modal-pop-leave-active .workspace-shell__about-dialog {
  transition:
    transform 320ms var(--curve-spring),
    opacity 220ms var(--curve-swift);
}

.modal-pop-enter-from .workspace-shell__about-dialog,
.modal-pop-leave-to .workspace-shell__about-dialog {
  opacity: 0;
  transform: scale(0.96);
}

.workspace-shell__tabbar {
  display: none;
}

.workspace-shell__tabbar-shell {
  --tabbar-gap: 0.45rem;
  --tabbar-pad: 0.45rem;
  --tabbar-pill-width: calc(
    (100% - (var(--tabbar-pad) * 2) - (var(--tabbar-gap) * 2)) / 3
  );
  position: relative;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--tabbar-gap);
  width: min(100%, 20rem);
  padding: var(--tabbar-pad);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-surface-strong) 42%, transparent);
  border: 1px solid var(--border-strong);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.14),
    0 18px 40px rgba(0, 0, 0, 0.2);
  backdrop-filter: blur(30px) saturate(165%);
  -webkit-backdrop-filter: blur(30px) saturate(165%);
}

.workspace-shell__tabbar-active-pill {
  position: absolute;
  top: var(--tabbar-pad);
  width: var(--tabbar-pill-width);
  height: calc(100% - (var(--tabbar-pad) * 2));
  border-radius: 999px;
  border: 1px solid var(--border-strong);
  background: color-mix(in srgb, var(--bg-surface-strong) 76%, transparent);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.2),
    0 10px 22px rgba(0, 0, 0, 0.12);
  transition:
    left 420ms cubic-bezier(0.22, 1, 0.2, 1.02),
    width 420ms cubic-bezier(0.22, 1, 0.2, 1.02),
    background-color 220ms var(--curve-swift),
    border-color 220ms var(--curve-swift),
    box-shadow 220ms var(--curve-swift),
    transform 460ms cubic-bezier(0.2, 1, 0.18, 1.04);
  pointer-events: none;
  will-change: left, transform;
}

.workspace-shell__tabbar-active-pill--stretch-left {
  width: calc(var(--tabbar-pill-width) * 1.12);
  transform: scaleY(0.96);
  transform-origin: right center;
}

.workspace-shell__tabbar-active-pill--stretch-right {
  width: calc(var(--tabbar-pill-width) * 1.12);
  transform: scaleY(0.96);
  transform-origin: left center;
}

.workspace-shell__tabbar-item {
  appearance: none;
  position: relative;
  z-index: 1;
  min-width: 0;
  min-height: 0;
  background: transparent;
  border: 0;
  border-radius: 0;
  padding: 0;
  box-shadow: none;
  -webkit-tap-highlight-color: transparent;
}

.workspace-shell__tabbar-item:focus-visible {
  outline: none;
  box-shadow: none;
}

.workspace-shell__tabbar-glass {
  width: 100%;
  border-radius: 1.15rem;
  border: 1px solid transparent;
  background: transparent;
  box-shadow: none;
  -webkit-tap-highlight-color: transparent;
}

.workspace-shell__tabbar-item:focus-visible .workspace-shell__tabbar-glass {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus-ring) 20%, transparent);
}

.workspace-shell__tabbar-item.active .workspace-shell__tabbar-glass {
  background: transparent;
  border-color: transparent;
  box-shadow: none;
}

.workspace-shell__tabbar-item-inner {
  align-content: center;
  display: grid;
  justify-items: center;
  gap: 0.18rem;
  min-height: 3.2rem;
  padding: 0.34rem 0.18rem 0.3rem;
  color: var(--text-muted);
  font-size: 0.7rem;
  font-weight: 600;
  line-height: 1.1;
  transition:
    color 220ms var(--curve-swift),
    transform 320ms cubic-bezier(0.2, 1, 0.18, 1.04);
}

.workspace-shell__tabbar-item.active .workspace-shell__tabbar-item-inner {
  color: var(--text-main);
  transform: translateY(-0.5px) scale(1.015);
}

@media (max-width: 980px) {
  .workspace-shell {
    grid-template-columns: 1fr;
  }

  .workspace-shell__sidebar {
    display: none;
  }

  .workspace-shell__mobile-toggle {
    display: none;
  }

  .workspace-shell__mobile-back {
    display: inline-flex;
  }

  .workspace-shell__topbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    width: auto;
    max-width: none;
    margin-inline: 1rem;
    padding-top: calc(1rem + env(safe-area-inset-top, 0px));
    gap: 0.75rem;
  }

  .workspace-shell__topbar--overlay {
    inset: 1rem 1rem auto auto;
  }

  .workspace-shell__hero {
    max-width: none;
    min-width: 0;
  }

  .workspace-shell__hero h1 {
    font-size: clamp(1.9rem, 8vw, 2.5rem);
  }

  .workspace-shell__hero .lede {
    max-width: none;
    margin-top: 0.7rem;
    font-size: 0.96rem;
    line-height: 1.62;
  }

  .workspace-shell__actions {
    justify-self: end;
    gap: 0.45rem;
  }

  .workspace-shell__account {
    min-width: 0;
  }

  .workspace-shell__account > button {
    min-width: 2.8rem;
    min-height: 2.8rem;
    padding-inline: 0.8rem;
  }

  .workspace-shell__account > button span {
    display: none;
  }

  .workspace-shell__content {
    width: auto;
    max-width: none;
    margin-inline: 1rem;
    padding-top: 1rem;
    padding-bottom: calc(6.5rem + env(safe-area-inset-bottom, 0px));
  }

  .workspace-shell__sidebar--onboarding
    + .workspace-shell__main
    .workspace-shell__content {
    width: auto;
    max-width: none;
    margin-inline: 1rem;
  }

  .workspace-shell__tabbar {
    position: fixed;
    left: 1rem;
    right: 1rem;
    bottom: calc(0.8rem + env(safe-area-inset-bottom, 0px));
    z-index: 71;
    display: flex;
    justify-content: center;
  }

  .workspace-shell__tabbar-shell {
    width: min(100%, 20rem);
  }
}
</style>
