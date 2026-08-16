"use client";

import Image from "next/image";
import Link from "next/link";
import { useEffect, useState } from "react";

import { fetchCurrentSession, type AuthUser } from "@/features/auth/auth-api";

export function BrandHeader({ initialUser }: { initialUser?: AuthUser }) {
  const [user, setUser] = useState<AuthUser | null>(initialUser ?? null);

  useEffect(() => {
    if (initialUser) return;
    const controller = new AbortController();
    fetchCurrentSession(controller.signal)
      .then(({ user: currentUser }) => setUser(currentUser))
      .catch(() => setUser(null));
    return () => controller.abort();
  }, [initialUser]);

  return (
    <header className="site-header">
      <div className="header-inner">
        <Link className="brand" href="/" aria-label="Bycard, ir para coleções">
          <span className="brand-logo" aria-hidden="true">
            <Image
              src="/brand/bycard-logo.png"
              alt=""
              fill
              sizes="(max-width: 30rem) 6.5rem, 7.65rem"
              priority
            />
          </span>
        </Link>
        <nav className="primary-nav" aria-label="Navegação principal">
          <Link href="/">Coleções</Link>
          <Link href="/conta">Meu fichário</Link>
        </nav>
        <div className="header-context">
          <Link className="header-account" href={user ? "/conta" : "/entrar"}>
            {user ? `@${user.username}` : "Entrar"}
          </Link>
        </div>
      </div>
    </header>
  );
}
