import { describe, expect, it } from "vitest";
import { parseOutdatedCache } from "./outdatedCache";

describe("outdated cache", () => {
  const item = {
    token: "node",
    name: "Node",
    kind: "formula" as const,
    currentVersion: "22.0.0",
    latestVersion: "22.1.0"
  };

  it("loads a valid stale snapshot for immediate display", () => {
    const parsed = parseOutdatedCache(JSON.stringify({
      brewPath: "/opt/homebrew/bin/brew",
      fetchedAt: 1234,
      items: [item]
    }));
    expect(parsed?.items).toEqual([item]);
    expect(parsed?.fetchedAt).toBe(1234);
  });

  it("rejects corrupted or incompatible snapshots", () => {
    expect(parseOutdatedCache("not-json")).toBeNull();
    expect(parseOutdatedCache(JSON.stringify({ brewPath: "brew", fetchedAt: "now", items: [] }))).toBeNull();
    expect(parseOutdatedCache(JSON.stringify({ brewPath: "brew", fetchedAt: 1, items: [{}] }))).toBeNull();
  });
});
