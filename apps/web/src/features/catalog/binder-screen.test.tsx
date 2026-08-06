import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { BinderScreen } from "./binder-screen";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), refresh: vi.fn() }),
}));

const setId = "019fd3c0-a42b-7f70-9e0d-18eebdfb8212";
const collection = {
  id: setId,
  slug: "horizonte-solar",
  name: "Horizonte Solar",
  seriesName: "Atlas de Luz",
  releaseDate: "2026-01-17",
  totalCards: 18,
  coverImageUrl: "/demo/placeholders/horizonte-solar.svg",
  language: "pt-BR",
};
const cards = Array.from({ length: 18 }, (_, index) => ({
  id: `card-${index + 1}`,
  setId,
  localNumber: String(index + 1).padStart(3, "0"),
  printedNumber: `${String(index + 1).padStart(3, "0")}/018`,
  name: index === 15 ? "Silêncio Azul" : `Carta ${index + 1}`,
  rarity: "comum",
  artist: "Ateliê Bycard",
  imageSmallUrl: index === 15 ? null : "/demo/placeholders/luz.svg",
  imageLargeUrl: index === 15 ? null : "/demo/placeholders/luz.svg",
  sortOrder: index + 1,
}));

afterEach(() => vi.unstubAllGlobals());

describe("BinderScreen", () => {
  it("busca, pagina e mostra fallback sem perder os detalhes", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation(mockCatalogRequest));
    render(<BinderScreen setId={setId} />);

    expect(
      await screen.findByRole("heading", { name: "Horizonte Solar" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Página 1 de 2" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /carta 001, Carta 1/ }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /carta 010/ }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Próxima página" }));
    expect(
      screen.getByRole("heading", { name: "Página 2 de 2" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /carta 010, Carta 10/ }),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByRole("searchbox"), {
      target: { value: "016" },
    });
    expect(screen.getByText("1 carta encontrada")).toBeInTheDocument();
    expect(
      screen.getByRole("img", { name: /Silêncio Azul. Imagem indisponível/ }),
    ).toBeInTheDocument();

    const cardButton = screen.getByRole("button", {
      name: /carta 016, Silêncio Azul/,
    });
    fireEvent.click(cardButton);
    expect(
      screen.getByRole("dialog", { name: "Silêncio Azul" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Entre e acompanhe a coleção para registrar cartas/),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Fechar detalhes" }));
    await waitFor(() => expect(cardButton).toHaveFocus());
  });

  it("mostra resultado vazio e limpa a busca", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation(mockCatalogRequest));
    render(<BinderScreen setId={setId} />);
    await screen.findByRole("heading", { name: "Horizonte Solar" });

    fireEvent.change(screen.getByRole("searchbox"), {
      target: { value: "inexistente" },
    });
    expect(
      screen.getByText("Nenhuma carta corresponde a “inexistente”."),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Limpar busca" }));
    expect(
      screen.getByRole("heading", { name: "Página 1 de 2" }),
    ).toBeInTheDocument();
  });

  it("atualiza quantidade, progresso e filtros no fichário acompanhado", async () => {
    let quantity = 0;
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockImplementation((input: RequestInfo | URL, init?: RequestInit) => {
          const url = String(input);
          if (url.includes("/auth/me")) {
            return Promise.resolve(
              jsonResponse({
                user: {
                  id: "user-1",
                  displayName: "Ana",
                  email: "ana@example.com",
                },
                expiresAt: "2026-08-06T18:00:00Z",
              }),
            );
          }
          if (url.includes("/auth/csrf")) {
            return Promise.resolve(jsonResponse({ csrfToken: "csrf-token" }));
          }
          if (url.includes("/me/collections/") && init?.method === "PUT") {
            quantity = (JSON.parse(String(init.body)) as { quantity: number })
              .quantity;
            return Promise.resolve(jsonResponse(personalDetail(quantity)));
          }
          if (url.includes("/me/collections/")) {
            return Promise.resolve(jsonResponse(personalDetail(quantity)));
          }
          return Promise.resolve(jsonResponse({ data: collection }));
        }),
    );

    render(<BinderScreen setId={setId} />);
    const firstCard = await screen.findByRole("button", {
      name: /carta 001, Carta 1. Faltante/,
    });
    fireEvent.click(firstCard);
    fireEvent.click(
      screen.getByRole("button", { name: "Adicionar uma cópia de Carta 1" }),
    );

    expect(
      await screen.findByText("Quantidade de Carta 1 atualizada para 1."),
    ).toBeInTheDocument();
    expect(screen.getByText("Quantidade possuída:")).toHaveTextContent("1");
    fireEvent.click(screen.getByRole("button", { name: "Fechar detalhes" }));
    fireEvent.click(screen.getByRole("button", { name: "Possuídas" }));
    expect(
      screen.getByRole("button", { name: /Carta 1. 1 cópia/ }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Carta 2/ }),
    ).not.toBeInTheDocument();
  });
});

function personalDetail(quantity: number) {
  return {
    data: {
      collection: {
        id: "personal-1",
        setId,
        slug: collection.slug,
        name: collection.name,
        coverImageUrl: collection.coverImageUrl,
        totalUnique: 18,
        ownedUnique: quantity > 0 ? 1 : 0,
        missingUnique: quantity > 0 ? 17 : 18,
        totalCopies: quantity,
        duplicateCopies: Math.max(quantity - 1, 0),
        completionPercentage: quantity > 0 ? 100 / 18 : 0,
      },
      cards: cards.map((card, index) => ({
        ...card,
        quantity: index === 0 ? quantity : 0,
      })),
    },
  };
}

function mockCatalogRequest(input: RequestInfo | URL) {
  const url = String(input);
  if (url.includes("/auth/me")) {
    return Promise.resolve(
      new Response(
        JSON.stringify({
          error: {
            code: "authentication_required",
            message: "Sua sessão não é válida.",
          },
        }),
        { status: 401, headers: { "Content-Type": "application/json" } },
      ),
    );
  }
  if (url.includes("/cards?")) {
    return Promise.resolve(
      jsonResponse({
        data: cards,
        pagination: { page: 1, pageSize: 100, totalItems: 18, totalPages: 1 },
      }),
    );
  }
  return Promise.resolve(jsonResponse({ data: collection }));
}

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}
