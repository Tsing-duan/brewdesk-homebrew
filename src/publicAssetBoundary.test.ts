import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

describe("public asset boundary", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("loads the public application without a non-exported private icon", () => {
    expect(typeof App).toBe("function");
  });

  it("renders the approved BrewDesk icon instead of the letter placeholder", async () => {
    vi.useFakeTimers();
    (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
    const container = document.createElement("div");
    const root = createRoot(container);

    await act(async () => {
      root.render(createElement(App));
      await vi.runAllTimersAsync();
    });

    const brand = container.querySelector(".brand");
    const icon = brand?.querySelector("img");
    expect(icon, "the sidebar brand must render the approved icon asset").not.toBeNull();
    expect(icon?.getAttribute("src") ?? "").toContain("brewdesk-icon-liquid-glass.png");
    expect(brand?.querySelector(".brand-mark")?.textContent).toBe("");
    expect(brand?.querySelector(".alpha-badge")?.textContent).toBe("Alpha");

    await act(async () => root.unmount());
  });
});
