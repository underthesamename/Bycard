import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AuthForm } from "./auth-form";
import { LogoutButton } from "./logout-button";

const navigation = vi.hoisted(() => ({
  replace: vi.fn(),
  refresh: vi.fn(),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => navigation,
}));

afterEach(() => {
  vi.unstubAllGlobals();
  navigation.replace.mockReset();
  navigation.refresh.mockReset();
});

describe("AuthForm", () => {
  it("envia o cadastro com credenciais incluídas e redireciona", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse(201, {
        user: {
          id: "019fd3c0-a42b-7f70-9e0d-18eebdfb8212",
          displayName: "Ana",
          email: "ana@example.com",
        },
        expiresAt: "2026-09-05T12:00:00Z",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    render(<AuthForm mode="register" />);
    fireEvent.change(screen.getByLabelText("Como podemos chamar você?"), {
      target: { value: "Ana" },
    });
    fireEvent.change(screen.getByLabelText("E-mail"), {
      target: { value: "ana@example.com" },
    });
    fireEvent.change(screen.getByLabelText("Senha"), {
      target: { value: "uma-senha-segura-com-15" },
    });
    fireEvent.submit(
      screen
        .getByRole("button", { name: "Criar meu fichário" })
        .closest("form")!,
    );

    await waitFor(() =>
      expect(navigation.replace).toHaveBeenCalledWith("/conta"),
    );
    expect(navigation.refresh).toHaveBeenCalledOnce();
    expect(fetchMock).toHaveBeenCalledWith(
      "http://localhost:8080/api/v1/auth/register",
      expect.objectContaining({
        method: "POST",
        credentials: "include",
        body: JSON.stringify({
          displayName: "Ana",
          email: "ana@example.com",
          password: "uma-senha-segura-com-15",
        }),
      }),
    );
  });

  it("associa o erro da API ao formulário sem redirecionar", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        jsonResponse(401, {
          error: {
            code: "invalid_credentials",
            message: "E-mail ou senha inválidos.",
          },
        }),
      ),
    );

    render(<AuthForm mode="login" />);
    fireEvent.change(screen.getByLabelText("E-mail"), {
      target: { value: "ana@example.com" },
    });
    fireEvent.change(screen.getByLabelText("Senha"), {
      target: { value: "senha-incorreta" },
    });
    fireEvent.submit(
      screen
        .getByRole("button", { name: "Entrar no fichário" })
        .closest("form")!,
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "E-mail ou senha inválidos.",
    );
    expect(navigation.replace).not.toHaveBeenCalled();
  });

  it("permite revelar e ocultar a senha", () => {
    render(<AuthForm mode="login" />);
    const password = screen.getByLabelText("Senha");
    expect(password).toHaveAttribute("type", "password");
    fireEvent.click(screen.getByRole("button", { name: "Mostrar" }));
    expect(password).toHaveAttribute("type", "text");
    fireEvent.click(screen.getByRole("button", { name: "Ocultar" }));
    expect(password).toHaveAttribute("type", "password");
  });
});

describe("LogoutButton", () => {
  it("obtém o token CSRF, encerra a sessão e redireciona", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(200, { csrfToken: "csrf-seguro" }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    render(<LogoutButton />);
    fireEvent.click(screen.getByRole("button", { name: "Sair da conta" }));

    await waitFor(() =>
      expect(navigation.replace).toHaveBeenCalledWith("/entrar"),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://localhost:8080/api/v1/auth/logout",
      expect.objectContaining({
        method: "POST",
        credentials: "include",
        headers: expect.objectContaining({ "x-csrf-token": "csrf-seguro" }),
      }),
    );
  });
});

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}
