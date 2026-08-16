"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

import {
  CollectionRequestError,
  addPersonalCollection,
  fetchPersonalCollections,
  type PersonalCollection,
} from "@/features/collections/collections-api";

import { BrandHeader } from "./brand-header";
import { CatalogImage } from "./catalog-image";
import { type Collection, fetchCollections } from "./catalog-api";
import { ArrowRightIcon } from "./icons";

type LoadState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; collections: Collection[] };

export function CatalogScreen() {
  const [loadState, setLoadState] = useState<LoadState>({ status: "loading" });
  const [personalCollections, setPersonalCollections] = useState<
    PersonalCollection[]
  >([]);
  const [personalStatus, setPersonalStatus] = useState<
    "loading" | "guest" | "ready" | "error"
  >("loading");
  const [pendingSetId, setPendingSetId] = useState("");
  const [actionError, setActionError] = useState("");

  function loadCollections() {
    const controller = new AbortController();
    setLoadState({ status: "loading" });
    requestCollections(controller.signal);
  }

  function requestCollections(signal: AbortSignal) {
    fetchCollections(signal)
      .then(({ data }) => setLoadState({ status: "ready", collections: data }))
      .catch((error: unknown) => {
        if (error instanceof Error && error.name !== "AbortError") {
          setLoadState({ status: "error", message: error.message });
        }
      });
  }

  function loadPersonalCollections(signal?: AbortSignal) {
    Promise.resolve()
      .then(() => setPersonalStatus("loading"))
      .then(() => fetchPersonalCollections(signal))
      .then(({ data }) => {
        setPersonalCollections(data);
        setPersonalStatus("ready");
      })
      .catch((error: unknown) => {
        if (error instanceof Error && error.name === "AbortError") return;
        if (error instanceof CollectionRequestError && error.status === 401) {
          setPersonalStatus("guest");
        } else {
          setPersonalStatus("error");
        }
      });
  }

  async function followCollection(setId: string) {
    setPendingSetId(setId);
    setActionError("");
    try {
      const { data } = await addPersonalCollection(setId);
      setPersonalCollections((current) =>
        current.some((collection) => collection.setId === setId)
          ? current
          : [data, ...current],
      );
    } catch (error) {
      setActionError(
        error instanceof Error ? error.message : "Não foi possível acompanhar.",
      );
    } finally {
      setPendingSetId("");
    }
  }

  useEffect(() => {
    const controller = new AbortController();
    requestCollections(controller.signal);
    loadPersonalCollections(controller.signal);
    return () => controller.abort();
  }, []);

  return (
    <div className="app-shell">
      <BrandHeader />
      <main className="catalog-main">
        <section className="catalog-intro" aria-labelledby="catalog-title">
          <div>
            <h1 id="catalog-title">Escolha sua próxima coleção.</h1>
            <p>
              Abra o fichário, encontre cada número e descubra o que falta sem
              depender de planilhas.
            </p>
          </div>
        </section>

        <section
          className="catalog-section"
          aria-labelledby="available-title"
          aria-live="polite"
        >
          <div className="section-heading">
            <h2 id="available-title">Coleções disponíveis</h2>
          </div>

          {personalStatus === "ready" && personalCollections.length > 0 && (
            <div className="personal-overview" aria-label="Meu fichário">
              <strong>Meu fichário</strong>
              <span>
                {personalCollections.length} coleç
                {personalCollections.length === 1
                  ? "ão acompanhada"
                  : "ões acompanhadas"}
              </span>
            </div>
          )}
          {personalStatus === "error" && (
            <div className="personal-load-error" role="alert">
              <span>Não foi possível carregar seu fichário.</span>
              <button type="button" onClick={() => loadPersonalCollections()}>
                Tentar novamente
              </button>
            </div>
          )}
          {actionError && (
            <p className="collection-action-error" role="alert">
              {actionError}
            </p>
          )}

          {loadState.status === "loading" && <CatalogSkeleton />}
          {loadState.status === "error" && (
            <ErrorState message={loadState.message} onRetry={loadCollections} />
          )}
          {loadState.status === "ready" &&
            loadState.collections.length === 0 && <EmptyCatalog />}
          {loadState.status === "ready" && loadState.collections.length > 0 && (
            <div className="collection-list">
              {loadState.collections.map((collection, index) => (
                <CollectionRow
                  key={collection.id}
                  collection={collection}
                  priority={index === 0}
                  personalStatus={personalStatus}
                  personalCollection={personalCollections.find(
                    (personal) => personal.setId === collection.id,
                  )}
                  pending={pendingSetId === collection.id}
                  onFollow={followCollection}
                />
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  );
}

function CollectionRow({
  collection,
  priority,
  personalStatus,
  personalCollection,
  pending,
  onFollow,
}: {
  collection: Collection;
  priority: boolean;
  personalStatus: "loading" | "guest" | "ready" | "error";
  personalCollection?: PersonalCollection;
  pending: boolean;
  onFollow: (setId: string) => void;
}) {
  return (
    <article className="collection-row">
      <div
        className={`collection-cover${
          collection.slug === "tcgdex-me05"
            ? " collection-cover--escuridao-absoluta"
            : ""
        }`}
      >
        <CatalogImage
          src={collection.coverImageUrl}
          alt={`Capa da coleção ${collection.name}`}
          sizes="(max-width: 640px) 104px, 180px"
          priority={priority}
        />
      </div>
      <div className="collection-copy">
        <p className="collection-series">
          {collection.seriesName ?? "Série não informada"}
        </p>
        <h3>{collection.name}</h3>
        <dl className="collection-facts">
          <div>
            <dt>Cartas</dt>
            <dd>{collection.totalCards}</dd>
          </div>
          <div>
            <dt>Lançamento</dt>
            <dd>{formatDate(collection.releaseDate)}</dd>
          </div>
          <div>
            <dt>Idioma</dt>
            <dd>{collection.language}</dd>
          </div>
        </dl>
      </div>
      <div className="collection-actions">
        {personalCollection && (
          <div className="collection-progress">
            <span>
              {Math.round(personalCollection.completionPercentage)}% completo
            </span>
            <progress
              max={100}
              value={personalCollection.completionPercentage}
              aria-label={`${Math.round(personalCollection.completionPercentage)}% da coleção concluída`}
            />
          </div>
        )}
        {personalStatus === "ready" && !personalCollection && (
          <button
            className="follow-collection"
            type="button"
            disabled={pending}
            onClick={() => onFollow(collection.id)}
          >
            {pending ? "Adicionando…" : "Acompanhar"}
          </button>
        )}
        {personalStatus === "guest" && (
          <Link className="follow-collection" href="/entrar?motivo=acompanhar">
            Entrar para acompanhar
          </Link>
        )}
        <Link
          className="open-collection"
          href={`/collections/${collection.id}`}
        >
          Abrir fichário
          <ArrowRightIcon aria-hidden="true" />
        </Link>
      </div>
    </article>
  );
}

function CatalogSkeleton() {
  return (
    <div
      className="collection-list"
      aria-label="Carregando coleções"
      role="status"
    >
      {[0, 1].map((item) => (
        <div
          className="collection-row skeleton-row"
          key={item}
          aria-hidden="true"
        >
          <span className="skeleton-cover" />
          <span className="skeleton-lines" />
          <span className="skeleton-action" />
        </div>
      ))}
      <span className="sr-only">Carregando coleções.</span>
    </div>
  );
}

function ErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <div className="message-state error-state" role="alert">
      <div>
        <h3>O catálogo não respondeu.</h3>
        <p>{message} Verifique a conexão e tente novamente.</p>
      </div>
      <button type="button" onClick={onRetry}>
        Tentar novamente
      </button>
    </div>
  );
}

function EmptyCatalog() {
  return (
    <div className="message-state">
      <div>
        <h3>Nenhuma coleção disponível.</h3>
        <p>Quando o catálogo for importado, suas coleções aparecerão aqui.</p>
      </div>
    </div>
  );
}

function formatDate(date: string) {
  return new Intl.DateTimeFormat("pt-BR", {
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  })
    .format(new Date(`${date}T00:00:00Z`))
    .replace(" de ", " ");
}
