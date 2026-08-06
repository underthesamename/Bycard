import { fetchCsrfToken } from "@/features/auth/auth-api";
import type { CatalogCard } from "@/features/catalog/catalog-api";
import { API_V1_URL } from "@/lib/api-base";

export type CollectionStats = {
  totalUnique: number;
  ownedUnique: number;
  missingUnique: number;
  totalCopies: number;
  duplicateCopies: number;
  completionPercentage: number;
};

export type PersonalCollection = CollectionStats & {
  id: string;
  setId: string;
  slug: string;
  name: string;
  coverImageUrl: string | null;
};

export type PersonalCard = CatalogCard & { quantity: number };

export type PersonalCollectionDetail = {
  collection: PersonalCollection;
  cards: PersonalCard[];
};

export class CollectionRequestError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly status: number,
  ) {
    super(message);
  }
}

export async function fetchPersonalCollections(signal?: AbortSignal) {
  return request<{ data: PersonalCollection[] }>("/me/collections", {
    signal,
  });
}

export async function fetchPersonalCollection(
  setId: string,
  signal?: AbortSignal,
) {
  return request<{ data: PersonalCollectionDetail }>(
    `/me/collections/${encodeURIComponent(setId)}`,
    { signal },
  );
}

export async function addPersonalCollection(setId: string) {
  return mutation<{ data: PersonalCollection }>("/me/collections", {
    method: "POST",
    body: JSON.stringify({ setId }),
  });
}

export async function removePersonalCollection(setId: string) {
  return mutation<void>(`/me/collections/${encodeURIComponent(setId)}`, {
    method: "DELETE",
  });
}

export async function updateCardQuantity(
  setId: string,
  cardId: string,
  quantity: number,
) {
  return mutation<{ data: PersonalCollectionDetail }>(
    `/me/collections/${encodeURIComponent(setId)}/cards/${encodeURIComponent(cardId)}`,
    { method: "PUT", body: JSON.stringify({ quantity }) },
  );
}

async function mutation<T>(path: string, init: RequestInit) {
  const { csrfToken } = await fetchCsrfToken();
  return request<T>(path, {
    ...init,
    headers: { ...init.headers, "x-csrf-token": csrfToken },
  });
}

async function request<T>(path: string, init: RequestInit = {}) {
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
  const body = (await response.json().catch(() => ({}))) as {
    error?: { code?: string; message?: string };
  };
  throw new CollectionRequestError(
    body.error?.message ?? "Não foi possível atualizar seu fichário.",
    body.error?.code ?? "unexpected_error",
    response.status,
  );
}
