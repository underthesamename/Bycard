const LOCAL_API_UPSTREAM_URL = "http://127.0.0.1:8080";
const SUPPORTED_ENVIRONMENTS = new Set(["local", "test", "production"]);

type ApiEnvironment = Readonly<{
  APP_ENV?: string;
  API_UPSTREAM_URL?: string;
}>;

export function resolveApiUpstreamUrl(
  environment: ApiEnvironment = {
    APP_ENV: process.env.APP_ENV,
    API_UPSTREAM_URL: process.env.API_UPSTREAM_URL,
  },
) {
  const appEnvironment = environment.APP_ENV ?? "local";
  if (!SUPPORTED_ENVIRONMENTS.has(appEnvironment)) {
    throw new Error("APP_ENV must be one of: local, test, production");
  }

  const configuredUrl = environment.API_UPSTREAM_URL?.trim();
  if (!configuredUrl && appEnvironment === "production") {
    throw new Error("API_UPSTREAM_URL is required in production");
  }

  let url: URL;
  try {
    url = new URL(configuredUrl || LOCAL_API_UPSTREAM_URL);
  } catch {
    throw new Error("API_UPSTREAM_URL must be a valid URL");
  }

  if (!["http:", "https:"].includes(url.protocol)) {
    throw new Error("API_UPSTREAM_URL must use HTTP or HTTPS");
  }
  if (
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.hash
  ) {
    throw new Error(
      "API_UPSTREAM_URL must contain only scheme, host, and optional port",
    );
  }
  if (appEnvironment === "production" && url.protocol !== "https:") {
    throw new Error("API_UPSTREAM_URL must use HTTPS in production");
  }

  return url.origin;
}
