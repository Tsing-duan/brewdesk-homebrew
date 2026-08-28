import { describe, expect, it } from "vitest";
import { hasMissingCaskAppSource } from "./operationRecovery";
import type { BrewOperation } from "./types";

function failedOperation(line: string, kind: "cask" | "formula" = "cask"): BrewOperation {
  return {
    id: "operation-1",
    action: "upgrade",
    target: { token: "wpsoffice-cn", kind },
    commandPreview: "/opt/homebrew/bin/brew upgrade --cask wpsoffice-cn",
    status: "failed",
    phase: "failed",
    phaseLabel: "操作失败",
    startedAt: 1,
    logs: [{ stream: "stderr", line, at: 2 }]
  };
}

describe("operation recovery", () => {
  it("recognizes a missing cask application source", () => {
    expect(hasMissingCaskAppSource(
      failedOperation("Error: wpsoffice-cn: It seems the App source '/Applications/wpsoffice.app' is not there.")
    )).toBe(true);
  });

  it("does not offer cask repair for an unrelated or formula failure", () => {
    expect(hasMissingCaskAppSource(failedOperation("curl: (6) Could not resolve host"))).toBe(false);
    expect(hasMissingCaskAppSource(
      failedOperation("Error: source '/Applications/demo.app' is not there.", "formula")
    )).toBe(false);
  });
});
