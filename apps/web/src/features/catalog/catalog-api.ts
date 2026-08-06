import { API_V1_URL } from "@/lib/api-base";

export type Collection = {
  id: string;
  slug: string;
  name: string;
  seriesName: string | null;
  releaseDate: string;
  totalCards: number;
  coverImageUrl: string | null;
  language: string;
};

export type CatalogCard = {
  id: string;
  setId: string;
  localNumber: string;
  printedNumber: string;
  name: string;
  rarity: string | null;
  artist: string | null;
  imageSmallUrl: string | null;
  imageLargeUrl: string | null;
  sortOrder: number;
};

type Pagination = {
  page: number;
  pageSize: number;
  totalItems: number;
  totalPages: number;
};

type ListResponse<T> = {
  data: T[];
  pagination: Pagination;
};

type ResourceResponse<T> = {
  data: T;
};

type ErrorResponse = {
  error?: {
    message?: string;
    requestId?: string;
  };
};

export async function fetchCollections(signal?: AbortSignal) {
  return request<ListResponse<Collection>>("/sets?pageSize=100", signal);
}

export async function fetchCollection(setId: string, signal?: AbortSignal) {
  return request<ResourceResponse<Collection>>(
    `/sets/${encodeURIComponent(setId)}`,
    signal,
  );
}

export async function fetchCards(
  setId: string,
  search: string,
  signal?: AbortSignal,
) {
  const parameters = new URLSearchParams({
    pageSize: "100",
    sort: "number_asc",
  });
  if (search) parameters.set("search", search);
  return request<ListResponse<CatalogCard>>(
    `/sets/${encodeURIComponent(setId)}/cards?${parameters.toString()}`,
    signal,
  );
}

async function request<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(`${API_V1_URL}${path}`, {
    headers: { Accept: "application/json" },
    signal,
  });
  if (response.ok) return (await response.json()) as T;

  const body = (await response.json().catch(() => ({}))) as ErrorResponse;
  const message =
    body.error?.message ?? "Não foi possível carregar o catálogo.";
  throw new Error(message);
}
