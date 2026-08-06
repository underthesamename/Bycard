"use client";

import Image from "next/image";
import { useState } from "react";

import { ImageIcon } from "./icons";

type CatalogImageProps = Readonly<{
  src: string | null;
  alt: string;
  sizes: string;
  priority?: boolean;
}>;

export function CatalogImage({
  src,
  alt,
  sizes,
  priority = false,
}: CatalogImageProps) {
  const [failed, setFailed] = useState(false);

  if (!src || failed) {
    return (
      <div
        className="image-fallback"
        role="img"
        aria-label={`${alt}. Imagem indisponível.`}
      >
        <ImageIcon aria-hidden="true" />
        <span>Imagem indisponível</span>
      </div>
    );
  }

  return (
    <Image
      src={src}
      alt={alt}
      fill
      sizes={sizes}
      priority={priority}
      onError={() => setFailed(true)}
    />
  );
}
