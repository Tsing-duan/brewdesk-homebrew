import type { BrewOperation } from "./types";

export function hasMissingCaskAppSource(operation: BrewOperation) {
  return operation.status === "failed" &&
    operation.target?.kind === "cask" &&
    operation.logs.some((log) => /App source ['"].+\.app['"] is not there/i.test(log.line));
}
