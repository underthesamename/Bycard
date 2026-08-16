import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CatalogScreen } from "./catalog-screen";

const collection = {
  id: "019fd3c0-a42b-7f70-9e0d-18eebdfb8212",
  slug: "horizonte-solar",
  name: "Horizonte Solar",
  seriesName: "Atlas de Luz",
  releaseDate: "2026-01-17",
  totalCards: 18,
  coverImageUrl: null,
  language: "pt-BR",
};

afterEach(() => vi.unstubAllGlobals());

describe("CatalogScreen", () => {
  it("mostra carregamento e renderiza o catálogo da API", async () => {
    const fetchMock = vi.fn().mockImplementation((input: RequestInfo | URL) =>
      String(input).includes("/auth/me") ||
      String(input).includes("/me/collections")
        ? Promise.resolve(unauthorizedResponse())
        : Promise.resolve(
            jsonResponse({
              data: [collection],
              pagination: {
                page: 1,
                pageSize: 20,
                totalItems: 1,
                totalPages: 1,
              },
            }),
          ),
    );
    vi.stubGlobal("fetch", fetchMock);

    render(<CatalogScreen />);
    expect(
      screen.getByRole("status", { name: "Carregando coleções" }),
    ).toBeInTheDocument();

    expect(
      await screen.findByRole("heading", { name: "Horizonte Solar" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /Abrir fichário/ }),
    ).toHaveAttribute("href", `/collections/${collection.id}`);
    expect(
      screen.getByRole("img", { name: /Capa da coleção Horizonte Solar/ }),
    ).toHaveTextContent("Imagem indisponível");
    expect(screen.queryByText(collection.slug)).not.toBeInTheDocument();
    expect(
      fetchMock.mock.calls.filter(([input]) =>
        String(input).includes("/auth/me"),
      ),
    ).toHaveLength(1);
  });

  it("explica a falha e permite tentar novamente", async () => {
    let catalogAttempts = 0;
    const fetchMock = vi.fn().mockImplementation((input: RequestInfo | URL) => {
      if (
        String(input).includes("/auth/me") ||
        String(input).includes("/me/collections")
      ) {
        return Promise.resolve(unauthorizedResponse());
      }
      catalogAttempts += 1;
      if (catalogAttempts === 1) {
        return Promise.reject(new Error("API indisponível"));
      }
      return Promise.resolve(
        jsonResponse({
          data: [],
          pagination: { page: 1, pageSize: 20, totalItems: 0, totalPages: 0 },
        }),
      );
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<CatalogScreen />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "API indisponível",
    );
    fireEvent.click(screen.getByRole("button", { name: "Tentar novamente" }));

    expect(
      await screen.findByText("Nenhuma coleção disponível."),
    ).toBeInTheDocument();
    expect(catalogAttempts).toBe(2);
  });
});

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}

function unauthorizedResponse() {
  return new Response(
    JSON.stringify({
      error: {
        code: "authentication_required",
        message: "Sua sessão não é válida.",
      },
    }),
    { status: 401, headers: { "Content-Type": "application/json" } },
  );
}
