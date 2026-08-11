import { XLogo } from "@phosphor-icons/react/dist/ssr";

export function SiteFooter() {
  return (
    <footer className="site-footer">
      <a
        className="footer-credit"
        href="https://x.com/nineteenlines"
        target="_blank"
        rel="me noreferrer"
        aria-label="Bycard por @nineteenlines no X"
      >
        <span>By</span>
        <span className="footer-x-mark" aria-hidden="true">
          <XLogo size={14} weight="regular" />
        </span>
        <span className="footer-handle">@nineteenlines</span>
      </a>
      <p>
        Projeto independente, não comercial e sem associação com Nintendo,
        Creatures, Game Freak ou The Pokémon Company.
      </p>
    </footer>
  );
}
