import type {
  BrewAction,
  BrewAvailability,
  BrewEnvironment,
  HomebrewUpdateStatus,
  HomebrewInstallLaunch,
  InstalledInventory,
  BrewOperation,
  BrewTarget,
  NetworkConfiguration,
  NetworkMode,
  NetworkTestResult,
  OperationEvent,
  OutdatedPackage,
  PackageDetail,
  PackageKind,
  PackageSummary,
  SearchResponse,
  SearchKind,
  TranslationResult
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

type OperationListener = (event: OperationEvent) => void;
const mockOperationListeners = new Set<OperationListener>();
let mockNetworkConfiguration: NetworkConfiguration = {
  mode: "auto",
  effectiveMode: "system-proxy",
  proxyUrl: "http://127.0.0.1:7892",
  message: "自动模式：浏览器预览使用已检测到的 macOS 系统代理"
};

const mockPackages: PackageSummary[] = [
  {
    token: "wechat",
    name: "WeChat for Mac",
    localizedName: "微信 Mac 版",
    kind: "cask",
    description: "Free messaging and calling application",
    version: "latest",
    homepage: "https://mac.weixin.qq.com/",
    installed: false,
    outdated: false
  },
  {
    token: "google-chrome",
    name: "Google Chrome",
    localizedName: "谷歌浏览器",
    kind: "cask",
    description: "Web browser",
    version: "latest",
    homepage: "https://www.google.com/chrome/",
    installed: false,
    outdated: false
  },
  {
    token: "visual-studio-code",
    name: "Visual Studio Code",
    kind: "cask",
    description: "Open-source code editor",
    installed: false,
    outdated: false
  },
  {
    token: "android-studio",
    name: "Android Studio",
    kind: "cask",
    description: "Tools for building Android applications",
    installed: false,
    outdated: false
  },
  {
    token: "android-platform-tools",
    name: "Android SDK Platform-Tools",
    kind: "cask",
    description: "Android SDK component including adb and fastboot",
    installed: false,
    outdated: false
  },
  {
    token: "android-commandlinetools",
    name: "Android SDK Command-line Tools",
    kind: "cask",
    description: "Command-line tools for building and debugging Android apps",
    installed: false,
    outdated: false
  },
  {
    token: "iterm2",
    name: "iTerm2",
    kind: "cask",
    description: "Terminal emulator as alternative to Apple's Terminal app",
    installed: true,
    outdated: true
  },
  {
    token: "wget",
    name: "Wget",
    kind: "formula",
    description: "Internet file retriever",
    installed: true,
    outdated: false
  },
  {
    token: "node",
    name: "Node",
    kind: "formula",
    description: "Platform built on V8 to build network applications",
    installed: true,
    outdated: true
  },
  {
    token: "raycast",
    name: "Raycast",
    kind: "cask",
    description: "Control your tools with a few keystrokes",
    installed: false,
    outdated: false
  }
];

const mockOperations: BrewOperation[] = [
  {
    id: "mock-update-1",
    action: "update",
    commandPreview: "/opt/homebrew/bin/brew update",
    status: "succeeded",
    phase: "completed",
    phaseLabel: "更新完成",
    progress: 100,
    startedAt: Date.now() - 1000 * 60 * 12,
    finishedAt: Date.now() - 1000 * 60 * 10,
    exitCode: 0,
    logs: [
      { stream: "stdout", line: "Already up-to-date.", at: Date.now() - 1000 * 60 * 11 },
      { stream: "stdout", line: "Updated 2 taps.", at: Date.now() - 1000 * 60 * 10 }
    ]
  }
];
const mockOperationQueue: BrewOperation[] = [];
let mockWorkerRunning = false;
let mockTranslationInstalled = false;

function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

function delay(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function previewDelay(fallback: number) {
  const configured = Number(new URLSearchParams(window.location.search).get("mockDelay"));
  return Number.isFinite(configured) && configured >= 0 ? configured : fallback;
}

function kindLabel(kind: PackageKind) {
  return kind === "cask" ? "cask" : "formula";
}

function mockCommand(action: BrewAction, target?: BrewTarget) {
  const prefix = "/opt/homebrew/bin/brew";
  if (action === "update") return `${prefix} update`;
  if (action === "doctor") return `${prefix} doctor`;
  if (action === "upgrade-all") return `${prefix} upgrade`;
  if (!target) return prefix;
  if (action === "repair") return `${prefix} reinstall --cask --force ${target.token}`;
  const caskFlag = target.kind === "cask" ? " --cask" : "";
  return `${prefix} ${action}${caskFlag} ${target.token}`;
}

function emitMockOperation(event: OperationEvent) {
  const operation = mockOperations.find((item) => item.id === event.operationId);
  if (operation) {
    operation.status = event.status;
    if (event.phase) operation.phase = event.phase;
    if (event.phaseLabel) operation.phaseLabel = event.phaseLabel;
    if (event.progress != null) operation.progress = event.progress;
    if (event.currentItem !== undefined) operation.currentItem = event.currentItem;
    if (event.networkIssue != null) operation.networkIssue = event.networkIssue;
    if (event.effectiveNetworkMode != null) {
      operation.effectiveNetworkMode = event.effectiveNetworkMode;
      operation.proxyUrl = event.proxyUrl ?? null;
    }
    if (event.lastActivityAt != null) operation.lastActivityAt = event.lastActivityAt;
    if (event.transferBytesPerSecond !== undefined) operation.transferBytesPerSecond = event.transferBytesPerSecond;
    if (event.downloadedBytes !== undefined) operation.downloadedBytes = event.downloadedBytes;
    if (event.activityLabel !== undefined) operation.activityLabel = event.activityLabel;
    if (event.finishedAt != null) operation.finishedAt = event.finishedAt;
    if (event.exitCode != null) operation.exitCode = event.exitCode;
  }
  mockOperationListeners.forEach((listener) => listener(event));
}

function runNextMockOperation() {
  if (mockWorkerRunning) return;
  const operation = mockOperationQueue.shift();
  if (!operation) return;
  mockWorkerRunning = true;
  emitMockOperation({
    operationId: operation.id,
    status: "running",
    phase: "preparing",
    phaseLabel: "正在准备",
    progress: 6
  });

  const stages: Array<{ delay: number; event: OperationEvent }> = [
    {
      delay: 800,
      event: {
        operationId: operation.id,
        status: "running",
        phase: "resolving",
        phaseLabel: "正在解析依赖",
        progress: 18,
        log: { stream: "stdout", line: "Resolving dependencies...", at: Date.now() }
      }
    },
    {
      delay: 2500,
      event: {
        operationId: operation.id,
        status: "running",
        phase: "downloading",
        phaseLabel: "正在下载",
        progress: 54,
        currentItem: operation.target?.token ?? "Homebrew 数据",
        lastActivityAt: Date.now(),
        transferBytesPerSecond: 1_850_000,
        downloadedBytes: 42_000_000,
        activityLabel: "持续收到下载数据",
        log: { stream: "stdout", line: "==> Downloading package", at: Date.now() }
      }
    },
    {
      delay: 5000,
      event: {
        operationId: operation.id,
        status: "running",
        phase: "installing",
        phaseLabel: "正在安装",
        progress: 82,
        transferBytesPerSecond: null,
        downloadedBytes: null,
        activityLabel: "正在安装软件包",
        log: { stream: "stdout", line: "==> Installing package", at: Date.now() }
      }
    },
    {
      delay: 8000,
      event: {
        operationId: operation.id,
        status: "succeeded",
        phase: "completed",
        phaseLabel: "操作完成",
        progress: 100,
        currentItem: null,
        transferBytesPerSecond: null,
        downloadedBytes: null,
        activityLabel: "操作已完成",
        finishedAt: Date.now() + 8000,
        exitCode: 0
      }
    }
  ];
  stages.forEach(({ delay: wait, event }, index) =>
    window.setTimeout(() => {
      if (operation.status === "cancelled") {
        if (index === stages.length - 1) {
          mockWorkerRunning = false;
          runNextMockOperation();
        }
        return;
      }
      const emittedAt = Date.now();
      emitMockOperation({
        ...event,
        lastActivityAt: emittedAt,
        log: event.log ? { ...event.log, at: emittedAt } : event.log
      });
      if (index === stages.length - 1) {
        mockWorkerRunning = false;
        runNextMockOperation();
      }
    }, wait)
  );
}

function mockDetail(summary: PackageSummary): PackageDetail {
  const detectedApplications = summary.token === "google-chrome"
    ? [{
        name: "Google Chrome",
        path: "/Applications/Google Chrome.app",
        version: "150.0.7871.115",
        bundleId: "com.google.Chrome",
        source: "其他渠道或来源未知",
        brewManaged: false
      }]
    : [];
  return {
    token: summary.token,
    name: summary.name,
    kind: summary.kind,
    description: summary.description,
    homepage: `https://formulae.brew.sh/${summary.kind}/${summary.token}`,
    version: summary.kind === "cask" ? "latest" : "1.0.0",
    installedVersions: summary.installed ? [summary.kind === "cask" ? "latest" : "1.0.0"] : [],
    installed: summary.installed,
    dependencies: summary.kind === "formula" ? ["openssl@3", "ca-certificates"] : [],
    commandName: summary.token,
    detectedApplications,
    installLocation: summary.installed
      ? summary.kind === "cask"
        ? `/Applications/${summary.name}.app`
        : `/opt/homebrew/Cellar/${summary.token}/1.0.0`
      : null,
    installedSizeBytes: summary.installed ? (summary.kind === "cask" ? 286 * 1024 ** 2 : 42 * 1024 ** 2) : null
  };
}

export const brewApi = {
  isTauriRuntime,

  homebrewInstallCommand:
    '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',

  async availability(): Promise<BrewAvailability> {
    if (isTauriRuntime()) return tauriInvoke("brew_availability");
    const missing = new URLSearchParams(window.location.search).get("brew") === "missing";
    return {
      status: missing ? "missing" : "ready",
      brewPath: missing ? null : "/opt/homebrew/bin/brew",
      arch: "arm64",
      recommendedPath: "/opt/homebrew/bin/brew",
      commandLineToolsAvailable: true,
      message: missing ? "这台 Mac 尚未安装 Homebrew" : "浏览器预览模式：Homebrew 已就绪"
    };
  },

  async startHomebrewInstall(networkMode: NetworkMode = "auto"): Promise<HomebrewInstallLaunch> {
    if (isTauriRuntime()) return tauriInvoke("start_homebrew_install", { networkMode });
    return {
      started: true,
      commandPreview: networkMode === "mirror" ? "清华大学开源软件镜像站安装脚本" : this.homebrewInstallCommand,
      networkMode: networkMode === "mirror" ? "mirror" : mockNetworkConfiguration.effectiveMode
    };
  },

  async networkConfiguration(): Promise<NetworkConfiguration> {
    if (isTauriRuntime()) return tauriInvoke("network_configuration");
    return mockNetworkConfiguration;
  },

  async setNetworkMode(mode: NetworkMode): Promise<NetworkConfiguration> {
    if (isTauriRuntime()) return tauriInvoke("set_network_mode", { mode });
    mockNetworkConfiguration = {
      mode,
      effectiveMode: mode === "mirror" ? "mirror" : mode === "official" ? "direct" : "system-proxy",
      proxyUrl: mode === "auto" ? "http://127.0.0.1:7892" : null,
      message:
        mode === "mirror"
          ? "备用线路：本次 Homebrew 操作使用清华镜像"
          : mode === "official"
            ? "官方直连：不使用系统代理或镜像"
            : "自动模式：浏览器预览使用已检测到的 macOS 系统代理"
    };
    return mockNetworkConfiguration;
  },

  async testNetworkConnection(): Promise<NetworkTestResult> {
    if (isTauriRuntime()) return tauriInvoke("test_network_connection");
    await delay(previewDelay(600));
    return {
      reachable: true,
      effectiveMode: mockNetworkConfiguration.effectiveMode,
      proxyUrl: mockNetworkConfiguration.proxyUrl,
      latencyMs: 128,
      message: `${mockNetworkConfiguration.effectiveMode === "system-proxy" ? "系统代理" : mockNetworkConfiguration.effectiveMode === "mirror" ? "备用线路" : "官方直连"}可达 · 128 ms`
    };
  },

  async openHomebrewPkg(): Promise<void> {
    if (isTauriRuntime()) return tauriInvoke("open_homebrew_pkg");
    window.open("https://github.com/Homebrew/brew/releases/latest", "_blank", "noopener,noreferrer");
  },

  async openExternalUrl(url: string): Promise<void> {
    if (isTauriRuntime()) return tauriInvoke("open_external_url", { url });
    window.open(url, "_blank", "noopener,noreferrer");
  },

  async translationStatus(): Promise<TranslationResult> {
    if (isTauriRuntime()) return tauriInvoke("translation_status");
    return {
      available: true,
      installed: mockTranslationInstalled,
      searchInstalled: mockTranslationInstalled,
      status: mockTranslationInstalled ? "installed" : "download-required",
      message: mockTranslationInstalled ? "Apple 英译中简介与中译英搜索语言能力均已安装" : "需要准备 Apple 本机中英文语言能力"
    };
  },

  async prepareTranslation(): Promise<void> {
    if (isTauriRuntime()) return tauriInvoke("prepare_translation");
    mockTranslationInstalled = true;
  },

  async translateDescriptions(texts: string[]): Promise<TranslationResult> {
    if (isTauriRuntime()) return tauriInvoke("translate_descriptions", { texts });
    return {
      available: true,
      installed: mockTranslationInstalled,
      searchInstalled: mockTranslationInstalled,
      status: mockTranslationInstalled ? "installed" : "download-required",
      message: mockTranslationInstalled ? "浏览器预览翻译" : "需要先下载 Apple 语言包",
      translations: mockTranslationInstalled ? texts.map((text) => `中文预览：${text}`) : null
    };
  },

  async environment(): Promise<BrewEnvironment> {
    if (isTauriRuntime()) return tauriInvoke("brew_environment");
    return {
      brewPath: "/opt/homebrew/bin/brew",
      version: "Homebrew 6.0.9",
      prefix: "/opt/homebrew",
      arch: "arm64",
      ok: true,
      message: "浏览器预览模式：使用 mock 数据"
    };
  },

  async homebrewUpdateStatus(): Promise<HomebrewUpdateStatus> {
    if (isTauriRuntime()) return tauriInvoke("homebrew_update_status");
    await delay(previewDelay(220));
    return {
      currentVersion: "6.0.9",
      latestVersion: "6.0.12",
      updateAvailable: true,
      checked: true,
      message: "Homebrew 6.0.12 可更新"
    };
  },

  async setBrewPath(path: string): Promise<void> {
    if (isTauriRuntime()) return tauriInvoke("set_brew_path", { path });
    window.localStorage.setItem("brewdesk.brewPath", path);
  },

  async searchPackages(query: string, kind: SearchKind): Promise<SearchResponse> {
    if (isTauriRuntime()) return tauriInvoke("search_packages", { query, kind });
    await delay(previewDelay(180));
    const normalized = query.trim().toLowerCase();
    const aliases: Record<string, { resolved: string; tokens: string[] }> = {
      微信: { resolved: "wechat", tokens: ["wechat"] },
      谷歌浏览器: { resolved: "google-chrome", tokens: ["google-chrome"] },
      浏览器: { resolved: "google-chrome", tokens: ["google-chrome"] },
      代码编辑器: { resolved: "visual-studio-code", tokens: ["visual-studio-code"] },
      终端: { resolved: "iterm2", tokens: ["iterm2"] },
      视频播放器: { resolved: "iina", tokens: ["iina", "vlc"] },
      截图: { resolved: "shottr", tokens: ["shottr"] },
      解压: { resolved: "keka", tokens: ["keka", "the-unarchiver"] },
      解压软件: { resolved: "keka", tokens: ["keka", "the-unarchiver"] },
      安卓: { resolved: "android", tokens: ["android-studio", "android-platform-tools", "android-commandlinetools"] },
      安卓工具: { resolved: "android", tokens: ["android-platform-tools", "android-commandlinetools", "android-studio"] },
      安卓调试工具: { resolved: "android", tokens: ["android-platform-tools", "android-commandlinetools", "android-studio"] }
    };
    const resolved = aliases[query.trim()];
    const items = mockPackages.filter((pkg) => {
      const matchesKind = kind === "both" || pkg.kind === kind;
      const matchesText =
        normalized === "code" ||
        resolved?.tokens.includes(pkg.token) ||
        [pkg.token, pkg.name, pkg.localizedName, pkg.description].join(" ").toLowerCase().includes(normalized);
      return matchesKind && matchesText;
    }).sort((left, right) => {
      if (!resolved) return 0;
      const leftRank = resolved.tokens.indexOf(left.token);
      const rightRank = resolved.tokens.indexOf(right.token);
      return (leftRank < 0 ? Number.MAX_SAFE_INTEGER : leftRank) - (rightRank < 0 ? Number.MAX_SAFE_INTEGER : rightRank);
    });
    return {
      query,
      resolvedQuery: resolved?.resolved,
      source: resolved ? "alias" : /[\u3400-\u9fff]/u.test(query) ? "catalog" : "brew",
      items
    };
  },

  async packageInfo(token: string, kind: PackageKind): Promise<PackageDetail> {
    if (isTauriRuntime()) return tauriInvoke("package_info", { token, kind });
    await delay(previewDelay(120));
    const summary = mockPackages.find((pkg) => pkg.token === token && pkg.kind === kind);
    if (!summary) throw new Error("没有找到包详情");
    return mockDetail(summary);
  },

  async installedPackages(): Promise<PackageSummary[]> {
    if (isTauriRuntime()) return tauriInvoke("installed_packages");
    return mockPackages.filter((pkg) => pkg.installed);
  },

  async installedInventory(): Promise<InstalledInventory> {
    if (isTauriRuntime()) return tauriInvoke("installed_inventory");
    return {
      homebrew: mockPackages.filter((pkg) => pkg.installed),
      other: [
        {
          name: "Notion",
          path: "/Applications/Notion.app",
          version: "4.13.0",
          bundleId: "notion.id"
        },
        {
          name: "ChatGPT",
          path: "/Applications/ChatGPT.app",
          version: "1.2026.189",
          bundleId: "com.openai.chat"
        }
      ]
    };
  },

  async revealInFinder(path: string): Promise<void> {
    if (isTauriRuntime()) return tauriInvoke("reveal_in_finder", { path });
  },

  async outdatedPackages(): Promise<OutdatedPackage[]> {
    if (isTauriRuntime()) return tauriInvoke("outdated_packages");
    await delay(previewDelay(240));
    return mockPackages
      .filter((pkg) => pkg.outdated)
      .map((pkg) => ({
        token: pkg.token,
        name: pkg.name,
        kind: pkg.kind,
        currentVersion: pkg.kind === "cask" ? "latest-1" : "1.0.0",
        latestVersion: pkg.kind === "cask" ? "latest" : "1.1.0"
      }));
  },

  async runBrewAction(action: BrewAction, target?: BrewTarget): Promise<BrewOperation> {
    if (isTauriRuntime()) return tauriInvoke("run_brew_action", { action, target });
    const op: BrewOperation = {
      id: `mock-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      action,
      target,
      commandPreview: mockCommand(action, target),
      status: "queued",
      phase: "preparing",
      phaseLabel: "等待前序任务",
      progress: 0,
      effectiveNetworkMode: mockNetworkConfiguration.effectiveMode,
      proxyUrl: mockNetworkConfiguration.proxyUrl,
      lastActivityAt: Date.now(),
      activityLabel: "任务已加入队列",
      startedAt: Date.now(),
      finishedAt: null,
      exitCode: null,
      logs: [
        {
          stream: "stdout",
          line: `预览模式：将执行 ${mockCommand(action, target)}`,
          at: Date.now()
        }
      ]
    };
    mockOperations.unshift(op);
    mockOperationQueue.push(op);
    window.setTimeout(runNextMockOperation, 0);
    return op;
  },

  async operationEvents(operationId: string): Promise<BrewOperation | null> {
    if (isTauriRuntime()) return tauriInvoke("operation_events", { operationId });
    return mockOperations.find((op) => op.id === operationId) ?? null;
  },

  async cancelOperation(operationId: string): Promise<void> {
    if (isTauriRuntime()) return tauriInvoke("cancel_operation", { operationId });
    const operation = mockOperations.find((item) => item.id === operationId);
    if (!operation || (operation.status !== "queued" && operation.status !== "running")) return;
    const queuedIndex = mockOperationQueue.findIndex((item) => item.id === operationId);
    if (queuedIndex >= 0) mockOperationQueue.splice(queuedIndex, 1);
    emitMockOperation({
      operationId,
      status: "cancelled",
      phase: "cancelled",
      phaseLabel: "已取消",
      finishedAt: Date.now()
    });
  },

  async listenOperations(listener: OperationListener): Promise<() => void> {
    if (isTauriRuntime()) {
      const { listen } = await import("@tauri-apps/api/event");
      return listen<OperationEvent>("brew-operation", (event) => listener(event.payload));
    }
    mockOperationListeners.add(listener);
    return () => mockOperationListeners.delete(listener);
  },

  getMockOperations() {
    return isTauriRuntime() ? [] : mockOperations;
  },

  previewCommand(action: BrewAction, target?: BrewTarget) {
    return mockCommand(action, target);
  },

  kindLabel
};
