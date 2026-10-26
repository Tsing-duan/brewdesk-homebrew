import type { BrewOperation, OperationEvent } from "./types";

export function mergeOperationEvent(operations: BrewOperation[], event: OperationEvent) {
  return mergeOperationEvents(operations, [event]);
}

export function mergeOperationEvents(operations: BrewOperation[], events: OperationEvent[]) {
  if (!events.length) return operations;
  const current = new Map(operations.map((operation) => [operation.id, operation]));
  const changed = new Map<string, BrewOperation>();
  const appended = new Map<string, BrewOperation["logs"]>();
  for (const event of events) {
    const operation = changed.get(event.operationId) ?? current.get(event.operationId);
    if (!operation) continue;
    changed.set(event.operationId, applyOperationEvent(operation, event));
    if (event.log) {
      const logs = appended.get(event.operationId) ?? [];
      logs.push(event.log);
      appended.set(event.operationId, logs);
    }
  }
  if (!changed.size) return operations;
  return operations.map((original) => {
    const updated = changed.get(original.id);
    if (!updated) return original;
    const logs = appended.get(original.id);
    return logs ? { ...updated, logs: [...original.logs, ...logs].slice(-1000) } : updated;
  });
}

function applyOperationEvent(operation: BrewOperation, event: OperationEvent): BrewOperation {
    const reset = event.resetTransfer === true;
    return {
      ...operation,
      status: event.status,
      phase: event.phase ?? operation.phase,
      phaseLabel: event.phaseLabel ?? operation.phaseLabel,
      progress: event.phase === "downloading" ? (event.progress ?? null) : (event.progress ?? operation.progress),
      currentItem: event.currentItem === undefined ? operation.currentItem : event.currentItem,
      networkIssue: event.networkIssue ?? operation.networkIssue,
      effectiveNetworkMode: event.effectiveNetworkMode ?? operation.effectiveNetworkMode,
      proxyUrl: event.effectiveNetworkMode != null ? (event.proxyUrl ?? null) : operation.proxyUrl,
      lastActivityAt: event.lastActivityAt ?? operation.lastActivityAt,
      transferBytesPerSecond: event.transferBytesPerSecond ?? (reset ? null : operation.transferBytesPerSecond),
      downloadedBytes: event.downloadedBytes ?? (reset ? null : operation.downloadedBytes),
      totalBytes: event.totalBytes ?? (reset ? null : operation.totalBytes),
      activityLabel: event.activityLabel ?? operation.activityLabel,
      exitCode: event.exitCode ?? operation.exitCode,
      finishedAt: event.finishedAt ?? operation.finishedAt,
      logs: operation.logs
    };
}

export function validTransferTotal(operation: BrewOperation) {
  const total = operation.totalBytes;
  const downloaded = operation.downloadedBytes;
  return total != null && Number.isFinite(total) && total > 0 &&
    (downloaded == null || (Number.isFinite(downloaded) && downloaded >= 0 && downloaded <= total))
    ? total : null;
}
