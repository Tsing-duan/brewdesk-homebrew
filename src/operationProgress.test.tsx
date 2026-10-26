import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { OperationProgress } from "./App";
import type { BrewOperation } from "./types";

const downloading: BrewOperation = {
  id: "screenshot", action: "upgrade-all", commandPreview: "brew upgrade",
  status: "running", phase: "downloading", phaseLabel: "正在下载", progress: null,
  startedAt: Date.now(), lastActivityAt: Date.now(), downloadedBytes: 392.5 * 1024 ** 2,
  totalBytes: 3, transferBytesPerSecond: 1024 ** 2, logs: []
};

describe("operation progress rendering", () => {
  it("renders unknown download length without an impossible denominator or invented percentage", () => {
    const html = renderToStaticMarkup(<OperationProgress operation={downloading} />);
    expect(html).toContain("392.5 MB");
    expect(html).not.toContain("/ 3 B");
    expect(html).toContain("正在传输");
    expect(html).toContain('aria-busy="true"');
    expect(html).not.toContain('aria-valuenow=');
  });

  it("does not mistake a queued task for a stalled network", () => {
    const html = renderToStaticMarkup(<OperationProgress operation={{
      ...downloading, status: "queued", phase: "preparing", startedAt: 1, lastActivityAt: 1
    }} />);
    expect(html).toContain("等待前面的任务完成");
    expect(html).not.toContain("可能在等待网络");
  });
});
