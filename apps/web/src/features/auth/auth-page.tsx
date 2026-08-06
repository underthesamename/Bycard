import { BrandHeader } from "@/features/catalog/brand-header";

import { AuthForm } from "./auth-form";

export function AuthPage({ mode }: { mode: "login" | "register" }) {
  const register = mode === "register";
  return (
    <div className="app-shell auth-shell">
      <BrandHeader />
      <main className="auth-main">
        <section className="auth-promise" aria-labelledby="auth-title">
          <p className="auth-kicker">Seu progresso, carta por carta</p>
          <h1 id="auth-title">
            {register
              ? "Comece a organizar sua coleção."
              : "Continue de onde parou."}
          </h1>
          <p>
            Marque o que já encontrou, veja o que ainda falta e volte ao seu
            fichário sempre que precisar.
          </p>
          <ol className="auth-steps">
            <li>
              <strong>01</strong>
              <span>Escolha uma coleção</span>
            </li>
            <li>
              <strong>02</strong>
              <span>Marque suas cartas</span>
            </li>
            <li>
              <strong>03</strong>
              <span>Acompanhe o progresso</span>
            </li>
          </ol>
        </section>
        <section
          className="auth-card"
          aria-label={register ? "Criar conta" : "Entrar"}
        >
          <p className="auth-card-number">BYC / 001</p>
          <h2>{register ? "Crie seu acesso" : "Acesse sua coleção"}</h2>
          <p>Seus dados ficam privados e vinculados apenas à sua conta.</p>
          <AuthForm mode={mode} />
        </section>
      </main>
    </div>
  );
}
