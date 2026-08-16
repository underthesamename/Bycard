import { cookies } from "next/headers";
import Link from "next/link";
import { redirect } from "next/navigation";

import { LogoutButton } from "@/features/auth/logout-button";
import { ProfileEditor } from "@/features/auth/profile-editor";
import type { AuthUser } from "@/features/auth/auth-api";
import { BrandHeader } from "@/features/catalog/brand-header";
import { SERVER_API_V1_URL } from "@/lib/server-api-base";

type Session = {
  user: AuthUser;
};

async function currentSession(): Promise<Session | null> {
  const cookieStore = await cookies();
  const response = await fetch(`${SERVER_API_V1_URL}/auth/me`, {
    headers: { Cookie: cookieStore.toString(), Accept: "application/json" },
    cache: "no-store",
  }).catch(() => null);
  if (!response?.ok) return null;
  return (await response.json()) as Session;
}

export default async function AccountPage() {
  const session = await currentSession();
  if (!session) redirect("/entrar?motivo=sessao");

  return (
    <div className="app-shell account-shell">
      <BrandHeader initialUser={session.user} />
      <main className="account-main">
        <div className="account-heading">
          <h1>Olá, {session.user.displayName}.</h1>
          <p className="account-email">
            @{session.user.username} · {session.user.email}
          </p>
        </div>
        <ProfileEditor initialUser={session.user} />
        <section className="account-next">
          <div>
            <h2>Escolha a coleção que você quer completar.</h2>
            <p>
              Abra o catálogo e encontre as cartas que já fazem parte do seu
              fichário.
            </p>
          </div>
          <Link className="open-collection" href="/">
            Explorar coleções
          </Link>
        </section>
        <LogoutButton />
      </main>
    </div>
  );
}
