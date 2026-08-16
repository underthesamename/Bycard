import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ProfileEditor } from "./profile-editor";

const navigation = vi.hoisted(() => ({ refresh: vi.fn() }));
const BrowserURL = URL;

vi.mock("next/navigation", () => ({
  useRouter: () => navigation,
}));

const user = {
  id: "019fd3c0-a42b-7f70-9e0d-18eebdfb8212",
  displayName: "Ana Colecionadora",
  username: "ana.tcg",
  email: "ana@example.com",
  avatarVersion: null,
};

beforeEach(() => {
  class TestURL extends BrowserURL {
    static createObjectURL = vi.fn(() => "blob:avatar-preview");
    static revokeObjectURL = vi.fn();
  }
  vi.stubGlobal("URL", TestURL);
});

afterEach(() => {
  vi.unstubAllGlobals();
  navigation.refresh.mockReset();
});

describe("ProfileEditor", () => {
  it("atualiza o nome de exibição com proteção CSRF", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(200, { csrfToken: "csrf-seguro" }))
      .mockResolvedValueOnce(
        jsonResponse(200, {
          user: { ...user, displayName: "Ana Silva" },
        }),
      );
    vi.stubGlobal("fetch", fetchMock);
    render(<ProfileEditor initialUser={user} />);

    fireEvent.change(screen.getByLabelText("Nome de exibição"), {
      target: { value: "Ana Silva" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Salvar nome" }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Nome atualizado.",
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/me/profile",
      expect.objectContaining({
        method: "PUT",
        credentials: "include",
        body: JSON.stringify({ displayName: "Ana Silva" }),
        headers: expect.objectContaining({ "x-csrf-token": "csrf-seguro" }),
      }),
    );
    expect(navigation.refresh).toHaveBeenCalledOnce();
  });

  it("envia uma foto válida com o tipo correto", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(200, { csrfToken: "csrf-seguro" }))
      .mockResolvedValueOnce(
        jsonResponse(200, {
          avatarVersion: "019fd3c0-a42b-7f70-9e0d-18eebdfb8999",
        }),
      );
    vi.stubGlobal("fetch", fetchMock);
    render(<ProfileEditor initialUser={user} />);
    const photo = new File([new Uint8Array([1, 2, 3])], "perfil.png", {
      type: "image/png",
    });

    fireEvent.change(screen.getByLabelText("Escolher foto"), {
      target: { files: [photo] },
    });

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Foto atualizada.",
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/me/avatar",
      expect.objectContaining({
        method: "PUT",
        credentials: "include",
        body: photo,
        headers: expect.objectContaining({
          "Content-Type": "image/png",
          "x-csrf-token": "csrf-seguro",
        }),
      }),
    );
  });

  it("rejeita uma foto acima de 2 MB antes de chamar a API", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    render(<ProfileEditor initialUser={user} />);
    const oversizedPhoto = new File(
      [new Uint8Array(2 * 1024 * 1024 + 1)],
      "grande.webp",
      { type: "image/webp" },
    );

    fireEvent.change(screen.getByLabelText("Escolher foto"), {
      target: { files: [oversizedPhoto] },
    });

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Escolha uma imagem JPEG, PNG ou WebP de até 2 MB.",
    );
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}
