import { BrandHeader } from "@/features/catalog/brand-header";

import { AuthExperience } from "./auth-experience";

export function AuthPage({ mode }: { mode: "login" | "register" }) {
  return (
    <div className="app-shell auth-shell">
      <BrandHeader />
      <main className="auth-main">
        <AuthExperience mode={mode} />
      </main>
    </div>
  );
}
