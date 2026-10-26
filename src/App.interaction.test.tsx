import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { brewApi } from "./brewApi";
import * as translation from "./descriptionTranslation";
import type { InstalledInventory, PackageSummary, SearchResponse } from "./types";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
const alpha: PackageSummary = {
  token: "alpha-tool", name: "Alpha Tool", kind: "formula",
  description: "Specialist quantum pickle processing", installed: true, outdated: false
};
const beta = { ...alpha, token: "beta-tool", name: "Beta Tool" };
const inventory: InstalledInventory = { homebrew: [alpha], other: [] };
const response = (item: PackageSummary): SearchResponse => ({ query: item.token, source: "brew", items: [item] });

let container: HTMLDivElement;
let root: Root;
beforeEach(() => {
  vi.restoreAllMocks();
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const storage = new Map<string, string>();
  Object.defineProperty(window, "localStorage", { configurable: true, value: {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
    removeItem: (key: string) => storage.delete(key),
    clear: () => storage.clear()
  }});
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  vi.spyOn(brewApi, "availability").mockResolvedValue({ status: "ready", brewPath: "/opt/homebrew/bin/brew", arch: "arm64", recommendedPath: "/opt/homebrew/bin/brew", commandLineToolsAvailable: true, message: "Ready" });
  vi.spyOn(brewApi, "networkConfiguration").mockResolvedValue({ mode: "official", effectiveMode: "direct", message: "Ready" });
  vi.spyOn(brewApi, "homebrewUpdateStatus").mockResolvedValue({ currentVersion: "1", latestVersion: "1", updateAvailable: false, checked: true, message: "Ready" });
  vi.spyOn(brewApi, "getMockOperations").mockReturnValue([]);
  vi.spyOn(brewApi, "listenOperations").mockResolvedValue(() => undefined);
  vi.spyOn(brewApi, "installedInventory").mockResolvedValue(inventory);
  vi.spyOn(brewApi, "outdatedPackages").mockResolvedValue([]);
  vi.spyOn(translation, "addChineseDescriptions").mockImplementation(async (items) => items);
});
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  container?.remove();
});

async function mount() { await act(async () => root.render(<App />)); }
async function click(label: string) {
  const button = [...container.querySelectorAll("button")].find((button) => button.textContent?.trim() === label || button.getAttribute("aria-label") === label);
  expect(button, label).toBeTruthy();
  await act(async () => button!.click());
}
async function search(query: string) {
  const input = container.querySelector<HTMLInputElement>('input[aria-label="搜索 Homebrew 包"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, query);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => container.querySelector("form.searchbar")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
}

describe("responsive asynchronous package flows", () => {
  it("shows search results before slow optional translation finishes", async () => {
    const translated = deferred<PackageSummary[]>();
    vi.mocked(translation.addChineseDescriptions).mockReturnValue(translated.promise);
    vi.spyOn(brewApi, "searchPackages").mockResolvedValue(response(alpha));
    await mount();
    await search("alpha");
    expect(container.querySelector('[aria-label="Alpha Tool，Homebrew formula"]')).not.toBeNull();
    expect(container.textContent).not.toContain("正在查询 Homebrew");
    await act(async () => translated.resolve([{ ...alpha, localizedDescription: "背景翻译完成" }]));
    expect(container.textContent).toContain("背景翻译完成");
  });

  it("keeps the newest search when older requests resolve later", async () => {
    const first = deferred<SearchResponse>();
    const second = deferred<SearchResponse>();
    vi.spyOn(brewApi, "searchPackages").mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    await mount();
    await search("alpha");
    await search("beta");
    await act(async () => second.resolve(response(beta)));
    await act(async () => first.resolve(response(alpha)));
    expect(container.querySelector('[aria-label="Beta Tool，Homebrew formula"]')).not.toBeNull();
    expect(container.querySelector('[aria-label="Alpha Tool，Homebrew formula"]')).toBeNull();
  });

  it("keeps the inventory usable while refreshing and restores its scroll on return", async () => {
    const translated = deferred<PackageSummary[]>();
    vi.mocked(translation.addChineseDescriptions).mockReturnValue(translated.promise);
    await mount();
    await click("已安装");
    const row = container.querySelector('[aria-label="Alpha Tool，Homebrew formula"]');
    expect(row).not.toBeNull();
    const refresh = deferred<InstalledInventory>();
    vi.mocked(brewApi.installedInventory).mockReturnValueOnce(refresh.promise);
    await click("刷新");
    expect(container.querySelector('[aria-label="Alpha Tool，Homebrew formula"]')).toBe(row);
    expect(container.textContent).toContain("正在后台刷新");
    const list = container.querySelector<HTMLElement>(".installed-inventory")!;
    await act(async () => { list.scrollTop = 350; list.dispatchEvent(new Event("scroll", { bubbles: false })); });
    await click("可更新");
    await click("已安装");
    expect(container.querySelector<HTMLElement>(".installed-inventory")!.scrollTop).toBe(350);
    expect(brewApi.installedInventory).toHaveBeenCalledTimes(2);
    await act(async () => refresh.resolve(inventory));
    await act(async () => translated.resolve(inventory.homebrew));
  });

  it("keeps independent scroll positions across populated searches, even before a scroll event arrives", async () => {
    vi.spyOn(brewApi, "searchPackages").mockResolvedValue(response(beta));
    await mount();
    await click("已安装");
    // Navigation must capture the current offset even if WebKit has not delivered its scroll event yet.
    container.querySelector<HTMLElement>('.installed-inventory')!.scrollTop = 350;
    await search("beta");
    const results = container.querySelector<HTMLElement>('[data-scroll-view="search"]')!;
    results.scrollTop = 75;
    await click("已安装");
    expect(container.querySelector<HTMLElement>('.installed-inventory')!.scrollTop).toBe(350);
    await click("搜索");
    expect(container.querySelector<HTMLElement>('[data-scroll-view="search"]')!.scrollTop).toBe(75);
  });

  it("does not let an old translation replace newer search results", async () => {
    const oldTranslation = deferred<PackageSummary[]>();
    vi.mocked(translation.addChineseDescriptions).mockImplementation((items) => items[0]?.token === alpha.token ? oldTranslation.promise : Promise.resolve(items));
    vi.spyOn(brewApi, "searchPackages").mockResolvedValueOnce(response(alpha)).mockResolvedValueOnce(response(beta));
    await mount();
    await search("alpha");
    await search("beta");
    await act(async () => oldTranslation.resolve([{ ...alpha, localizedDescription: "过期翻译" }]));
    expect(container.querySelector('[aria-label="Beta Tool，Homebrew formula"]')).not.toBeNull();
    expect(container.textContent).not.toContain("过期翻译");
  });
});
