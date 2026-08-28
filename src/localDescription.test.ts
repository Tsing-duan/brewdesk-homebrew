import { describe, expect, it } from "vitest";
import { localizeDescription } from "./localDescription";

describe("localizeDescription", () => {
  it("does not ship a pre-generated translation for a previously curated package", () => {
    expect(localizeDescription("Free messaging and calling application", "cask", "wechat")).toEqual({
      chinese: null,
      english: "Free messaging and calling application",
      isApproximate: false
    });
  });

  it("keeps unfamiliar descriptions in English instead of guessing", () => {
    expect(localizeDescription("Fast SQL client for developers", "cask", "new-sql-app")).toEqual({
      chinese: null,
      english: "Fast SQL client for developers",
      isApproximate: false
    });
  });

  it("keeps an existing Chinese description without duplicating an English source", () => {
    expect(localizeDescription("中文音乐播放应用", "cask", "music-app")).toEqual({
      chinese: "中文音乐播放应用",
      english: null,
      isApproximate: false
    });
  });

  it("does not invent a purpose when Homebrew has no description", () => {
    expect(localizeDescription("", "formula", "unknown-tool")).toEqual({
      chinese: "Homebrew 暂未提供用途说明",
      english: null,
      isApproximate: false
    });
  });
});
