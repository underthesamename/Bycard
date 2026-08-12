import { describe, expect, it } from "vitest";

import { resolveApiUpstreamUrl } from "./api-upstream";

describe("resolveApiUpstreamUrl", () => {
  it("uses the local API when no deployment environment is configured", () => {
    expect(resolveApiUpstreamUrl({})).toBe("http://127.0.0.1:8080");
  });

  it("requires an explicit HTTPS upstream in production", () => {
    expect(() => resolveApiUpstreamUrl({ APP_ENV: "production" })).toThrow(
      "API_UPSTREAM_URL is required",
    );
    expect(() =>
      resolveApiUpstreamUrl({
        APP_ENV: "production",
        API_UPSTREAM_URL: "http://api.bycard.example",
      }),
    ).toThrow("must use HTTPS");
  });

  it("rejects credentials, paths, queries, and unsupported protocols", () => {
    for (const API_UPSTREAM_URL of [
      "https://user@api.bycard.example",
      "https://api.bycard.example/v1",
      "https://api.bycard.example?debug=1",
      "file:///tmp/bycard.sock",
    ]) {
      expect(() =>
        resolveApiUpstreamUrl({ APP_ENV: "production", API_UPSTREAM_URL }),
      ).toThrow();
    }
  });

  it("normalizes a valid production upstream", () => {
    expect(
      resolveApiUpstreamUrl({
        APP_ENV: "production",
        API_UPSTREAM_URL: "https://api.bycard.example/",
      }),
    ).toBe("https://api.bycard.example");
  });
});
