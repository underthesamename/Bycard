"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";

import {
  CollectionRequestError,
  fetchPersonalCollection,
  removePersonalCollection,
  updateCardQuantity,
  type CollectionStats,
  type PersonalCard,
} from "@/features/collections/collections-api";

import { BrandHeader } from "./brand-header";
import { CatalogImage } from "./catalog-image";
import { type Collection, fetchCards, fetchCollection } from "./catalog-api";
import {
  ArrowLeftIcon,
  ArrowRightIcon,
  CloseIcon,
  ImageIcon,
  SearchIcon,
} from "./icons";

const CARDS_PER_PAGE = 9;
const ABOVE_FOLD_CARD_IMAGES = 3;
const MAX_CARD_TILT_DEGREES = 7;
const CARD_MOTION_PROPERTIES = [
  "--card-tilt-x",
  "--card-tilt-y",
  "--card-sheen-x",
  "--card-sheen-y",
] as const;

type BinderState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | {
      status: "ready";
      collection: Collection;
      cards: PersonalCard[];
      mode: "guest" | "untracked" | "tracked";
      stats: CollectionStats | null;
    };

type OwnershipFilter = "all" | "owned" | "missing";

export function BinderScreen({ setId }: { setId: string }) {
  const router = useRouter();
  const [binderState, setBinderState] = useState<BinderState>({
    status: "loading",
  });
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [selectedCard, setSelectedCard] = useState<PersonalCard | null>(null);
  const [ownershipFilter, setOwnershipFilter] =
    useState<OwnershipFilter>("all");
  const [pendingCardId, setPendingCardId] = useState("");
  const [mutationFeedback, setMutationFeedback] = useState<{
    kind: "success" | "error";
    message: string;
  } | null>(null);
  const [removing, setRemoving] = useState(false);

  function loadBinder() {
    const controller = new AbortController();
    setBinderState({ status: "loading" });
    fetchBinder(setId, controller.signal)
      .then(setBinderState)
      .catch((error: unknown) => setBinderError(error, setBinderState));
  }

  useEffect(() => {
    const controller = new AbortController();
    fetchBinder(setId, controller.signal)
      .then(setBinderState)
      .catch((error: unknown) => setBinderError(error, setBinderState));
    return () => controller.abort();
  }, [setId]);

  const filteredCards = useMemo(
    () =>
      filterCards(
        binderState.status === "ready" ? binderState.cards : [],
        search,
        ownershipFilter,
      ),
    [binderState, ownershipFilter, search],
  );
  const totalPages = Math.ceil(filteredCards.length / CARDS_PER_PAGE);
  const currentPage = totalPages === 0 ? 1 : Math.min(page, totalPages);
  const visibleCards = filteredCards.slice(
    (currentPage - 1) * CARDS_PER_PAGE,
    currentPage * CARDS_PER_PAGE,
  );

  function closeDetails() {
    const cardId = selectedCard?.id;
    setSelectedCard(null);
    if (cardId) {
      requestAnimationFrame(() => {
        document
          .querySelector<HTMLButtonElement>(`[data-card-id="${cardId}"]`)
          ?.focus();
      });
    }
  }

  async function changeQuantity(card: PersonalCard, quantity: number) {
    if (binderState.status !== "ready" || binderState.mode !== "tracked") {
      return;
    }
    setPendingCardId(card.id);
    setMutationFeedback(null);
    try {
      const { data } = await updateCardQuantity(setId, card.id, quantity);
      setBinderState({
        ...binderState,
        cards: data.cards,
        stats: data.collection,
      });
      setSelectedCard(
        data.cards.find((currentCard) => currentCard.id === card.id) ?? null,
      );
      setMutationFeedback({
        kind: "success",
        message:
          quantity === 0
            ? `${card.name} foi marcada como faltante.`
            : `Quantidade de ${card.name} atualizada para ${quantity}.`,
      });
    } catch (error) {
      setMutationFeedback({
        kind: "error",
        message:
          error instanceof CollectionRequestError && error.status === 401
            ? "Sua sessão expirou. Entre novamente para continuar."
            : error instanceof Error
              ? error.message
              : "Não foi possível atualizar a quantidade.",
      });
    } finally {
      setPendingCardId("");
    }
  }

  async function removeCollection() {
    const confirmed = window.confirm(
      "Parar de acompanhar esta coleção e apagar todas as quantidades registradas?",
    );
    if (!confirmed) return;
    setRemoving(true);
    setMutationFeedback(null);
    try {
      await removePersonalCollection(setId);
      router.push("/");
      router.refresh();
    } catch (error) {
      setMutationFeedback({
        kind: "error",
        message:
          error instanceof CollectionRequestError && error.status === 401
            ? "Sua sessão expirou. Entre novamente para continuar."
            : error instanceof Error
              ? error.message
              : "Não foi possível remover a coleção.",
      });
      setRemoving(false);
    }
  }

  return (
    <div className="app-shell binder-shell">
      <BrandHeader />
      <main className="binder-main">
        {binderState.status === "loading" && <BinderSkeleton />}
        {binderState.status === "error" && (
          <BinderError message={binderState.message} onRetry={loadBinder} />
        )}
        {binderState.status === "ready" && (
          <>
            <section className="binder-toolbar" aria-labelledby="binder-title">
              <div className="binder-heading">
                <Link href="/" className="back-link">
                  <ArrowLeftIcon aria-hidden="true" />
                  Coleções
                </Link>
                <div>
                  <p>
                    {binderState.collection.seriesName ?? "Coleção de cartas"}
                  </p>
                  <h1 id="binder-title">{binderState.collection.name}</h1>
                </div>
              </div>
              <div className="binder-summary" aria-label="Resumo da coleção">
                <strong>
                  {binderState.stats
                    ? `${Math.round(binderState.stats.completionPercentage)}%`
                    : binderState.collection.totalCards}
                </strong>
                <span>
                  {binderState.stats
                    ? `${binderState.stats.ownedUnique} de ${binderState.stats.totalUnique} obtidas`
                    : "cartas no catálogo"}
                </span>
                {binderState.stats && (
                  <progress
                    max={100}
                    value={binderState.stats.completionPercentage}
                    aria-label={`${Math.round(binderState.stats.completionPercentage)}% da coleção concluída`}
                  />
                )}
              </div>
              <label className="search-field">
                <SearchIcon aria-hidden="true" />
                <span className="sr-only">Buscar por número ou nome</span>
                <input
                  type="search"
                  value={search}
                  onChange={(event) => {
                    setSearch(event.target.value);
                    setPage(1);
                  }}
                  placeholder="Buscar por número ou nome"
                />
              </label>
            </section>

            {binderState.mode === "tracked" && binderState.stats && (
              <section
                className="ownership-tools"
                aria-label="Progresso e filtros"
              >
                <dl className="ownership-stats">
                  <div>
                    <dt>Faltam</dt>
                    <dd>{binderState.stats.missingUnique}</dd>
                  </div>
                  <div>
                    <dt>Cópias</dt>
                    <dd>{binderState.stats.totalCopies}</dd>
                  </div>
                  <div>
                    <dt>Repetidas</dt>
                    <dd>{binderState.stats.duplicateCopies}</dd>
                  </div>
                </dl>
                <div className="ownership-filters" aria-label="Filtrar cartas">
                  {(["all", "owned", "missing"] as const).map((filter) => (
                    <button
                      key={filter}
                      type="button"
                      aria-pressed={ownershipFilter === filter}
                      onClick={() => {
                        setOwnershipFilter(filter);
                        setPage(1);
                      }}
                    >
                      {filter === "all"
                        ? "Todas"
                        : filter === "owned"
                          ? "Possuídas"
                          : "Faltantes"}
                    </button>
                  ))}
                </div>
                <button
                  className="remove-collection"
                  type="button"
                  disabled={removing}
                  onClick={removeCollection}
                >
                  {removing ? "Removendo…" : "Parar de acompanhar"}
                </button>
              </section>
            )}

            {binderState.mode !== "tracked" && (
              <p className="ownership-callout">
                {binderState.mode === "guest" ? (
                  <>
                    Entre na sua conta para registrar cartas e acompanhar o
                    progresso.{" "}
                    <Link href="/entrar?motivo=acompanhar">Entrar</Link>
                  </>
                ) : (
                  <>
                    Esta coleção ainda não está no seu fichário. Volte às{" "}
                    <Link href="/">coleções</Link> para acompanhá-la.
                  </>
                )}
              </p>
            )}

            {mutationFeedback && (
              <p
                className={`mutation-message is-${mutationFeedback.kind}`}
                role={mutationFeedback.kind === "error" ? "alert" : "status"}
              >
                {mutationFeedback.message}
              </p>
            )}

            <section className="binder-workspace" aria-labelledby="page-label">
              <div className="page-bar">
                <div>
                  <h2 id="page-label">
                    {totalPages === 0
                      ? "Nenhum resultado"
                      : `Página ${currentPage} de ${totalPages}`}
                  </h2>
                  <p>
                    {filteredCards.length}{" "}
                    {filteredCards.length === 1
                      ? "carta encontrada"
                      : "cartas encontradas"}
                  </p>
                </div>
                <div
                  className="page-controls"
                  aria-label="Navegação entre páginas"
                >
                  <button
                    type="button"
                    onClick={() => setPage((current) => current - 1)}
                    disabled={currentPage <= 1}
                    aria-label="Página anterior"
                  >
                    <ArrowLeftIcon aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    onClick={() => setPage((current) => current + 1)}
                    disabled={currentPage >= totalPages}
                    aria-label="Próxima página"
                  >
                    <ArrowRightIcon aria-hidden="true" />
                  </button>
                </div>
              </div>

              {filteredCards.length === 0 ? (
                <NoCards search={search} onClear={() => setSearch("")} />
              ) : (
                <div className="binder-book">
                  <span className="binder-spine" aria-hidden="true" />
                  <div className="binder-page">
                    {visibleCards.map((card, index) => (
                      <CardSlot
                        key={card.id}
                        card={card}
                        priority={index < ABOVE_FOLD_CARD_IMAGES}
                        onSelect={setSelectedCard}
                      />
                    ))}
                    {Array.from(
                      { length: CARDS_PER_PAGE - visibleCards.length },
                      (_, index) => (
                        <span
                          className="empty-pocket"
                          aria-hidden="true"
                          key={index}
                        />
                      ),
                    )}
                  </div>
                </div>
              )}
            </section>
          </>
        )}
      </main>
      {selectedCard && (
        <CardDetails
          card={selectedCard}
          tracked={
            binderState.status === "ready" && binderState.mode === "tracked"
          }
          pending={pendingCardId === selectedCard.id}
          onQuantityChange={changeQuantity}
          onClose={closeDetails}
        />
      )}
    </div>
  );
}

