import { API_V1_URL } from "@/lib/api-base";

export type AuthUser = {
  id: string;
  displayName: string;
  email: string;
};

type AuthResponse = { user: AuthUser; expiresAt: string };
type ErrorResponse = { error?: { code?: string; message?: string } };

export class AuthRequestError extends Error {
  constructor(
    message: string,
    public readonly code: string,
  ) {
    super(message);
  }
}

export async function registerAccount(input: {
  displayName: string;
  email: string;
  password: string;
}) {
  return authRequest<AuthResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export async function loginAccount(input: { email: string; password: string }) {
  return authRequest<AuthResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export async function logoutAccount() {
  const { csrfToken } = await authRequest<{ csrfToken: string }>("/auth/csrf");
  await authRequest<void>("/auth/logout", {
    method: "POST",
    headers: { "x-csrf-token": csrfToken },
  });
}

export async function fetchCurrentSession(signal?: AbortSignal) {
  return authRequest<AuthResponse>("/auth/me", { signal });
}

export async function fetchCsrfToken() {
  return authRequest<{ csrfToken: string }>("/auth/csrf");
}

async function authRequest<T>(path: string, init: RequestInit = {}) {
  const response = await fetch(`${API_V1_URL}${path}`, {
    ...init,
    credentials: "include",
    headers: {
      Accept: "application/json",
      ...(init.body ? { "Content-Type": "application/json" } : {}),
      ...init.headers,
    },
  });
  if (response.ok) {
    return response.status === 204
      ? (undefined as T)
      : ((await response.json()) as T);
  }
  const body = (await response.json().catch(() => ({}))) as ErrorResponse;
  throw new AuthRequestError(
    body.error?.message ?? "Não foi possível concluir agora.",
    body.error?.code ?? "unexpected_error",
  );
}
