/**
 * Web studio API base path.
 * Can be overridden by Vite env when the UI is deployed behind another prefix.
 */
export const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "/api";

/**
 * Normalize an API path onto the configured API base.
 */
export function buildApiPath(path: string): string {
  return `${API_BASE.replace(/\/$/, "")}/${path.replace(/^\//, "")}`;
}

export function buildWebSocketPath(path: string): string {
  if (typeof window === "undefined") {
    return buildApiPath(path);
  }

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const apiPath = buildApiPath(path);
  return `${protocol}//${window.location.host}${apiPath.startsWith("/") ? apiPath : `/${apiPath}`}`;
}

/**
 * Session and account management endpoints.
 */
export const AUTH_API = {
  register: () => "auth/register",
  login: () => "auth/login",
  logout: () => "auth/logout",
  password: () => "auth/password",
  currentUser: () => "auth/me",
} as const;

/**
 * Draft lifecycle and guided configuration endpoints.
 */
export const DRAFTS_API = {
  list: () => "drafts",
  create: () => "drafts",
  template: (streamMode: "frontend" | "backend" | "dual") =>
    `drafts/templates/${streamMode}`,
  detail: (draftId: string) => `drafts/${draftId}`,
  save: (draftId: string) => `drafts/${draftId}`,
  remove: (draftId: string) => `drafts/${draftId}`,
  generate: (draftId: string) => `drafts/${draftId}/generate`,
  metadata: (draftId: string) => `drafts/${draftId}/metadata`,
} as const;

/**
 * Generated config-set browsing and artifact download endpoints.
 */
export const CONFIG_SETS_API = {
  list: () => "config-sets",
  artifacts: (configSetId: string) => `config-sets/${configSetId}/artifacts`,
  artifactDownload: (configSetId: string, artifactType: string) =>
    buildApiPath(
      `config-sets/${configSetId}/artifacts/${artifactType}/download`,
    ),
  duplicate: (configSetId: string) => `config-sets/${configSetId}/duplicate`,
  remove: (configSetId: string) => `config-sets/${configSetId}`,
  metadata: (configSetId: string) => `config-sets/${configSetId}/metadata`,
} as const;

/**
 * Login background rotation endpoint.
 */
export const BACKGROUNDS_API = {
  login: () => "backgrounds/login",
} as const;

/**
 * Admin log browser endpoint.
 */
export const LOGS_API = {
  list: () => "logs",
  stream: () => "logs/stream",
} as const;

/**
 * Admin-only system and user management endpoints.
 */
export const ADMIN_API = {
  system: () => "admin/system",
  users: () => "admin/users",
  userRole: (userId: string) => `admin/users/${userId}/role`,
  userDisabled: (userId: string) => `admin/users/${userId}/disabled`,
  userPassword: (userId: string) => `admin/users/${userId}/password`,
  userDelete: (userId: string) => `admin/users/${userId}`,
} as const;
