"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { FormEvent, useState } from "react";

import { AuthRequestError, loginAccount, registerAccount } from "./auth-api";

type AuthFormProps = { mode: "login" | "register" };

export function AuthForm({ mode }: AuthFormProps) {
  const router = useRouter();
  const [showPassword, setShowPassword] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const isRegister = mode === "register";

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError("");
    const data = new FormData(event.currentTarget);
    try {
      const email = String(data.get("email"));
      const password = String(data.get("password"));
      if (isRegister) {
        await registerAccount({
          displayName: String(data.get("displayName")),
          email,
          password,
        });
      } else {
        await loginAccount({ email, password });
      }
      router.replace("/conta");
      router.refresh();
    } catch (cause) {
      setError(
        cause instanceof AuthRequestError
          ? cause.message
          : "Não foi possível conectar ao Bycard.",
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <form
      className="auth-form"
      onSubmit={submit}
      aria-describedby={error ? "auth-error" : undefined}
    >
      {isRegister && (
        <label>
          Como podemos chamar você?
          <input
            name="displayName"
            autoComplete="name"
            minLength={2}
            maxLength={60}
            required
          />
        </label>
      )}
      <label>
        E-mail
        <input
          name="email"
          type="email"
          autoComplete="email"
          maxLength={254}
          required
        />
      </label>
      <div>
        <label htmlFor={`${mode}-password`}>Senha</label>
        <span className="password-field">
          <input
            id={`${mode}-password`}
            name="password"
            type={showPassword ? "text" : "password"}
            autoComplete={isRegister ? "new-password" : "current-password"}
            aria-describedby={isRegister ? "password-guidance" : undefined}
            minLength={isRegister ? 15 : undefined}
            maxLength={128}
            required
          />
          <button
            type="button"
            onClick={() => setShowPassword((value) => !value)}
          >
            {showPassword ? "Ocultar" : "Mostrar"}
          </button>
        </span>
        {isRegister && (
          <small id="password-guidance">Use pelo menos 15 caracteres.</small>
        )}
      </div>
      {error && (
        <p className="auth-error" id="auth-error" role="alert">
          {error}
        </p>
      )}
      <button className="auth-submit" type="submit" disabled={pending}>
        {pending
          ? "Só um instante…"
          : isRegister
            ? "Criar meu fichário"
            : "Entrar no fichário"}
      </button>
      <p className="auth-switch">
        {isRegister ? "Já possui uma conta?" : "Ainda não possui conta?"}{" "}
        <Link href={isRegister ? "/entrar" : "/criar-conta"}>
          {isRegister ? "Entrar" : "Criar conta"}
        </Link>
      </p>
    </form>
  );
}
