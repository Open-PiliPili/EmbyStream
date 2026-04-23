import type {
  UserEnvelope,
  UserListResponse,
  ArtifactListResponse,
  AuthResponse,
  ConfigSetEnvelope,
  ConfigSetListResponse,
  DraftDocumentEnvelope,
  DraftEnvelope,
  DraftListResponse,
  GenerateDraftResponse,
  LoginBackgroundResponse,
  LogListResponse,
  LogoutResponse,
  RegistrationSettingsResponse,
  RegisterRequest,
  SaveDraftRequest,
  SystemMetricsResponse,
  UpdateRegistrationSettingsRequest,
  WizardTemplateResponse,
} from "./types";
import {
  ADMIN_API,
  AUTH_API,
  BACKGROUNDS_API,
  CONFIG_SETS_API,
  DRAFTS_API,
  LOGS_API,
  buildApiPath,
  buildWebSocketPath,
} from "./constants";

export class ApiError extends Error {
  code: string;
  field?: string;
  status: number;

  constructor(status: number, code: string, message: string, field?: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.field = field;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(buildApiPath(path), {
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new ApiError(
      response.status,
      body?.error?.code ?? "internal_error",
      body?.error?.message ?? "Request failed",
      body?.error?.field,
    );
  }

  return response.json() as Promise<T>;
}

export function getLoginBackground() {
  return request<LoginBackgroundResponse>(BACKGROUNDS_API.login(), {
    method: "GET",
  });
}

export function register(payload: RegisterRequest) {
  return request<AuthResponse>(AUTH_API.register(), {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function getRegistrationSettings() {
  return request<RegistrationSettingsResponse>(
    AUTH_API.registrationSettings(),
    {
      method: "GET",
    },
  );
}

export function login(payload: { login: string; password: string }) {
  return request<AuthResponse>(AUTH_API.login(), {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function logout() {
  return request<LogoutResponse>(AUTH_API.logout(), {
    method: "POST",
  });
}

export function changeOwnPassword(payload: {
  current_password: string;
  new_password: string;
}) {
  return request<LogoutResponse>(AUTH_API.password(), {
    method: "PATCH",
    body: JSON.stringify(payload),
  });
}

export function getCurrentUser() {
  return request<AuthResponse>(AUTH_API.currentUser(), {
    method: "GET",
  });
}

export function listDrafts() {
  return request<DraftListResponse>(DRAFTS_API.list(), { method: "GET" });
}

export function createDraft(payload: {
  name: string;
  stream_mode: "frontend" | "backend" | "dual";
}) {
  return request<DraftEnvelope>(DRAFTS_API.create(), {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function getDraftTemplate(streamMode: "frontend" | "backend" | "dual") {
  return request<WizardTemplateResponse>(DRAFTS_API.template(streamMode), {
    method: "GET",
  });
}

export function getDraft(draftId: string) {
  return request<DraftDocumentEnvelope>(DRAFTS_API.detail(draftId), {
    method: "GET",
  });
}

export function deleteDraft(draftId: string) {
  return request<LogoutResponse>(DRAFTS_API.remove(draftId), {
    method: "DELETE",
  });
}

export function saveDraft(draftId: string, payload: SaveDraftRequest) {
  return request<{
    draft: { id: string; updated_at: string; server_revision: number };
  }>(DRAFTS_API.save(draftId), {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export function generateDraft(draftId: string) {
  return request<GenerateDraftResponse>(DRAFTS_API.generate(draftId), {
    method: "POST",
  });
}

export function updateDraftMetadata(
  draftId: string,
  payload: { name: string },
) {
  return request<DraftEnvelope>(DRAFTS_API.metadata(draftId), {
    method: "PATCH",
    body: JSON.stringify(payload),
  });
}

export function listConfigSets() {
  return request<ConfigSetListResponse>(CONFIG_SETS_API.list(), {
    method: "GET",
  });
}

export function getArtifacts(configSetId: string) {
  return request<ArtifactListResponse>(CONFIG_SETS_API.artifacts(configSetId), {
    method: "GET",
  });
}

export function duplicateConfigSet(configSetId: string) {
  return request<DraftEnvelope>(CONFIG_SETS_API.duplicate(configSetId), {
    method: "POST",
  });
}

export function deleteConfigSet(configSetId: string) {
  return request<LogoutResponse>(CONFIG_SETS_API.remove(configSetId), {
    method: "DELETE",
  });
}

export function updateConfigSetMetadata(
  configSetId: string,
  payload: { name: string },
) {
  return request<ConfigSetEnvelope>(CONFIG_SETS_API.metadata(configSetId), {
    method: "PATCH",
    body: JSON.stringify(payload),
  });
}

export function listLogs(params?: { source?: string; limit?: number }) {
  const query = new URLSearchParams();
  if (params?.source) {
    query.set("source", params.source);
  }
  if (params?.limit) {
    query.set("limit", String(params.limit));
  }
  const suffix = query.toString() ? `?${query.toString()}` : "";
  return request<LogListResponse>(`${LOGS_API.list()}${suffix}`, {
    method: "GET",
  });
}

export function buildLogsStreamUrl(params?: {
  source?: string;
  level?: string;
  limit?: number;
}) {
  const query = new URLSearchParams();
  if (params?.source) {
    query.set("source", params.source);
  }
  if (params?.level) {
    query.set("level", params.level);
  }
  if (params?.limit) {
    query.set("limit", String(params.limit));
  }
  const suffix = query.toString() ? `?${query.toString()}` : "";
  return `${buildWebSocketPath(LOGS_API.stream())}${suffix}`;
}

export function getSystemMetrics() {
  return request<SystemMetricsResponse>(ADMIN_API.system(), {
    method: "GET",
  });
}

export function listUsers() {
  return request<UserListResponse>(ADMIN_API.users(), {
    method: "GET",
  });
}

export function updateRegistrationSettings(
  payload: UpdateRegistrationSettingsRequest,
) {
  return request<RegistrationSettingsResponse>(
    ADMIN_API.registrationSettings(),
    {
      method: "PATCH",
      body: JSON.stringify(payload),
    },
  );
}

export function updateUserRole(userId: string, role: "admin" | "user") {
  return request<UserEnvelope>(ADMIN_API.userRole(userId), {
    method: "PATCH",
    body: JSON.stringify({ role }),
  });
}

export function updateUserDisabled(userId: string, disabled: boolean) {
  return request<UserEnvelope>(ADMIN_API.userDisabled(userId), {
    method: "PATCH",
    body: JSON.stringify({ disabled }),
  });
}

export function updateUserPassword(userId: string, password: string) {
  return request<UserEnvelope>(ADMIN_API.userPassword(userId), {
    method: "PATCH",
    body: JSON.stringify({ password }),
  });
}

export function deleteUser(userId: string) {
  return request<LogoutResponse>(ADMIN_API.userDelete(userId), {
    method: "DELETE",
  });
}
