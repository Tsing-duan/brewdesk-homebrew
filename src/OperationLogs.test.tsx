import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { OperationLogs } from "./OperationLogs";
import type { OperationLog } from "./types";

describe("live logs", () => {
  it("reuses unchanged log rows on unrelated parent updates", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const container = document.createElement("div");
    const root = createRoot(container);
    let reads = 0;
    const logs: OperationLog[] = [{ stream: "stdout", at: 1, get line() { reads++; return "Existing log"; } }];
    await act(async () => root.render(<div data-tick={0}><OperationLogs logs={logs} /></div>));
    const originalReads = reads;
    const originalRow = container.querySelector("p");
    await act(async () => root.render(<div data-tick={1}><OperationLogs logs={logs} /></div>));
    expect(reads).toBe(originalReads);
    expect(container.querySelector("p")).toBe(originalRow);
    await act(async () => root.unmount());
  });

  it("lets readers stay on older output, then resume following explicitly", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const container = document.createElement("div");
    const root = createRoot(container);
    const logs: OperationLog[] = [{ stream: "stdout", at: 1, line: "one" }];
    await act(async () => root.render(<OperationLogs logs={logs} />));
    const viewport = container.querySelector<HTMLElement>('[role="log"]')!;
    Object.defineProperties(viewport, { scrollHeight: { value: 1000 }, clientHeight: { value: 100 } });
    await act(async () => { viewport.scrollTop = 100; viewport.dispatchEvent(new Event("scroll")); });
    await act(async () => root.render(<OperationLogs logs={[...logs, { stream: "stdout", at: 2, line: "two" }]} />));
    expect(viewport.scrollTop).toBe(100);
    expect(container.textContent).toContain("回到最新日志");
    await act(async () => container.querySelector("button")!.click());
    expect(viewport.scrollTop).toBe(1000);
    await act(async () => root.unmount());
  });
});
