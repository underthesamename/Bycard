"use client";

import Image from "next/image";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useRef, useState } from "react";

import {
  AuthRequestError,
  deleteProfileAvatar,
  profileAvatarUrl,
  updateProfile,
  uploadProfileAvatar,
  type AuthUser,
} from "./auth-api";

const MAX_AVATAR_BYTES = 2 * 1024 * 1024;
const AVATAR_TYPES = new Set(["image/jpeg", "image/png", "image/webp"]);

type Feedback = { kind: "success" | "error"; message: string } | null;

export function ProfileEditor({ initialUser }: { initialUser: AuthUser }) {
  const router = useRouter();
  const fileInput = useRef<HTMLInputElement>(null);
  const [displayName, setDisplayName] = useState(initialUser.displayName);
  const [savedDisplayName, setSavedDisplayName] = useState(
    initialUser.displayName,
  );
  const [avatarVersion, setAvatarVersion] = useState(initialUser.avatarVersion);
  const [previewUrl, setPreviewUrl] = useState("");
  const [avatarAvailable, setAvatarAvailable] = useState(
    Boolean(initialUser.avatarVersion),
  );
  const [savingName, setSavingName] = useState(false);
  const [savingPhoto, setSavingPhoto] = useState(false);
  const [nameFeedback, setNameFeedback] = useState<Feedback>(null);
  const [photoFeedback, setPhotoFeedback] = useState<Feedback>(null);

  useEffect(
    () => () => {
      if (previewUrl) URL.revokeObjectURL(previewUrl);
    },
    [previewUrl],
  );

  const initials = initialsFor(savedDisplayName);
  const avatarSource = previewUrl
    ? previewUrl
    : avatarVersion
      ? profileAvatarUrl(avatarVersion)
      : "";
  const nameChanged = displayName.trim() !== savedDisplayName;

  async function saveDisplayName(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const normalizedName = displayName.trim();
    if (normalizedName.length < 2) {
      setNameFeedback({
        kind: "error",
        message: "Use pelo menos 2 caracteres no nome.",
      });
      return;
    }
    setSavingName(true);
    setNameFeedback(null);
    try {
      const { user } = await updateProfile(normalizedName);
      setDisplayName(user.displayName);
      setSavedDisplayName(user.displayName);
      setNameFeedback({ kind: "success", message: "Nome atualizado." });
      router.refresh();
    } catch (error) {
      setNameFeedback({ kind: "error", message: profileErrorMessage(error) });
    } finally {
      setSavingName(false);
    }
  }

  async function choosePhoto(file: File | undefined) {
    if (!file) return;
    setPhotoFeedback(null);
    if (!AVATAR_TYPES.has(file.type) || file.size > MAX_AVATAR_BYTES) {
      setPhotoFeedback({
        kind: "error",
        message: "Escolha uma imagem JPEG, PNG ou WebP de até 2 MB.",
      });
      if (fileInput.current) fileInput.current.value = "";
      return;
    }
    const nextPreviewUrl = URL.createObjectURL(file);
    setPreviewUrl(nextPreviewUrl);
    setAvatarAvailable(true);
    setSavingPhoto(true);
    try {
      const result = await uploadProfileAvatar(file);
      setAvatarVersion(result.avatarVersion);
      setPreviewUrl("");
      setAvatarAvailable(true);
      setPhotoFeedback({ kind: "success", message: "Foto atualizada." });
      router.refresh();
    } catch (error) {
      setPreviewUrl("");
      setAvatarAvailable(Boolean(avatarVersion));
      setPhotoFeedback({ kind: "error", message: profileErrorMessage(error) });
    } finally {
      setSavingPhoto(false);
      if (fileInput.current) fileInput.current.value = "";
    }
  }

  async function removePhoto() {
    setSavingPhoto(true);
    setPhotoFeedback(null);
    try {
      await deleteProfileAvatar();
      setAvatarVersion(null);
      setPreviewUrl("");
      setAvatarAvailable(false);
      setPhotoFeedback({ kind: "success", message: "Foto removida." });
      router.refresh();
    } catch (error) {
      setPhotoFeedback({ kind: "error", message: profileErrorMessage(error) });
    } finally {
      setSavingPhoto(false);
    }
  }

  return (
    <section className="profile-editor" aria-labelledby="profile-editor-title">
      <div className="profile-editor-heading">
        <div>
          <h2 id="profile-editor-title">Seu perfil</h2>
          <p>Escolha como seu nome e sua foto aparecem no Bycard.</p>
        </div>
        <span className="profile-privacy">Visível somente para você</span>
      </div>

      <div className="profile-photo-row">
        <div className="profile-avatar" aria-label="Foto atual do perfil">
          {avatarSource && avatarAvailable ? (
            <Image
              src={avatarSource}
              alt=""
              width={128}
              height={128}
              unoptimized
              onError={() => setAvatarAvailable(false)}
            />
          ) : (
            <span aria-hidden="true">{initials}</span>
          )}
        </div>
        <div className="profile-photo-actions">
          <strong>Foto do colecionador</strong>
          <p>JPEG, PNG ou WebP. O recorte final será quadrado.</p>
          <div>
            <label className="profile-file-button">
              {savingPhoto ? "Processando…" : "Escolher foto"}
              <input
                ref={fileInput}
                type="file"
                accept="image/jpeg,image/png,image/webp"
                disabled={savingPhoto}
                onChange={(event) => void choosePhoto(event.target.files?.[0])}
              />
            </label>
            {(avatarVersion || previewUrl) && (
              <button
                className="profile-remove-photo"
                type="button"
                disabled={savingPhoto}
                onClick={() => void removePhoto()}
              >
                Remover foto
              </button>
            )}
          </div>
          {photoFeedback && (
            <p
              className={`profile-feedback profile-feedback--${photoFeedback.kind}`}
              role={photoFeedback.kind === "error" ? "alert" : "status"}
            >
              {photoFeedback.message}
            </p>
          )}
        </div>
      </div>

      <form className="profile-name-form" onSubmit={saveDisplayName}>
        <div>
          <label htmlFor="profile-display-name">Nome de exibição</label>
          <input
            id="profile-display-name"
            name="displayName"
            value={displayName}
            minLength={2}
            maxLength={60}
            autoComplete="name"
            onChange={(event) => {
              setDisplayName(event.target.value);
              setNameFeedback(null);
            }}
            required
          />
          <small>
            Seu identificador continua sendo @{initialUser.username}.
          </small>
        </div>
        <button type="submit" disabled={savingName || !nameChanged}>
          {savingName ? "Salvando…" : "Salvar nome"}
        </button>
        {nameFeedback && (
          <p
            className={`profile-feedback profile-feedback--${nameFeedback.kind}`}
            role={nameFeedback.kind === "error" ? "alert" : "status"}
          >
            {nameFeedback.message}
          </p>
        )}
      </form>
    </section>
  );
}

function initialsFor(displayName: string) {
  return displayName
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toLocaleUpperCase("pt-BR");
}

function profileErrorMessage(error: unknown) {
  if (
    error instanceof AuthRequestError &&
    error.code === "authentication_required"
  ) {
    return "Sua sessão expirou. Entre novamente para continuar.";
  }
  return error instanceof Error
    ? error.message
    : "Não foi possível atualizar seu perfil.";
}
