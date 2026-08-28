import type { PackageKind } from "./types";

export interface LocalDescription {
  chinese: string | null;
  english: string | null;
  isApproximate: boolean;
}

function containsChinese(value: string) {
  return /[\u3400-\u9fff]/u.test(value);
}

export function localizeDescription(
  description: string | null | undefined,
  _kind: PackageKind,
  _token?: string
): LocalDescription {
  const english = description?.trim() || "";
  if (!english) {
    return {
      chinese: "Homebrew 暂未提供用途说明",
      english: null,
      isApproximate: false
    };
  }

  if (containsChinese(english)) {
    return { chinese: english, english: null, isApproximate: false };
  }

  return {
    chinese: null,
    english,
    isApproximate: false
  };
}
