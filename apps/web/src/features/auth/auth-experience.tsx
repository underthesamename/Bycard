"use client";

import { useCallback, useState } from "react";

import { AuthForm } from "./auth-form";
import { HolographicCard } from "./holographic-card";

type AuthExperienceProps = Readonly<{ mode: "login" | "register" }>;

const SUCCESS_ANIMATION_DURATION_MS = 800;

export function AuthExperience({ mode }: AuthExperienceProps) {
  const [isCardCollected, setIsCardCollected] = useState(false);
  const isRegister = mode === "register";

  const animateSuccessfulAccess = useCallback(async () => {
    setIsCardCollected(true);

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return;
    }

    await new Promise<void>((resolve) => {
      window.setTimeout(resolve, SUCCESS_ANIMATION_DURATION_MS);
    });
  }, []);

  return (
    <>
      <section className="auth-promise" aria-labelledby="auth-title">
        <div className="auth-promise-copy">
          <h1 id="auth-title">
            {isRegister
              ? "Comece a organizar sua coleção."
              : "Continue de onde parou."}
          </h1>
          <p>
            Marque o que já encontrou, veja o que ainda falta e volte ao seu
            fichário sempre que precisar.
          </p>
        </div>
        <HolographicCard isCollected={isCardCollected} />
      </section>
      <section
        className="auth-card"
        aria-label={isRegister ? "Criar conta" : "Entrar"}
      >
        <h2>{isRegister ? "Crie seu acesso" : "Acesse sua coleção"}</h2>
        <p>Seus dados ficam privados e vinculados apenas à sua conta.</p>
        <AuthForm mode={mode} onAuthenticated={animateSuccessfulAccess} />
      </section>
    </>
  );
}
