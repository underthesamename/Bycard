import type { Metadata } from "next";
import type { ReactNode } from "react";

import { SiteFooter } from "@/features/site/site-footer";

import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/sora/600.css";
import "@fontsource/sora/700.css";

import "./globals.css";

export const metadata: Metadata = {
  title: "Bycard",
  description: "Guia não oficial para acompanhar suas coleções de cartas.",
};

type RootLayoutProps = Readonly<{
  children: ReactNode;
}>;

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html lang="pt-BR" data-scroll-behavior="smooth">
      <body>
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
