import type { OutdatedCacheV1, OutdatedPackage } from "./types";

export const OUTDATED_CACHE_KEY = "brewdesk.outdated.v1";

function isOutdatedPackage(value: unknown): value is OutdatedPackage {
  if (!value || typeof value !== "object") return false;
  const item = value as Partial<OutdatedPackage>;
  return (
    typeof item.token === "string" &&
    typeof item.name === "string" &&
    (item.kind === "formula" || item.kind === "cask") &&
    typeof item.currentVersion === "string" &&
    typeof item.latestVersion === "string"
  );
}

export function parseOutdatedCache(raw: string | null): OutdatedCacheV1 | null {
  if (!raw) return null;
  try {
    const value = JSON.parse(raw) as Partial<OutdatedCacheV1>;
    if (
      typeof value.brewPath !== "string" ||
      typeof value.fetchedAt !== "number" ||
      !Number.isFinite(value.fetchedAt) ||
      !Array.isArray(value.items) ||
      !value.items.every(isOutdatedPackage)
    ) {
      return null;
    }
    return value as OutdatedCacheV1;
  } catch {
    return null;
  }
}

export function readOutdatedCache(): OutdatedCacheV1 | null {
  if (typeof window === "undefined") return null;
  return parseOutdatedCache(window.localStorage.getItem(OUTDATED_CACHE_KEY));
}

export function writeOutdatedCache(cache: OutdatedCacheV1): void {
  window.localStorage.setItem(OUTDATED_CACHE_KEY, JSON.stringify(cache));
}

export function invalidateOutdatedCache(): void {
  window.localStorage.removeItem(OUTDATED_CACHE_KEY);
}
