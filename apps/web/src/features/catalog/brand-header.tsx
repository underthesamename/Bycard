import Link from "next/link";

export function BrandHeader() {
  return (
    <header className="site-header">
      <div className="header-inner">
        <Link className="brand" href="/" aria-label="Bycard — ir para coleções">
          <span className="brand-word">Bycard</span>
          <span className="brand-spark" aria-hidden="true" />
        </Link>
        <div className="header-context">
          <span className="pokemon-context">Pokémon TCG</span>
          <span className="guide-label">Guia não oficial</span>
          <Link className="header-account" href="/conta">
            Meu fichário
          </Link>
        </div>
      </div>
    </header>
  );
}
