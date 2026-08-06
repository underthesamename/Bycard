import { cookies } from "next/headers";
import Link from "next/link";
import { redirect } from "next/navigation";

import { LogoutButton } from "@/features/auth/logout-button";
import { BrandHeader } from "@/features/catalog/brand-header";
import { API_V1_URL } from "@/lib/api-base";

type Session = {
  user: { displayName: string; email: string };
};

async function currentSession(): Promise<Session | null> {
  const cookieStore = await cookies();
  const response = await fetch(`${API_V1_URL}/auth/me`, {
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
      <BrandHeader />
      <main className="account-main">
        <p className="auth-kicker">Área privada</p>
        <h1>Olá, {session.user.displayName}.</h1>
        <p className="account-email">
          Sua sessão está ativa em {session.user.email}.
        </p>
        <section className="account-next">
          <div>
            <span>Próximo passo</span>
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
