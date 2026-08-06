"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

import { logoutAccount } from "./auth-api";

export function LogoutButton() {
  const router = useRouter();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  async function logout() {
    setPending(true);
    setError("");
    try {
      await logoutAccount();
      router.replace("/entrar");
      router.refresh();
    } catch {
      setError("Não foi possível sair agora. Tente novamente.");
      setPending(false);
    }
  }

  return (
    <div className="logout-action">
      <button type="button" onClick={logout} disabled={pending}>
        {pending ? "Saindo…" : "Sair da conta"}
      </button>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