function CardSlot({
  card,
  priority,
  onSelect,
}: {
  card: PersonalCard;
  priority: boolean;
  onSelect: (card: PersonalCard) => void;
}) {
  return (
    <button
      type="button"
      className={`card-pocket ${card.quantity > 0 ? "is-owned" : "is-missing"}`}
      data-card-id={card.id}
      onClick={() => onSelect(card)}
      onPointerMove={updateCardMotion}
      onPointerLeave={(event) => resetCardMotion(event.currentTarget)}
      onPointerCancel={(event) => resetCardMotion(event.currentTarget)}
      onBlur={(event) => resetCardMotion(event.currentTarget)}
      aria-label={`Abrir detalhes da carta ${card.localNumber}, ${card.name}. ${card.quantity > 0 ? `${card.quantity} cópia${card.quantity === 1 ? "" : "s"}` : "Faltante"}`}
    >
      <span className="card-pocket-surface">
        <span className="card-number">{card.localNumber}</span>
        <span className="card-image">
          <CatalogImage
            src={card.imageSmallUrl}
            alt={`Imagem da carta ${card.name}`}
            sizes="(max-width: 480px) 27vw, (max-width: 900px) 180px, 220px"
            priority={priority}
          />
        </span>
        <span className="card-name">{card.name}</span>
        <span className="ownership-label">
          {card.quantity > 0 ? `Possuída · ${card.quantity}` : "Faltante"}
        </span>
        <span className="card-rarity">
          {card.rarity ?? "Raridade não informada"}
        </span>
      </span>
    </button>
  );
}

