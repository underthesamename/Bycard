export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:8080";

export const API_V1_URL =
  typeof window === "undefined" ? `${API_BASE_URL}/api/v1` : "/api/v1";
