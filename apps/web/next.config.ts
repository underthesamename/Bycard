import type { NextConfig } from "next";

import { resolveApiUpstreamUrl } from "./src/lib/api-upstream";

const nextConfig: NextConfig = {
  agentRules: false,
  devIndicators: false,
  experimental: {
    useTypeScriptCli: false,
  },
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "assets.tcgdex.net",
        pathname: "/**",
      },
      {
        protocol: "https",
        hostname: "d1i787aglh9bmb.cloudfront.net",
        pathname: "/assets/img/me-expansions/**",
      },
    ],
  },
  poweredByHeader: false,
  reactStrictMode: true,
  async rewrites() {
    const apiUpstreamUrl = resolveApiUpstreamUrl();
    return [
      {
        source: "/api/:path*",
        destination: `${apiUpstreamUrl}/api/:path*`,
      },
    ];
  },
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: [
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "X-Frame-Options", value: "DENY" },
          {
            key: "Referrer-Policy",
            value: "strict-origin-when-cross-origin",
          },
          {
            key: "Permissions-Policy",
            value: "camera=(), microphone=(), geolocation=()",
          },
        ],
      },
    ];
  },
};

export default nextConfig;
