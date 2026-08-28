import { describe, expect, it } from "vitest";
import { brewApi } from "./brewApi";

describe("brewApi preview helpers", () => {
  it("builds cask install preview with args in the right order", () => {
    const command = brewApi.previewCommand("install", { token: "visual-studio-code", kind: "cask" });
    expect(command).toBe("/opt/homebrew/bin/brew install --cask visual-studio-code");
  });

  it("resolves common Chinese software names in preview mode", async () => {
    const response = await brewApi.searchPackages("微信", "both");
    expect(response.source).toBe("alias");
    expect(response.resolvedQuery).toBe("wechat");
    expect(response.items[0]?.token).toBe("wechat");
  });

  it("returns related Android tools for a broad Chinese query", async () => {
    const response = await brewApi.searchPackages("安卓", "both");
    expect(response.source).toBe("alias");
    expect(response.resolvedQuery).toBe("android");
    expect(response.items.map((item) => item.token)).toEqual(
      expect.arrayContaining(["android-studio", "android-platform-tools", "android-commandlinetools"])
    );
  });

  it("prioritizes platform tools for an Android debugging query", async () => {
    const response = await brewApi.searchPackages("安卓调试工具", "both");
    expect(response.items[0]?.token).toBe("android-platform-tools");
  });

  it("builds formula upgrade preview without cask flag", () => {
    const command = brewApi.previewCommand("upgrade", { token: "node", kind: "formula" });
    expect(command).toBe("/opt/homebrew/bin/brew upgrade node");
  });

  it("builds an explicit forced cask reinstall for repair", () => {
    const command = brewApi.previewCommand("repair", { token: "wpsoffice-cn", kind: "cask" });
    expect(command).toBe("/opt/homebrew/bin/brew reinstall --cask --force wpsoffice-cn");
  });

  it("keeps mirror selection session-scoped in preview mode", async () => {
    const configuration = await brewApi.setNetworkMode("mirror");
    expect(configuration.mode).toBe("mirror");
    expect(configuration.effectiveMode).toBe("mirror");
    expect(configuration.message).toContain("清华镜像");
  });

  it("can remove a queued preview operation safely", async () => {
    const operation = await brewApi.runBrewAction("doctor");
    await brewApi.cancelOperation(operation.id);
    const cancelled = await brewApi.operationEvents(operation.id);
    expect(cancelled?.status).toBe("cancelled");
    expect(cancelled?.phase).toBe("cancelled");
  });
});
