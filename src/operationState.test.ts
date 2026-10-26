import { describe, expect, it } from "vitest";
import { mergeOperationEvent, mergeOperationEvents, validTransferTotal } from "./operationState";
import { operationFailureGuidance } from "./operationRecovery";
import type { BrewOperation } from "./types";

const operation: BrewOperation = {
  id: "batch", action: "upgrade-all", commandPreview: "brew upgrade",
  status: "running", phase: "resolving", phaseLabel: "正在解析依赖", progress: 16,
  startedAt: 1, downloadedBytes: 3, totalBytes: 3, transferBytesPerSecond: 2,
  logs: []
};

describe("download state from real batch-update failures", () => {
  it("combines a large ordered burst once while preserving unrelated history", () => {
    const other = { ...operation, id: "unchanged" };
    const events = Array.from({ length: 1200 }, (_, index) => ({
      operationId: "batch", status: "running" as const,
      log: { stream: "stdout" as const, line: `line ${index}`, at: index }
    }));
    const merged = mergeOperationEvents([operation, other], [
      ...events,
      { operationId: "batch", status: "succeeded", resetTransfer: true, progress: 100, finishedAt: 1300 }
    ]);
    expect(merged[0].logs).toHaveLength(1000);
    expect(merged[0].logs[0].line).toBe("line 200");
    expect(merged[0].logs[999].line).toBe("line 1199");
    expect(merged[0].status).toBe("succeeded");
    expect(merged[0].totalBytes).toBeNull();
    expect(merged[1]).toBe(other);
    expect(operation.logs).toHaveLength(0);
  });

  it("clears the previous total and estimate when a file starts without a known length", () => {
    const [next] = mergeOperationEvent([operation], {
      operationId: "batch", status: "running", phase: "downloading",
      phaseLabel: "正在下载", resetTransfer: true, downloadedBytes: 392.5 * 1024 ** 2,
      totalBytes: null, transferBytesPerSecond: 1024 ** 2, progress: null
    });
    expect(next.totalBytes).toBeNull();
    expect(next.progress).toBeNull();
    expect(next.phase).toBe("downloading");
    expect(validTransferTotal(next)).toBeNull();
  });

  it("preserves measurements on unrelated log events but clears them at completion", () => {
    const [logged] = mergeOperationEvent([operation], {
      operationId: "batch", status: "running", totalBytes: null,
      log: { stream: "stdout", line: "next log", at: 2 }
    });
    expect(logged.totalBytes).toBe(3);
    const [finished] = mergeOperationEvent([logged], {
      operationId: "batch", status: "failed", resetTransfer: true
    });
    expect(finished.totalBytes).toBeNull();
    expect(finished.downloadedBytes).toBeNull();
    expect(finished.transferBytesPerSecond).toBeNull();
  });

  it("never renders the screenshot's 392.5 MB / 3 B as a valid fraction", () => {
    expect(validTransferTotal({ ...operation, downloadedBytes: 392.5 * 1024 ** 2 })).toBeNull();
    expect(validTransferTotal({ ...operation, downloadedBytes: 3, totalBytes: 10 })).toBe(10);
    expect(validTransferTotal({ ...operation, totalBytes: Infinity })).toBeNull();
  });

  it("explains an ongoing TLS failure and the partial result of a failed batch", () => {
    expect(operationFailureGuidance({ ...operation, logs: [
      { stream: "stderr", line: "curl: (35) LibreSSL SSL_connect: SSL_ERROR_SYSCALL", at: 2 }
    ] })).toContain("已有下载报错");
    expect(operationFailureGuidance({ ...operation, status: "failed" })).toContain("部分项目可能已更新");
    expect(operationFailureGuidance({ ...operation, logs: [
      { stream: "stderr", line: "sudo: a terminal is required to read the password", at: 2 }
    ] })).toContain("终端中的管理员认证");
  });
});
