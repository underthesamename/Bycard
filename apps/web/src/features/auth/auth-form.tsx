"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { FormEvent, useState } from "react";

import { AuthRequestError, loginAccount, registerAccount } from "./auth-api";

type AuthFormProps = Readonly<{
  mode: "login" | "register";
  onAuthenticated?: () => void | Promise<void>;
}>;

type SubmissionState = "idle" | "submitting" | "authenticated";

export function AuthForm({ mode, onAuthenticated }: AuthFormProps) {
  const router = useRouter();
  const [showPassword, setShowPassword] = useState(false);
  const [submissionState, setSubmissionState] =
    useState<SubmissionState>("idle");
  const [error, setError] = useState("");
  const [password, setPassword] = useState("");
  const isRegister = mode === "register";
  const pending = submissionState !== "idle";

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmissionState("submitting");
    setError("");
    const data = new FormData(event.currentTarget);
    try {
      const submittedPassword = String(data.get("password"));
      if (isRegister) {
        await registerAccount({
          displayName: String(data.get("displayName")),
          username: String(data.get("username")),
          email: String(data.get("email")),
          password: submittedPassword,
        });
      } else {
        await loginAccount({
          identifier: String(data.get("identifier")),
          password: submittedPassword,
        });
      }
      setSubmissionState("authenticated");
      await onAuthenticated?.();
      router.replace("/");
      router.refresh();
    } catch (cause) {
      setError(
        cause instanceof AuthRequestError
          ? cause.message
          : "Não foi possível conectar ao Bycard.",
      );
      setSubmissionState("idle");
    }
  }

  return (
    <form
      className="auth-form"
      onSubmit={submit}
      aria-describedby={error ? "auth-error" : undefined}
    >
      {isRegister && (
        <>
          <label>
            Seu nome
            <input
              name="displayName"
              autoComplete="name"
              minLength={2}
              maxLength={60}
              placeholder="Como podemos chamar você?"
              required
            />
          </label>
          <div>
            <label htmlFor="register-username">Nome de usuário</label>
            <span className="username-field">
              <span aria-hidden="true">@</span>
              <input
                id="register-username"
                name="username"
                autoComplete="username"
                minLength={3}
                maxLength={24}
                pattern="[a-zA-Z0-9]+([._-][a-zA-Z0-9]+)*"
                placeholder="treinador.ana"
                aria-describedby="username-guidance"
                required
              />
            </span>
            <small id="username-guidance">
              De 3 a 24 letras, números, ponto, hífen ou sublinhado.
            </small>
          </div>
        </>
      )}
      {isRegister ? (
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
      ) : (
        <label>
          E-mail ou nome de usuário
          <input
            name="identifier"
            autoComplete="username"
            maxLength={254}
            required
          />
        </label>
      )}
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
            value={password}
            onChange={(event) => setPassword(event.target.value)}
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
          <ul className="password-rules" id="password-guidance">
            <li data-met={password.length >= 15}>Pelo menos 15 caracteres</li>
            <li data-met={password.length <= 128}>No máximo 128 caracteres</li>
            <li data-met={password.trim().length === password.length}>
              Sem espaços no começo ou no fim
            </li>
          </ul>
        )}
      </div>
      {error && (
        <p className="auth-error" id="auth-error" role="alert">
          {error}
        </p>
      )}
      <button className="auth-submit" type="submit" disabled={pending}>
        {submissionState === "submitting"
          ? "Verificando…"
          : submissionState === "authenticated"
            ? "Abrindo seu fichário…"
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