function updateCardMotion(event: ReactPointerEvent<HTMLButtonElement>) {
  if (event.pointerType !== "mouse") return;

  const card = event.currentTarget;
  const bounds = card.getBoundingClientRect();
  if (bounds.width === 0 || bounds.height === 0) return;

  const horizontalPosition = clampUnit(
    (event.clientX - bounds.left) / bounds.width,
  );
  const verticalPosition = clampUnit(
    (event.clientY - bounds.top) / bounds.height,
  );
  const horizontalTilt = (horizontalPosition - 0.5) * MAX_CARD_TILT_DEGREES * 2;
  const verticalTilt = (0.5 - verticalPosition) * MAX_CARD_TILT_DEGREES * 2;

  card.style.setProperty("--card-tilt-x", `${verticalTilt}deg`);
  card.style.setProperty("--card-tilt-y", `${horizontalTilt}deg`);
  card.style.setProperty("--card-sheen-x", `${horizontalPosition * 100}%`);
  card.style.setProperty("--card-sheen-y", `${verticalPosition * 100}%`);
}

function resetCardMotion(card: HTMLButtonElement) {
  CARD_MOTION_PROPERTIES.forEach((property) => {
    card.style.removeProperty(property);
  });
}

function clampUnit(value: number) {
  return Math.min(Math.max(value, 0), 1);
}

