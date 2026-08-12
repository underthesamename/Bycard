"use client";

import { useEffect, useRef } from "react";

import { CatalogImage } from "@/features/catalog/catalog-image";

type HolographicCardProps = Readonly<{ isCollected: boolean }>;

const FEATURED_CARD_IMAGE =
  "https://assets.tcgdex.net/en/sv/sv08.5/162/high.webp";
const POCKET_POSITIONS = Array.from({ length: 9 }, (_, index) => index);
const BINDER_RING_POSITIONS = Array.from({ length: 4 }, (_, index) => index);
const MOTION_PROPERTIES = [
  "--card-rotate-x",
  "--card-rotate-y",
  "--binder-rotate-x",
  "--binder-rotate-y",
  "--foil-x",
  "--foil-y",
] as const;

function resetSceneMotion(scene: HTMLDivElement) {
  MOTION_PROPERTIES.forEach((property) => {
    scene.style.removeProperty(property);
  });
}

function updateSceneMotion(
  scene: HTMLDivElement,
  horizontalPosition: number,
  verticalPosition: number,
) {
  scene.style.setProperty(
    "--card-rotate-x",
    `${(0.5 - verticalPosition) * 14}deg`,
  );
  scene.style.setProperty(
    "--card-rotate-y",
    `${(horizontalPosition - 0.5) * 18}deg`,
  );
  scene.style.setProperty(
    "--binder-rotate-x",
    `${(0.5 - verticalPosition) * 4}deg`,
  );
  scene.style.setProperty(
    "--binder-rotate-y",
    `${(horizontalPosition - 0.5) * 6}deg`,
  );
  scene.style.setProperty("--foil-x", `${horizontalPosition * 100}%`);
  scene.style.setProperty("--foil-y", `${verticalPosition * 100}%`);
}

export function HolographicCard({ isCollected }: HolographicCardProps) {
  const sceneRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const scene = sceneRef.current;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

    if (!scene) {
      return;
    }

    const activeScene = scene;

    if (isCollected || reducedMotion.matches) {
      resetSceneMotion(activeScene);
      return;
    }

    let animationFrame = 0;

    function followPointer(event: PointerEvent) {
      if (event.pointerType === "touch") {
        return;
      }

      window.cancelAnimationFrame(animationFrame);
      animationFrame = window.requestAnimationFrame(() => {
        updateSceneMotion(
          activeScene,
          event.clientX / window.innerWidth,
          event.clientY / window.innerHeight,
        );
      });
    }

    function resetWhenLeavingWindow(event: PointerEvent) {
      if (event.relatedTarget === null) {
        resetSceneMotion(activeScene);
      }
    }

    function resetOnBlur() {
      resetSceneMotion(activeScene);
    }

    window.addEventListener("pointermove", followPointer, { passive: true });
    window.addEventListener("pointerout", resetWhenLeavingWindow, {
      passive: true,
    });
    window.addEventListener("blur", resetOnBlur);

    return () => {
      window.cancelAnimationFrame(animationFrame);
      window.removeEventListener("pointermove", followPointer);
      window.removeEventListener("pointerout", resetWhenLeavingWindow);
      window.removeEventListener("blur", resetOnBlur);
    };
  }, [isCollected]);

  return (
    <div
      ref={sceneRef}
      className={`auth-card-scene${isCollected ? " is-collected" : ""}`}
      aria-hidden="true"
    >
      <div className="auth-binder">
        <div className="auth-binder-cover" />
        <div className="auth-binder-pages">
          {(["left", "right"] as const).map((side) => (
            <div
              className={`auth-binder-page auth-binder-page--${side}`}
              key={side}
            >
              <div className="auth-binder-pocket-grid">
                {POCKET_POSITIONS.map((position) => (
                  <span className="auth-binder-pocket" key={position} />
                ))}
              </div>
            </div>
          ))}
        </div>
        <div className="auth-card-carriage">
          <div className="auth-card-float">
            <div className="auth-holographic-card">
              <CatalogImage
                src={FEATURED_CARD_IMAGE}
                alt="Carta Roaring Moon ex da coleção Evoluções Prismáticas"
                sizes="(max-width: 768px) 10rem, 17rem"
                priority
              />
              <span className="auth-card-foil" />
            </div>
          </div>
        </div>
        <div className="auth-target-sleeve" />
        <div className="auth-binder-spine">
          {BINDER_RING_POSITIONS.map((position) => (
            <span key={position} />
          ))}
        </div>
      </div>
    </div>
  );
}
