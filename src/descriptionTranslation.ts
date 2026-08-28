import { brewApi } from "./brewApi";
import { localizeDescription } from "./localDescription";
import type { PackageKind, TranslationResult } from "./types";

const CACHE_KEY = "brewdesk.apple-description-cache.v1";
const ENABLED_KEY = "brewdesk.apple-description-translation.enabled";
const CACHE_LIMIT = 1500;

type DescribedItem = {
  token: string;
  kind: PackageKind;
  description: string;
  localizedDescription?: string | null;
  descriptionSource?: "apple" | "curated" | "original";
};

let statusSnapshot: { value: TranslationResult; at: number } | null = null;

function readCache(): Record<string, string> {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(CACHE_KEY) ?? "{}") as unknown;
    return parsed && typeof parsed === "object" ? parsed as Record<string, string> : {};
  } catch {
    return {};
  }
}

function writeCache(cache: Record<string, string>) {
  const entries = Object.entries(cache).slice(-CACHE_LIMIT);
  window.localStorage.setItem(CACHE_KEY, JSON.stringify(Object.fromEntries(entries)));
}

export function getTranslationCacheStats() {
  const cache = readCache();
  const serialized = JSON.stringify(cache);
  return {
    entries: Object.keys(cache).length,
    bytes: new TextEncoder().encode(serialized).byteLength
  };
}

export async function getTranslationStatus(force = false): Promise<TranslationResult> {
  if (!force && statusSnapshot && Date.now() - statusSnapshot.at < 15_000) return statusSnapshot.value;
  const value = await brewApi.translationStatus();
  if (value.installed) window.localStorage.setItem(ENABLED_KEY, "true");
  statusSnapshot = { value, at: Date.now() };
  return value;
}

export function resetTranslationStatus() {
  statusSnapshot = null;
}

export async function addChineseDescriptions<T extends DescribedItem>(items: T[]): Promise<T[]> {
  if (!items.length) return items;
  const cache = readCache();
  const missing = Array.from(new Set(items.flatMap((item) => {
    const local = localizeDescription(item.description, item.kind, item.token);
    return !local.chinese && item.description && !cache[item.description] ? [item.description] : [];
  })));

  if (missing.length && window.localStorage.getItem(ENABLED_KEY) === "true") {
    try {
      const status = await getTranslationStatus();
      if (status.installed) {
        for (let index = 0; index < missing.length; index += 20) {
          const batch = missing.slice(index, index + 20);
          const result = await brewApi.translateDescriptions(batch);
          if (!result.installed || !result.translations || result.translations.length !== batch.length) break;
          batch.forEach((english, translationIndex) => {
            const chinese = result.translations?.[translationIndex]?.trim();
            if (chinese && chinese !== english) cache[english] = chinese;
          });
        }
        writeCache(cache);
      }
    } catch {
      // Translation is an enhancement. Search and package management must still work offline.
    }
  }

  return items.map((item) => {
    const local = localizeDescription(item.description, item.kind, item.token);
    if (local.chinese) {
      return { ...item, localizedDescription: local.chinese, descriptionSource: "curated" };
    }
    const translated = cache[item.description];
    return translated
      ? { ...item, localizedDescription: translated, descriptionSource: "apple" }
      : { ...item, localizedDescription: null, descriptionSource: "original" };
  });
}
