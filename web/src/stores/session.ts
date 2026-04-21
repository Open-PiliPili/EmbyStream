import { defineStore } from "pinia";

import {
  ApiError,
  getCurrentUser,
  login,
  logout,
  register,
} from "@/api/client";
import type { SessionUser } from "@/api/types";

export const useSessionStore = defineStore("session", {
  state: () => ({
    user: null as SessionUser | null,
    bootstrapped: false,
  }),
  getters: {
    isAuthenticated: (state) => Boolean(state.user),
    isAdmin: (state) => state.user?.role === "admin",
  },
  actions: {
    async ensureLoaded() {
      if (this.bootstrapped) {
        return;
      }
      try {
        const response = await getCurrentUser();
        this.user = response.user;
      } catch (error) {
        if (!(error instanceof ApiError) || error.status !== 401) {
          throw error;
        }
        this.user = null;
      } finally {
        this.bootstrapped = true;
      }
    },
    async signIn(payload: { login: string; password: string }) {
      const response = await login(payload);
      this.user = response.user;
      this.bootstrapped = true;
      return response.user;
    },
    async signUp(payload: {
      username: string;
      email?: string | null;
      password: string;
    }) {
      return register(payload);
    },
    async signOut() {
      await logout();
      this.user = null;
      this.bootstrapped = true;
    },
  },
});
