import { createRouter, createWebHistory } from "vue-router";

import { pinia } from "@/stores";
import { useSessionStore } from "@/stores/session";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/login",
    },
    {
      path: "/login",
      name: "login",
      component: () => import("@/views/LoginView.vue"),
    },
    {
      path: "/register",
      name: "register",
      component: () => import("@/views/RegisterView.vue"),
    },
    {
      path: "/dashboard",
      name: "dashboard",
      component: () => import("@/views/DashboardView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/wizard",
      name: "wizard",
      component: () => import("@/views/WizardView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/drafts",
      name: "drafts",
      component: () => import("@/views/DraftsView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/config-sets",
      name: "config-sets",
      component: () => import("@/views/ConfigSetsView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/config-sets/:configSetId",
      name: "config-set-detail",
      component: () => import("@/views/ConfigSetDetailView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/account",
      name: "account",
      component: () => import("@/views/AccountView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("@/views/SettingsView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/docs",
      name: "docs",
      component: () => import("@/views/DocsView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/more",
      name: "more",
      component: () => import("@/views/MoreView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/about",
      name: "about",
      component: () => import("@/views/AboutView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/disclaimer",
      name: "disclaimer",
      component: () => import("@/views/DisclaimerView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/users",
      name: "users",
      component: () => import("@/views/UsersView.vue"),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
    {
      path: "/logs",
      name: "logs",
      component: () => import("@/views/LogsView.vue"),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
  ],
});

router.beforeEach(async (to, from) => {
  const sessionStore = useSessionStore(pinia);
  await sessionStore.ensureLoaded();

  if (to.meta.requiresAuth && !sessionStore.isAuthenticated) {
    return { name: "login" };
  }

  if (to.meta.requiresAdmin && !sessionStore.isAdmin) {
    if (
      (to.name === "logs" || to.name === "users") &&
      to.query.access === "forbidden"
    ) {
      return true;
    }

    return {
      name: (to.name as string) || "logs",
      query: { access: "forbidden" },
    };
  }

  if (
    sessionStore.isAuthenticated &&
    (to.name === "login" || to.name === "register")
  ) {
    if (from.matched.length > 0 && from.name !== to.name) {
      return false;
    }

    return { name: "dashboard" };
  }

  return true;
});

export default router;