function CardDetails({
  card,
  tracked,
  pending,
  onQuantityChange,
  onClose,
}: {
  card: PersonalCard;
  tracked: boolean;
  pending: boolean;
  onQuantityChange: (card: PersonalCard, quantity: number) => void;
  onClose: () => void;
}) {
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLElement>(null);

  useEffect(() => {
    closeButtonRef.current?.focus();
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
      if (event.key !== "Tab") return;
      const controls = dialogRef.current?.querySelectorAll<HTMLElement>(
        "button:not(:disabled), a[href], input:not(:disabled)",
      );
      if (!controls?.length) return;
      const first = controls[0];
      const last = controls[controls.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  return (
    <div className="details-backdrop" role="presentation" onMouseDown={onClose}>
      <aside
        ref={dialogRef}
        className="card-details"
        role="dialog"
        aria-modal="true"
        aria-labelledby="card-details-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button
          ref={closeButtonRef}
          type="button"
          className="close-details"
          onClick={onClose}
          aria-label="Fechar detalhes"
        >
          <CloseIcon aria-hidden="true" />
        </button>
        <div className="details-image">
          <CatalogImage
            src={card.imageLargeUrl ?? card.imageSmallUrl}
            alt={`Imagem ampliada da carta ${card.name}`}
            sizes="(max-width: 640px) 70vw, 320px"
          />
        </div>
        <div className="details-copy">
          <p>Carta #{card.localNumber}</p>
          <h2 id="card-details-title">{card.name}</h2>
          <dl>
            <div>
              <dt>Número impresso</dt>
              <dd>{card.printedNumber}</dd>
            </div>
            {card.rarity && (
              <div>
                <dt>Raridade</dt>
                <dd>{card.rarity}</dd>
              </div>
            )}
            {card.artist && (
              <div>
                <dt>Ilustração</dt>
                <dd>{card.artist}</dd>
              </div>
            )}
          </dl>
          {tracked ? (
            <div className="quantity-editor">
              <p aria-live="polite">
                Quantidade possuída: <strong>{card.quantity}</strong>
              </p>
              <div>
                <button
                  type="button"
                  disabled={pending || card.quantity === 0}
                  onClick={() => onQuantityChange(card, card.quantity - 1)}
                  aria-label={`Diminuir quantidade de ${card.name}`}
                >
                  −
                </button>
                <button
                  type="button"
                  disabled={pending || card.quantity >= 999}
                  onClick={() => onQuantityChange(card, card.quantity + 1)}
                  aria-label={`Adicionar uma cópia de ${card.name}`}
                >
                  +
                </button>
                <button
                  type="button"
                  disabled={pending || card.quantity === 0}
                  onClick={() => onQuantityChange(card, 0)}
                >
                  Zerar
                </button>
              </div>
            </div>
          ) : (
            <p className="readonly-note">
              Visualização do catálogo. Entre e acompanhe a coleção para
              registrar cartas.
            </p>
          )}
        </div>
      </aside>
    </div>
  );
}

function NoCards({ search, onClear }: { search: string; onClear: () => void }) {
  return (
    <div className="binder-empty" role="status">
      <ImageIcon aria-hidden="true" />
      <h2>
        {search
          ? `Nenhuma carta corresponde a “${search}”.`
          : "Esta coleção ainda não possui cartas."}
      </h2>
      <p>
        {search
          ? "Tente outro nome ou número."
          : "As cartas aparecerão depois da próxima importação."}
      </p>
      {search && (
        <button type="button" onClick={onClear}>
          Limpar busca
        </button>
      )}
    </div>
  );
}

function BinderSkeleton() {
  return (
    <div
      className="binder-loading"
      role="status"
      aria-label="Carregando fichário"
    >
      <span className="skeleton-heading" />
      <span className="skeleton-search" />
      <div className="binder-book skeleton-book">
        <span className="binder-spine" aria-hidden="true" />
        <div className="binder-page skeleton-page">
          {Array.from({ length: 9 }, (_, index) => (
            <span key={index} />
          ))}
        </div>
      </div>
      <span className="sr-only">Carregando fichário.</span>
    </div>
  );
}

function BinderError({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <div className="binder-error" role="alert">
      <Link href="/" className="back-link">
        <ArrowLeftIcon aria-hidden="true" />
        Coleções
      </Link>
      <h1>Não foi possível abrir este fichário.</h1>
      <p>{message}</p>
      <button type="button" onClick={onRetry}>
        Tentar novamente
      </button>
    </div>
  );
}

function filterCards(
  cards: PersonalCard[],
  search: string,
  ownership: OwnershipFilter,
) {
  const normalizedSearch = search.trim().toLocaleLowerCase("pt-BR");
  return cards.filter(
    (card) =>
      (ownership === "all" ||
        (ownership === "owned" ? card.quantity > 0 : card.quantity === 0)) &&
      (!normalizedSearch ||
        [card.name, card.localNumber, card.printedNumber].some((value) =>
          value.toLocaleLowerCase("pt-BR").includes(normalizedSearch),
        )),
  );
}

async function fetchBinder(
  setId: string,
  signal: AbortSignal,
): Promise<BinderState> {
  const [collectionResponse, personalResult] = await Promise.all([
    fetchCollection(setId, signal),
    fetchPersonalCollection(setId, signal)
      .then((personal) => ({ status: "tracked" as const, personal }))
      .catch((error: unknown) => ({ status: "error" as const, error })),
  ]);
  if (personalResult.status === "tracked") {
    return {
      status: "ready",
      collection: collectionResponse.data,
      cards: personalResult.personal.data.cards,
      mode: "tracked",
      stats: personalResult.personal.data.collection,
    };
  }
  const { error } = personalResult;
  if (error instanceof Error && error.name === "AbortError") throw error;
  if (
    !(error instanceof CollectionRequestError) ||
    ![401, 404].includes(error.status)
  ) {
    throw error;
  }
  const cardsResponse = await fetchCards(setId, "", signal);
  return {
    status: "ready",
    collection: collectionResponse.data,
    cards: cardsResponse.data.map((card) => ({ ...card, quantity: 0 })),
    mode: error.status === 401 ? "guest" : "untracked",
    stats: null,
  };
}

function setBinderError(
  error: unknown,
  setBinderState: (state: BinderState) => void,
) {
  if (error instanceof Error && error.name !== "AbortError") {
    setBinderState({ status: "error", message: error.message });
  }
}
