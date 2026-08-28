import {
  AlertTriangle,
  AppWindow,
  CheckCircle2,
  CircleStop,
  Clipboard,
  Download,
  ExternalLink,
  FolderOpen,
  HardDriveDownload,
  Info,
  ListChecks,
  Loader2,
  PackageSearch,
  RefreshCw,
  Search,
  Settings,
  ShieldAlert,
  Stethoscope,
  SquareTerminal,
  Terminal,
  Trash2,
  Upload,
  X
} from "lucide-react";
import { FormEvent, useEffect, useMemo, useRef, useState } from "react";
import { brewApi } from "./brewApi";
import { localizeDescription } from "./localDescription";
import {
  addChineseDescriptions,
  getTranslationCacheStats,
  getTranslationStatus,
  resetTranslationStatus
} from "./descriptionTranslation";
import {
  invalidateOutdatedCache,
  readOutdatedCache,
  writeOutdatedCache
} from "./outdatedCache";
import { hasMissingCaskAppSource } from "./operationRecovery";
import brewDeskIcon from "../design/brewdesk-icon-liquid-glass.png";
import type {
  BrewAction,
  BrewAvailability,
  BrewEnvironment,
  HomebrewUpdateStatus,
  DetectedApplication,
  LocalApplication,
  NetworkConfiguration,
  NetworkMode,
  NetworkTestResult,
  BrewOperation,
  BrewTarget,
  OperationEvent,
  OutdatedPackage,
  PackageDetail,
  PackageKind,
  PackageSummary,
  SearchResponse,
  SearchKind,
  TranslationResult,
  ViewKey
} from "./types";

const views: Array<{ key: ViewKey; label: string; icon: typeof PackageSearch }> = [
  { key: "search", label: "搜索", icon: PackageSearch },
  { key: "installed", label: "已安装", icon: HardDriveDownload },
  { key: "outdated", label: "可更新", icon: Upload },
  { key: "operations", label: "操作记录", icon: Terminal },
  { key: "settings", label: "设置", icon: Settings }
];

const actionLabels: Record<BrewAction, string> = {
  install: "安装",
  uninstall: "卸载",
  upgrade: "更新",
  repair: "修复安装",
  "upgrade-all": "全部更新",
  update: "更新 Homebrew 引用",
  doctor: "健康检查"
};

function packageKindLabel(kind: PackageKind) {
  return kind;
}

function packageKindDescription(kind: PackageKind) {
  return kind === "cask"
    ? "cask 通常是带图形界面的 macOS 应用，例如编辑器、浏览器或设计工具。"
    : "formula 通常是命令行工具或运行库，会安装到 Homebrew 的 prefix 目录中。";
}

function homepageDomain(homepage?: string | null) {
  if (!homepage) return "主页未知";
  try {
    return new URL(homepage).hostname;
  } catch {
    return "主页未知";
  }
}

function PackageDescription({
  description,
  kind,
  token,
  localizedDescription,
  descriptionSource,
  showOriginal = false,
  className
}: {
  description?: string | null;
  kind: PackageKind;
  token: string;
  localizedDescription?: string | null;
  descriptionSource?: "apple" | "curated" | "original";
  showOriginal?: boolean;
  className?: string;
}) {
  const localized = localizeDescription(description, kind, token);
  const chinese = localizedDescription || localized.chinese;
  return (
    <span className={className} title={!showOriginal && localized.english ? `Homebrew 原文：${localized.english}` : undefined}>
      <span className={chinese ? undefined : "description-english-primary"}>
        {chinese ?? localized.english ?? "Homebrew 暂未提供用途说明"}
      </span>
      {showOriginal && chinese && localized.english ? (
        <small className="description-original">
          {descriptionSource === "apple" ? "Apple 本机翻译" : "中文说明"} · Homebrew 原文：{localized.english}
        </small>
      ) : showOriginal && !chinese && localized.english ? (
        <small className="description-original">Homebrew 原文 · 暂无可靠中文翻译</small>
      ) : null}
    </span>
  );
}

function actionDescription(action: BrewAction) {
  if (action === "install") return "安装会下载软件包及必要依赖，并写入本机 Homebrew 环境。";
  if (action === "uninstall") return "卸载会移除目标包本身；是否保留配置文件由 Homebrew 和软件自身决定。";
  if (action === "upgrade") return "更新只处理当前选中的项目，适合先更新明确需要的软件。";
  if (action === "repair") return "修复安装会清理损坏的 cask 安装记录并重新安装应用；不会使用 zap 主动删除用户配置。";
  if (action === "upgrade-all") return "全部更新会处理所有可更新项目，建议先确认列表和网络状态。";
  if (action === "update") return "刷新 Homebrew 的索引和 tap 信息，不会直接升级已安装软件。";
  return "健康检查会运行 brew doctor，用于发现路径、权限、缓存或网络相关问题。";
}

function statusText(status: BrewOperation["status"]) {
  if (status === "running") return "执行中";
  if (status === "succeeded") return "已完成";
  if (status === "failed") return "失败";
  if (status === "cancelled") return "已取消";
  return "排队中";
}

function isOperationActive(operation?: BrewOperation | null) {
  return operation?.status === "queued" || operation?.status === "running";
}

function targetKey(target?: BrewTarget | null) {
  return target ? `${target.kind}:${target.token}` : null;
}

function commandFor(action: BrewAction, detail?: PackageDetail | null, target?: BrewTarget) {
  if (action === "update") return brewApi.previewCommand("update");
  if (action === "doctor") return brewApi.previewCommand("doctor");
  if (action === "upgrade-all") return brewApi.previewCommand("upgrade-all");
  if (target) return brewApi.previewCommand(action, target);
  if (!detail) return "";
  return brewApi.previewCommand(action, { token: detail.token, kind: detail.kind });
}

export default function App() {
  const cachedOutdated = useMemo(() => readOutdatedCache(), []);
  const [activeView, setActiveView] = useState<ViewKey>("search");
  const [availability, setAvailability] = useState<BrewAvailability | null>(null);
  const [availabilityLoading, setAvailabilityLoading] = useState(true);
  const [installGuideStarted, setInstallGuideStarted] = useState(false);
  const [installGuideLoading, setInstallGuideLoading] = useState(false);
  const [environment, setEnvironment] = useState<BrewEnvironment | null>(null);
  const [homebrewUpdateStatus, setHomebrewUpdateStatus] = useState<HomebrewUpdateStatus | null>(null);
  const [translationStatus, setTranslationStatus] = useState<TranslationResult | null>(null);
  const [translationLoading, setTranslationLoading] = useState(false);
  const [networkConfiguration, setNetworkConfiguration] = useState<NetworkConfiguration | null>(null);
  const [networkLoading, setNetworkLoading] = useState(false);
  const [networkTest, setNetworkTest] = useState<NetworkTestResult | null>(null);
  const [networkTestLoading, setNetworkTestLoading] = useState(false);
  const [brewPathDraft, setBrewPathDraft] = useState("/opt/homebrew/bin/brew");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchKind, setSearchKind] = useState<SearchKind>("both");
  const [searchResults, setSearchResults] = useState<PackageSummary[]>([]);
  const [installed, setInstalled] = useState<PackageSummary[]>([]);
  const [otherApplications, setOtherApplications] = useState<LocalApplication[]>([]);
  const [outdated, setOutdated] = useState<OutdatedPackage[]>(cachedOutdated?.items ?? []);
  const [outdatedFetchedAt, setOutdatedFetchedAt] = useState<number | null>(cachedOutdated?.fetchedAt ?? null);
  const [searchResponse, setSearchResponse] = useState<SearchResponse | null>(null);
  const [selectedSummary, setSelectedSummary] = useState<PackageSummary | null>(null);
  const [selectedDetail, setSelectedDetail] = useState<PackageDetail | null>(null);
  const [selectedLocalApplication, setSelectedLocalApplication] = useState<LocalApplication | null>(null);
  const [operations, setOperations] = useState<BrewOperation[]>(brewApi.getMockOperations());
  const [pendingAction, setPendingAction] = useState<{ action: BrewAction; target?: BrewTarget } | null>(null);
  const [pendingCancel, setPendingCancel] = useState<BrewOperation | null>(null);
  const [cancellingId, setCancellingId] = useState<string | null>(null);
  const [activityOpen, setActivityOpen] = useState(false);
  const [activityDismissed, setActivityDismissed] = useState(false);
  const [loading, setLoading] = useState(false);
  const [installedLoading, setInstalledLoading] = useState(false);
  const [outdatedLoading, setOutdatedLoading] = useState(false);
  const [environmentLoading, setEnvironmentLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const installedLoaded = useRef(false);
  const outdatedLoaded = useRef(false);
  const checkedHomebrewUpdateOperation = useRef<string | null>(null);
  const detailCache = useRef(new Map<string, PackageDetail>());
  const detailPromises = useRef(new Map<string, Promise<PackageDetail>>());
  const detailRequest = useRef(0);
  const operationQueue = useRef<OperationEvent[]>([]);
  const operationFlushTimer = useRef<number | null>(null);

  useEffect(() => {
    let disposed = false;
    void brewApi
      .availability()
      .then((result) => {
        if (!disposed) setAvailability(result);
      })
      .catch((err) => {
        if (!disposed) setError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (!disposed) setAvailabilityLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    void brewApi
      .networkConfiguration()
      .then((result) => {
        if (!disposed) setNetworkConfiguration(result);
      })
      .catch((err) => {
        if (!disposed) setError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (!installGuideStarted || availability?.status === "ready") return;
    let checking = false;
    const timer = window.setInterval(() => {
      if (checking) return;
      checking = true;
      void brewApi
        .availability()
        .then(async (result) => {
          if (result.status === "ready") {
            const verified = await brewApi.environment();
            if (verified.ok) {
              setEnvironment(verified);
              setAvailability(result);
              setInstallGuideStarted(false);
            } else {
              setAvailability({ ...result, status: "invalid", message: verified.message });
            }
          } else {
            setAvailability(result);
          }
        })
        .catch((err) => setError(err instanceof Error ? err.message : String(err)))
        .finally(() => {
          checking = false;
        });
    }, 2000);
    return () => window.clearInterval(timer);
  }, [installGuideStarted, availability?.status]);

  const activeOperations = useMemo(() => operations.filter(isOperationActive), [operations]);
  const activeOperation = useMemo(
    () => operations.find((operation) => operation.status === "running") ?? activeOperations[0],
    [operations, activeOperations]
  );
  const queuedCount = useMemo(
    () => activeOperations.filter((operation) => operation.status === "queued").length,
    [activeOperations]
  );
  const pendingTargetKeys = useMemo(
    () => new Set(activeOperations.map((operation) => targetKey(operation.target)).filter((key): key is string => Boolean(key))),
    [activeOperations]
  );
  const updatePending = activeOperations.some((operation) => operation.action === "update");
  const doctorPending = activeOperations.some((operation) => operation.action === "doctor");
  const upgradeAllPending = activeOperations.some((operation) => operation.action === "upgrade-all");
  const upgradesPending = activeOperations.some(
    (operation) => operation.action === "upgrade" || operation.action === "upgrade-all"
  );
  const selectedOutdated = useMemo(
    () =>
      selectedDetail
        ? outdated.find((pkg) => pkg.token === selectedDetail.token && pkg.kind === selectedDetail.kind)
        : undefined,
    [outdated, selectedDetail]
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void brewApi
      .listenOperations((event) => {
        if (event.status === "succeeded" && event.finishedAt) {
          invalidateOutdatedCache();
          setOutdatedFetchedAt(null);
          outdatedLoaded.current = false;
          installedLoaded.current = false;
        }
        operationQueue.current.push(event);
        if (operationFlushTimer.current !== null) return;
        operationFlushTimer.current = window.setTimeout(() => {
          const events = operationQueue.current.splice(0);
          operationFlushTimer.current = null;
          if (events.length) {
            setOperations((current) => events.reduce(mergeOperationEvent, current));
          }
        }, 80);
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      });

    return () => {
      disposed = true;
      unlisten?.();
      if (operationFlushTimer.current !== null) window.clearTimeout(operationFlushTimer.current);
      operationFlushTimer.current = null;
      operationQueue.current = [];
    };
  }, []);

  useEffect(() => {
    if (activeView === "installed" && !installedLoaded.current && !installedLoading) void refreshInstalled();
    if (activeView === "outdated" && !outdatedLoaded.current && !outdatedLoading) void refreshOutdated();
    if (activeView === "settings" && !environment && !environmentLoading) void refreshEnvironment();
    if (activeView === "settings" && !translationStatus && !translationLoading) void refreshTranslationStatus();
  }, [activeView]);

  useEffect(() => {
    if (availability?.status === "ready" && !homebrewUpdateStatus) void refreshHomebrewUpdateStatus();
  }, [availability?.status]);

  useEffect(() => {
    const completedUpdate = operations.find((operation) => operation.action === "update" && operation.status === "succeeded");
    if (!completedUpdate || checkedHomebrewUpdateOperation.current === completedUpdate.id) return;
    checkedHomebrewUpdateOperation.current = completedUpdate.id;
    void refreshHomebrewUpdateStatus();
  }, [operations]);

  useEffect(() => {
    if (environment?.brewPath) setBrewPathDraft(environment.brewPath);
  }, [environment?.brewPath]);

  useEffect(() => {
    if (
      availability?.status === "ready" &&
      cachedOutdated &&
      availability.brewPath &&
      cachedOutdated.brewPath !== availability.brewPath
    ) {
      invalidateOutdatedCache();
      setOutdated([]);
      setOutdatedFetchedAt(null);
      outdatedLoaded.current = false;
    }
  }, [availability?.status, availability?.brewPath]);

  async function refreshEnvironment() {
    setEnvironmentLoading(true);
    try {
      setEnvironment(await brewApi.environment());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setEnvironmentLoading(false);
    }
  }

  async function refreshHomebrewUpdateStatus() {
    try {
      setHomebrewUpdateStatus(await brewApi.homebrewUpdateStatus());
    } catch {
      setHomebrewUpdateStatus({
        currentVersion: "",
        latestVersion: null,
        updateAvailable: false,
        checked: false,
        message: "暂时无法检查 Homebrew 新版本"
      });
    }
  }

  async function refreshTranslationStatus(force = false) {
    setTranslationLoading(true);
    try {
      setTranslationStatus(await getTranslationStatus(force));
    } catch (err) {
      setTranslationStatus({
        available: false,
        installed: false,
        searchInstalled: false,
        status: "error",
        message: err instanceof Error ? err.message : String(err)
      });
    } finally {
      setTranslationLoading(false);
    }
  }

  async function prepareDescriptionTranslation() {
    setTranslationLoading(true);
    setError(null);
    try {
      await brewApi.prepareTranslation();
      resetTranslationStatus();
      window.setTimeout(() => void refreshTranslationStatus(true), 1500);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setTranslationLoading(false);
    }
  }

  async function refreshInstalled() {
    setInstalledLoading(true);
    try {
      const inventory = await brewApi.installedInventory();
      setInstalled(await addChineseDescriptions(inventory.homebrew));
      setOtherApplications(inventory.other);
      installedLoaded.current = true;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstalledLoading(false);
    }
  }

  async function refreshOutdated() {
    setOutdatedLoading(true);
    try {
      const items = await brewApi.outdatedPackages();
      const fetchedAt = Date.now();
      const brewPath = availability?.brewPath ?? environment?.brewPath ?? brewPathDraft;
      setOutdated(items);
      setOutdatedFetchedAt(fetchedAt);
      writeOutdatedCache({ brewPath, fetchedAt, items });
      outdatedLoaded.current = true;
    } catch (err) {
      setError(
        outdated.length
          ? `后台检查失败，仍显示上次结果：${err instanceof Error ? err.message : String(err)}`
          : err instanceof Error
            ? err.message
            : String(err)
      );
    } finally {
      setOutdatedLoading(false);
    }
  }

  async function submitSearch(query = searchQuery) {
    const trimmed = query.trim();
    if (trimmed.length < 2) return;
    setLoading(true);
    setHasSearched(true);
    setError(null);
    try {
      const response = await brewApi.searchPackages(trimmed, searchKind);
      const marked = response.items.map((pkg) => ({
        ...pkg,
        installed: installed.some((item) => item.token === pkg.token && item.kind === pkg.kind) || pkg.installed,
        outdated: outdated.some((item) => item.token === pkg.token && item.kind === pkg.kind) || pkg.outdated
      }));
      const localizedItems = await addChineseDescriptions(marked);
      setSearchResults(localizedItems);
      setSearchResponse({ ...response, items: localizedItems });
      setSelectedSummary(null);
      setSelectedDetail(null);
      setSelectedLocalApplication(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function loadPackageDetail(summary: PackageSummary): Promise<PackageDetail> {
    const cacheKey = `${summary.kind}:${summary.token}`;
    const cached = detailCache.current.get(cacheKey);
    if (cached) return Promise.resolve(cached);
    const inFlight = detailPromises.current.get(cacheKey);
    if (inFlight) return inFlight;
    const request = brewApi.packageInfo(summary.token, summary.kind).then(async (detail) => {
      const [localizedDetail] = await addChineseDescriptions([detail]);
      detailCache.current.set(cacheKey, localizedDetail);
      detailPromises.current.delete(cacheKey);
      return localizedDetail;
    }, (err) => {
      detailPromises.current.delete(cacheKey);
      throw err;
    });
    detailPromises.current.set(cacheKey, request);
    return request;
  }

  async function selectPackage(summary: PackageSummary) {
    const requestId = ++detailRequest.current;
    setSelectedSummary(summary);
    setSelectedLocalApplication(null);
    setSelectedDetail(null);
    setDetailLoading(true);
    setError(null);
    try {
      const detail = await loadPackageDetail(summary);
      if (requestId !== detailRequest.current) return;
      setSelectedDetail({
        ...detail,
        installed: detail.installed || summary.installed
      });
    } catch (err) {
      if (requestId !== detailRequest.current) return;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (requestId === detailRequest.current) setDetailLoading(false);
    }
  }

  function selectLocalApplication(application: LocalApplication) {
    detailRequest.current += 1;
    setSelectedLocalApplication(application);
    setSelectedSummary(null);
    setSelectedDetail(null);
    setDetailLoading(false);
  }

  async function revealInFinder(path: string) {
    setError(null);
    try {
      await brewApi.revealInFinder(path);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function openHomepage(url: string) {
    setError(null);
    try {
      await brewApi.openExternalUrl(url);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  function requestAction(action: BrewAction, target?: BrewTarget) {
    setPendingAction({ action, target });
  }

  async function confirmAction() {
    if (!pendingAction) return;
    const externalApplications = selectedDetail?.detectedApplications.filter((app) => !app.brewManaged) ?? [];
    if (
      pendingAction.action === "install" &&
      pendingAction.target?.kind === "cask" &&
      pendingAction.target.token === selectedDetail?.token &&
      externalApplications.length
    ) {
      setError("检测到其他渠道安装的同名应用。为避免覆盖或产生重复文件，BrewDesk 已阻止本次安装。");
      setPendingAction(null);
      return;
    }
    setError(null);
    try {
      const operation = await brewApi.runBrewAction(pendingAction.action, pendingAction.target);
      setOperations((current) => [operation, ...current.filter((item) => item.id !== operation.id)]);
      setActivityOpen(false);
      setActivityDismissed(false);
      setPendingAction(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function saveBrewPath(event: FormEvent) {
    event.preventDefault();
    setError(null);
    try {
      await brewApi.setBrewPath(brewPathDraft.trim());
      invalidateOutdatedCache();
      setOutdatedFetchedAt(null);
      setOutdated([]);
      outdatedLoaded.current = false;
      detailCache.current.clear();
      detailPromises.current.clear();
      setAvailability(await brewApi.availability());
      await refreshEnvironment();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  const detailVisible =
    (activeView === "search" || activeView === "installed" || activeView === "outdated") &&
    Boolean(selectedSummary || selectedDetail || selectedLocalApplication || detailLoading);
  const currentLoading =
    activeView === "installed"
      ? installedLoading
      : activeView === "outdated"
        ? outdatedLoading
        : activeView === "settings"
          ? environmentLoading
          : loading;

  async function startHomebrewInstall(networkMode: NetworkMode = "auto") {
    setInstallGuideLoading(true);
    setError(null);
    try {
      await brewApi.startHomebrewInstall(networkMode);
      const currentNetwork = await brewApi.setNetworkMode(networkMode);
      setNetworkConfiguration(currentNetwork);
      setInstallGuideStarted(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstallGuideLoading(false);
    }
  }

  async function changeNetworkMode(mode: NetworkMode) {
    setNetworkLoading(true);
    setError(null);
    try {
      setNetworkConfiguration(await brewApi.setNetworkMode(mode));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setNetworkLoading(false);
    }
  }

  async function testNetworkConnection() {
    setNetworkTestLoading(true);
    setNetworkTest(null);
    setError(null);
    try {
      setNetworkTest(await brewApi.testNetworkConnection());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setNetworkTestLoading(false);
    }
  }

  async function retryWithMirror(operation: BrewOperation) {
    setNetworkLoading(true);
    setError(null);
    try {
      setNetworkConfiguration(await brewApi.setNetworkMode("mirror"));
      const retry = await brewApi.runBrewAction(operation.action, operation.target ?? undefined);
      setOperations((current) => [retry, ...current.filter((item) => item.id !== retry.id)]);
      setActivityOpen(false);
      setActivityDismissed(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setNetworkLoading(false);
    }
  }

  async function cancelOperation(operation: BrewOperation) {
    setCancellingId(operation.id);
    setError(null);
    try {
      await brewApi.cancelOperation(operation.id);
      setPendingCancel(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCancellingId(null);
    }
  }

  if (availabilityLoading || !availability) {
    return <StartupState />;
  }

  if (availability.status !== "ready") {
    return (
      <HomebrewOnboarding
        availability={availability}
        started={installGuideStarted}
        loading={installGuideLoading}
        error={error}
        networkConfiguration={networkConfiguration}
        onStart={(mode) => void startHomebrewInstall(mode)}
        onCopy={() => void navigator.clipboard?.writeText(brewApi.homebrewInstallCommand)}
        onOpenPkg={() => void brewApi.openHomebrewPkg()}
      />
    );
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            <img className="brand-icon" src={brewDeskIcon} alt="" />
          </div>
          <div>
            <div className="brand-title-row">
              <strong>BrewDesk</strong>
              <span className="alpha-badge">Alpha</span>
            </div>
            <span>Homebrew 管理台</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="主导航">
          {views.map((item) => {
            const Icon = item.icon;
            const badge = item.key === "outdated" ? outdated.length : item.key === "operations" ? operations.length : null;
            return (
              <button
                key={item.key}
                className={activeView === item.key ? "nav-item selected" : "nav-item"}
                onClick={() => {
                  if (item.key !== activeView) {
                    detailRequest.current += 1;
                    setSelectedSummary(null);
                    setSelectedDetail(null);
                    setDetailLoading(false);
                  }
                  setActiveView(item.key);
                  if (item.key === "operations") setActivityOpen(false);
                }}
              >
                <Icon size={17} />
                <span>{item.label}</span>
                {badge ? <b>{badge}</b> : null}
              </button>
            );
          })}
        </nav>

        <div className={availability.status === "ready" && environment?.ok !== false ? "env-card ok" : "env-card"}>
          <span className="status-dot" />
          <div>
            <strong>{environment?.version ?? "Homebrew 已就绪"}</strong>
            <span>{environment ? environment.message : availability.message}</span>
          </div>
          {homebrewUpdateStatus?.updateAvailable ? (
            <button
              type="button"
              className="env-update-button"
              aria-label={homebrewUpdateStatus.message}
              title={homebrewUpdateStatus.message}
              disabled={updatePending}
              onClick={() => requestAction("update")}
            >
              <Download size={15} />
            </button>
          ) : null}
        </div>
      </aside>

      <section className={error ? "workspace has-error" : "workspace"}>
        <header className="topbar">
          <form
            className="searchbar"
            onSubmit={(event) => {
              event.preventDefault();
              setActiveView("search");
              void submitSearch();
            }}
          >
            <Search size={18} />
            <input
              aria-label="搜索 Homebrew 包"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="搜索 Homebrew 包"
            />
            <select value={searchKind} onChange={(event) => setSearchKind(event.target.value as SearchKind)}>
              <option value="both">全部</option>
              <option value="cask">cask</option>
              <option value="formula">formula</option>
            </select>
            <button type="submit" className="primary search-submit" disabled={loading || searchQuery.trim().length < 2}>
              {loading ? <Loader2 className="spin" size={16} /> : <PackageSearch size={16} />}
              <span>搜索</span>
            </button>
          </form>

          <div className="top-actions">
            <button
              className="utility-button"
              onClick={() => requestAction("doctor")}
              disabled={doctorPending}
              aria-label="运行健康检查"
              title="运行健康检查"
            >
              <Stethoscope size={16} />
            </button>
            <button
              className="utility-button"
              onClick={() => requestAction("update")}
              disabled={updatePending}
              aria-label="刷新 Homebrew 源数据"
              title="刷新 Homebrew 源数据"
            >
              <RefreshCw size={16} />
            </button>
          </div>
        </header>

        {error ? (
          <div className="error-banner">
            <ShieldAlert size={17} />
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="关闭错误">
              <X size={14} />
            </button>
          </div>
        ) : null}

        <div className={detailVisible ? "content-grid" : "content-grid single"}>
          <section className="main-panel">
            <PanelHeader
              activeView={activeView}
              loading={currentLoading}
              hasSearched={hasSearched}
              outdatedCount={outdated.length}
              outdatedFetchedAt={outdatedFetchedAt}
              searchResponse={searchResponse}
              onRefresh={() => {
                if (activeView === "installed") void refreshInstalled();
                if (activeView === "outdated") void refreshOutdated();
                if (activeView === "search") void submitSearch();
              }}
              onUpgradeAll={() => requestAction("upgrade-all")}
              disabled={upgradesPending}
            />

            {activeView === "outdated" ? (
              <OutdatedList
                items={outdated}
                loading={outdatedLoading}
                selected={selectedDetail}
                pendingTargetKeys={pendingTargetKeys}
                bulkUpgradePending={upgradeAllPending}
                onUpdate={(item) => requestAction("upgrade", { token: item.token, kind: item.kind })}
                onSelect={(item) =>
                  selectPackage({
                    token: item.token,
                    name: item.name,
                    kind: item.kind,
                    description: `${item.currentVersion} -> ${item.latestVersion}`,
                    installed: true,
                    outdated: true
                  })
                }
              />
            ) : activeView === "installed" ? (
              <InstalledInventoryList
                homebrew={installed}
                other={otherApplications}
                selectedPackage={selectedSummary}
                selectedApplication={selectedLocalApplication}
                loading={installedLoading}
                onSelectPackage={selectPackage}
                onSelectApplication={selectLocalApplication}
                onPreview={loadPackageDetail}
              />
            ) : activeView === "operations" ? (
              <OperationsList
                operations={operations}
                retrying={networkLoading}
                onRetryWithMirror={(operation) => void retryWithMirror(operation)}
                onRepairMissingCask={(operation) => requestAction("repair", operation.target ?? undefined)}
                cancellingId={cancellingId}
                onCancel={setPendingCancel}
              />
            ) : activeView === "settings" ? (
              <SettingsPanel
                environment={environment}
                brewPathDraft={brewPathDraft}
                setBrewPathDraft={setBrewPathDraft}
                onSubmit={saveBrewPath}
                onRefresh={refreshEnvironment}
                networkConfiguration={networkConfiguration}
                onNetworkModeChange={(mode) => void changeNetworkMode(mode)}
                networkLoading={networkLoading}
                networkTest={networkTest}
                networkTestLoading={networkTestLoading}
                onTestNetwork={() => void testNetworkConnection()}
                loading={environmentLoading}
                translationStatus={translationStatus}
                translationLoading={translationLoading}
                onPrepareTranslation={() => void prepareDescriptionTranslation()}
                onRefreshTranslation={() => void refreshTranslationStatus(true)}
              />
            ) : (
              <PackageList
                items={searchResults}
                selected={selectedSummary}
                loading={loading}
                hasSearched={hasSearched}
                emptyMessage={
                  activeView === "search" && searchResponse && /[\u3400-\u9fff]/u.test(searchResponse.query)
                    ? searchResponse.warnings?.[0] ?? "暂未识别这个中文名称，请尝试英文名。"
                    : undefined
                }
                onSelect={selectPackage}
                onPreview={loadPackageDetail}
                onQuickSearch={(query) => {
                  setSearchQuery(query);
                  void submitSearch(query);
                }}
              />
            )}
          </section>

          {detailVisible ? (
            <aside className="detail-panel">
              {selectedLocalApplication ? (
                <LocalApplicationDetail application={selectedLocalApplication} onReveal={revealInFinder} />
              ) : (
                <DetailPanel
                  detail={selectedDetail}
                  summary={selectedSummary}
                  loading={detailLoading}
                  outdated={selectedOutdated}
                  targetPending={
                    selectedDetail
                      ? pendingTargetKeys.has(targetKey(selectedDetail) ?? "") ||
                        (upgradeAllPending && selectedDetail.installed)
                      : false
                  }
                  queueActive={activeOperations.length > 0}
                  onAction={requestAction}
                  onReveal={revealInFinder}
                  onOpenHomepage={(url) => void openHomepage(url)}
                />
              )}
            </aside>
          ) : null}
        </div>

        {activeView !== "operations" && !(activeView === "search" && !hasSearched) ? (
          <ActivityDrawer
            operations={operations}
            activeOperation={activeOperation}
            queuedCount={queuedCount}
            open={activityOpen}
            dismissed={activityDismissed}
            onOpen={() => setActivityOpen(true)}
            onClose={() => setActivityOpen(false)}
            onDismiss={() => {
              setActivityOpen(false);
              setActivityDismissed(true);
            }}
            cancellingId={cancellingId}
            onCancel={setPendingCancel}
          />
        ) : null}
      </section>

      {pendingAction ? (
        <ConfirmModal
          action={pendingAction.action}
          target={pendingAction.target}
          command={commandFor(pendingAction.action, selectedDetail, pendingAction.target)}
          conflicts={
            pendingAction.action === "install" && pendingAction.target?.token === selectedDetail?.token
              ? selectedDetail?.detectedApplications.filter((app) => !app.brewManaged) ?? []
              : []
          }
          onCancel={() => setPendingAction(null)}
          onConfirm={confirmAction}
        />
      ) : null}
      {pendingCancel ? (
        <CancelOperationModal
          operation={pendingCancel}
          busy={cancellingId === pendingCancel.id}
          onClose={() => setPendingCancel(null)}
          onConfirm={() => void cancelOperation(pendingCancel)}
        />
      ) : null}
    </main>
  );
}

function StartupState() {
  return (
    <main className="startup-state" aria-live="polite">
      <div className="startup-orb"><Loader2 className="spin" size={24} /></div>
      <strong>正在准备 BrewDesk</strong>
      <span>只检查本机默认路径，不运行 brew 命令。</span>
    </main>
  );
}

function HomebrewOnboarding({
  availability,
  started,
  loading,
  error,
  networkConfiguration,
  onStart,
  onCopy,
  onOpenPkg
}: {
  availability: BrewAvailability;
  started: boolean;
  loading: boolean;
  error: string | null;
  networkConfiguration: NetworkConfiguration | null;
  onStart: (mode: NetworkMode) => void;
  onCopy: () => void;
  onOpenPkg: () => void;
}) {
  return (
    <main className="onboarding-shell">
      <section className="onboarding-card">
        <div className="onboarding-mark" aria-hidden="true">
          <img className="brand-icon" src={brewDeskIcon} alt="" />
        </div>
        <p className="eyebrow">首次使用 · 安全引导</p>
        <h1>先为这台 Mac 准备 Homebrew</h1>
        <p className="onboarding-lead">
          BrewDesk 没有在默认位置找到 Homebrew。安装会在 macOS Terminal 中完成；确认提示和管理员密码始终由系统终端处理，BrewDesk 不读取也不保存。
        </p>

        <div className="onboarding-facts">
          <article>
            <span>系统架构</span>
            <strong>{availability.arch}</strong>
          </article>
          <article>
            <span>推荐路径</span>
            <code>{availability.recommendedPath}</code>
          </article>
          <article>
            <span>命令行工具</span>
            <strong>{availability.commandLineToolsAvailable ? "已检测到" : "安装器会提示"}</strong>
          </article>
        </div>

        <div className="official-command">
          <div>
            <Terminal size={16} />
            <strong>Homebrew 官方安装命令</strong>
          </div>
          <code>{brewApi.homebrewInstallCommand}</code>
        </div>

        {error ? <div className="onboarding-error"><ShieldAlert size={16} />{error}</div> : null}
        {started ? (
          <div className="install-wait" role="status">
            <Loader2 className="spin" size={18} />
            <span>
              Terminal 已打开，当前使用
              {networkConfiguration?.effectiveMode === "mirror"
                ? "备用线路"
                : networkConfiguration?.effectiveMode === "system-proxy"
                  ? "系统代理"
                  : "官方直连"}
              。BrewDesk 每 2 秒检查一次默认路径，安装完成后会自动进入主界面。
            </span>
          </div>
        ) : null}

        <div className="onboarding-actions">
          <button className="primary install-primary" onClick={() => onStart("auto")} disabled={loading}>
            {loading ? <Loader2 className="spin" size={17} /> : <Download size={17} />}
            {started ? "重新打开 Terminal" : "开始安装 Homebrew"}
          </button>
          <button className="ghost" onClick={() => onStart("mirror")} disabled={loading}>
            <RefreshCw size={16} />网络不通？使用备用线路
          </button>
          <button className="ghost" onClick={onCopy}><Clipboard size={16} />复制命令</button>
          <button className="ghost" onClick={onOpenPkg}><HardDriveDownload size={16} />打开官方 PKG 页面</button>
        </div>

        <p className="onboarding-note">
          默认优先使用官方线路和 macOS 系统代理；备用线路仅对本次安装生效，不修改 shell 配置。脚本内容固定且权限为 0700，不接受任何用户命令文本。
        </p>
      </section>
    </main>
  );
}

function ActivityDrawer({
  operations,
  activeOperation,
  queuedCount,
  open,
  dismissed,
  onOpen,
  onClose,
  onDismiss,
  cancellingId,
  onCancel
}: {
  operations: BrewOperation[];
  activeOperation?: BrewOperation;
  queuedCount: number;
  open: boolean;
  dismissed: boolean;
  onOpen: () => void;
  onClose: () => void;
  onDismiss: () => void;
  cancellingId: string | null;
  onCancel: (operation: BrewOperation) => void;
}) {
  const selected = activeOperation ?? operations[0];
  if (dismissed) return null;
  if (!open) {
    return (
      <div className="activity-fab" role="status" aria-label="Homebrew 任务状态">
        <button className="activity-fab-main" type="button" onClick={onOpen}>
          {activeOperation ? <Loader2 className="spin" size={16} /> : <Terminal size={16} />}
          <span>
            {activeOperation
              ? `${activeOperation.target?.token ?? actionLabels[activeOperation.action]}：${statusText(activeOperation.status)}`
              : selected
                ? `最近操作：${statusText(selected.status)}`
                : "查看操作记录"}
          </span>
          {queuedCount > 0 ? <b>{queuedCount} 项排队</b> : null}
        </button>
        <button className="activity-fab-close" type="button" onClick={onDismiss} aria-label="关闭任务状态提示">
          <X size={14} />
        </button>
      </div>
    );
  }

  return (
    <section className="activity-drawer">
      <header>
        <div>
          <strong>操作记录</strong>
          <span>{selected ? statusText(selected.status) : "等待操作"}</span>
          {queuedCount > 0 ? <span>{queuedCount} 项排队</span> : null}
        </div>
        <code>{selected?.commandPreview ?? "brew 命令会在这里显示"}</code>
        <div className="activity-header-actions">
          {selected && isOperationActive(selected) ? (
            <button className="danger-subtle" type="button" onClick={() => onCancel(selected)} disabled={cancellingId === selected.id}>
              {cancellingId === selected.id ? <Loader2 className="spin" size={14} /> : <CircleStop size={14} />}
              {cancellingId === selected.id ? "正在停止" : "取消任务"}
            </button>
          ) : null}
          <button className="icon-button" type="button" onClick={onClose} aria-label="收起操作记录">
            <X size={15} />
          </button>
        </div>
      </header>
      {selected ? <OperationProgress operation={selected} compact /> : null}
      <div className="activity-grid">
        <div className="activity-list">
          {operations.slice(0, 5).map((operation) => (
            <div key={operation.id} className="activity-row">
              <span className={`op-status ${operation.status}`}>{statusText(operation.status)}</span>
              <code>{operation.commandPreview}</code>
            </div>
          ))}
        </div>
        <div className="log-box compact">
          {selected?.logs.length ? (
            selected.logs.slice(-6).map((log, index) => (
              <p key={`${selected.id}-dock-${index}`} className={log.stream}>
                <b>{log.stream}</b>
                <span>{log.line}</span>
              </p>
            ))
          ) : (
            <p className="stdout">
              <b>system</b>
              <span>执行安装、升级、刷新源数据或健康检查后，会在这里显示实时 stdout/stderr。</span>
            </p>
          )}
        </div>
      </div>
    </section>
  );
}

function mergeOperationEvent(operations: BrewOperation[], event: OperationEvent) {
  return operations.map((operation) => {
    if (operation.id !== event.operationId) return operation;
    return {
      ...operation,
      status: event.status,
      phase: event.phase ?? operation.phase,
      phaseLabel: event.phaseLabel ?? operation.phaseLabel,
      progress: event.progress ?? operation.progress,
      currentItem: event.currentItem === undefined ? operation.currentItem : event.currentItem,
      networkIssue: event.networkIssue ?? operation.networkIssue,
      effectiveNetworkMode: event.effectiveNetworkMode ?? operation.effectiveNetworkMode,
      proxyUrl: event.effectiveNetworkMode != null ? (event.proxyUrl ?? null) : operation.proxyUrl,
      lastActivityAt: event.lastActivityAt ?? operation.lastActivityAt,
      transferBytesPerSecond: event.transferBytesPerSecond ?? operation.transferBytesPerSecond,
      downloadedBytes: event.downloadedBytes ?? operation.downloadedBytes,
      totalBytes: event.totalBytes ?? operation.totalBytes,
      activityLabel: event.activityLabel ?? operation.activityLabel,
      exitCode: event.exitCode ?? operation.exitCode,
      finishedAt: event.finishedAt ?? operation.finishedAt,
      logs: event.log ? [...operation.logs.slice(-999), event.log] : operation.logs
    };
  });
}

function PanelHeader({
  activeView,
  loading,
  hasSearched,
  outdatedCount,
  outdatedFetchedAt,
  searchResponse,
  disabled,
  onRefresh,
  onUpgradeAll
}: {
  activeView: ViewKey;
  loading: boolean;
  hasSearched: boolean;
  outdatedCount: number;
  outdatedFetchedAt: number | null;
  searchResponse: SearchResponse | null;
  disabled: boolean;
  onRefresh: () => void;
  onUpgradeAll: () => void;
}) {
  const title =
    activeView === "installed"
      ? "已安装"
      : activeView === "outdated"
        ? "可更新"
        : activeView === "operations"
          ? "操作记录"
          : activeView === "settings"
            ? "设置"
            : hasSearched
              ? "搜索结果"
              : "发现软件";
  const description =
    activeView === "outdated"
      ? `${outdatedCount} 个包有可用更新。${loading ? "正在后台检查，当前页面仍可操作。" : outdatedFetchedAt ? `检查于 ${new Date(outdatedFetchedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}。` : "打开页面后在后台检查。"}`
      : activeView === "operations"
        ? "这里保留 brew 的 stdout / stderr，失败时优先查看最后几行错误和下载 URL。"
        : activeView === "settings"
          ? "配置 brew 路径、检测 Homebrew 环境，并查看代理和网络使用说明。"
          : activeView === "installed"
            ? "按 Homebrew 与其他渠道分组；其他渠道仅展示本机信息，Mac App Store 应用不列入。"
            : hasSearched
              ? searchResponse?.resolvedQuery
                ? `${searchResponse.source === "translation" ? "Apple 已在本机将查询翻译为" : "已将查询扩展为"} ${searchResponse.resolvedQuery}。停留在结果上可快速预览。`
                : "选择一个结果查看版本、状态和可执行操作；停留 250ms 可快速预览。"
              : "支持中文软件名和常用别名；查询只在需要时开始。";
  return (
    <div className="panel-header">
      <div>
        <h1>{title}</h1>
        <p>{description}</p>
        {activeView === "search" && searchResponse?.warnings?.length ? (
          <p className="search-warning">{searchResponse.warnings.join("；")}</p>
        ) : null}
      </div>
      <div className="header-actions">
        {activeView === "outdated" ? (
          <button className="danger-soft" onClick={onUpgradeAll} disabled={disabled || outdatedCount === 0}>
            <Upload size={15} />
            全部更新
          </button>
        ) : null}
        {activeView !== "operations" && !(activeView === "search" && !hasSearched) ? (
          <button className="icon-button" onClick={onRefresh} disabled={loading} aria-label="刷新">
            <RefreshCw size={16} className={loading ? "spin" : ""} />
          </button>
        ) : null}
      </div>
    </div>
  );
}

function PackageList({
  items,
  selected,
  loading,
  hasSearched,
  emptyMessage,
  onSelect,
  onPreview,
  onQuickSearch
}: {
  items: PackageSummary[];
  selected: PackageSummary | null;
  loading: boolean;
  hasSearched: boolean;
  emptyMessage?: string;
  onSelect: (item: PackageSummary) => void;
  onPreview: (item: PackageSummary) => Promise<PackageDetail>;
  onQuickSearch: (query: string) => void;
}) {
  if (loading) {
    return (
      <div className="empty-state">
        <Loader2 className="spin" size={24} />
        <strong>正在查询 Homebrew</strong>
      </div>
    );
  }
  if (!items.length) {
    return (
      <div className="empty-state">
        <PackageSearch size={28} />
        <strong>{hasSearched ? "没有找到匹配项目" : "搜索需要时才开始"}</strong>
        <span>
          {hasSearched
            ? emptyMessage ?? "换一个名称，或切换 cask / formula 范围后重试。"
            : "输入至少两个字符。cask 多为 macOS 应用，formula 多为命令行工具。"}
        </span>
        {!hasSearched ? (
          <div className="quick-searches" aria-label="常用分类">
            {["浏览器", "代码编辑器", "终端", "视频播放器", "截图", "解压软件"].map((query) => (
              <button type="button" key={query} onClick={() => onQuickSearch(query)}>{query}</button>
            ))}
          </div>
        ) : null}
      </div>
    );
  }
  return (
    <div className="package-list">
      {items.map((item) => (
        <PackageRow
          key={`${item.kind}:${item.token}`}
          item={item}
          selected={selected?.token === item.token && selected.kind === item.kind}
          onSelect={onSelect}
          onPreview={onPreview}
        />
      ))}
    </div>
  );
}

function InstalledInventoryList({
  homebrew,
  other,
  selectedPackage,
  selectedApplication,
  loading,
  onSelectPackage,
  onSelectApplication,
  onPreview
}: {
  homebrew: PackageSummary[];
  other: LocalApplication[];
  selectedPackage: PackageSummary | null;
  selectedApplication: LocalApplication | null;
  loading: boolean;
  onSelectPackage: (item: PackageSummary) => void;
  onSelectApplication: (item: LocalApplication) => void;
  onPreview: (item: PackageSummary) => Promise<PackageDetail>;
}) {
  if (loading) {
    return (
      <div className="empty-state">
        <Loader2 className="spin" size={24} />
        <strong>正在扫描本机应用</strong>
        <span>正在整理 Homebrew 与其他渠道应用，App Store 应用不会显示。</span>
      </div>
    );
  }
  if (!homebrew.length && !other.length) {
    return (
      <div className="empty-state">
        <HardDriveDownload size={28} />
        <strong>没有找到可显示的应用</strong>
        <span>这里只展示 Homebrew 安装项目和其他渠道的 macOS 应用。</span>
      </div>
    );
  }
  return (
    <div className="installed-inventory">
      <section className="source-group">
        <header>
          <div>
            <strong>Homebrew 管理</strong>
            <span>formula 与 cask</span>
          </div>
          <b>{homebrew.length}</b>
        </header>
        <div className="package-list grouped-list">
          {homebrew.map((item) => (
            <PackageRow
              key={`${item.kind}:${item.token}`}
              item={item}
              selected={selectedPackage?.token === item.token && selectedPackage.kind === item.kind}
              onSelect={onSelectPackage}
              onPreview={onPreview}
            />
          ))}
        </div>
      </section>

      <section className="source-group">
        <header>
          <div>
            <strong>其他渠道</strong>
            <span>不包含 Mac App Store</span>
          </div>
          <b>{other.length}</b>
        </header>
        {other.length ? (
          <div className="package-list grouped-list">
            {other.map((application) => (
              <LocalApplicationRow
                key={application.path}
                application={application}
                selected={selectedApplication?.path === application.path}
                onSelect={onSelectApplication}
              />
            ))}
          </div>
        ) : (
          <p className="source-empty">没有检测到其他渠道应用。</p>
        )}
      </section>
    </div>
  );
}

function LocalApplicationRow({
  application,
  selected,
  onSelect
}: {
  application: LocalApplication;
  selected: boolean;
  onSelect: (item: LocalApplication) => void;
}) {
  return (
    <button
      className={selected ? "package-row selected local-app-row" : "package-row local-app-row"}
      onClick={() => onSelect(application)}
      aria-label={`${application.name}，其他渠道应用`}
    >
      <div className="package-heading">
        <span className="type-icon cask" aria-hidden="true"><AppWindow size={17} /></span>
        <span>
          <strong>{application.name}</strong>
          <small>{application.bundleId || application.path}</small>
        </span>
      </div>
      <span>{application.path}</span>
      <div className="row-meta">
        <b>其他渠道</b>
        <em className="installed">{application.version || "未声明版本"}</em>
      </div>
    </button>
  );
}

function PackageRow({
  item,
  selected,
  onSelect,
  onPreview
}: {
  item: PackageSummary;
  selected: boolean;
  onSelect: (item: PackageSummary) => void;
  onPreview: (item: PackageSummary) => Promise<PackageDetail>;
}) {
  const [previewOpen, setPreviewOpen] = useState(false);
  const [preview, setPreview] = useState<PackageDetail | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const timer = useRef<number | null>(null);
  const alive = useRef(true);

  useEffect(() => () => {
    alive.current = false;
    if (timer.current !== null) window.clearTimeout(timer.current);
  }, []);

  useEffect(() => {
    if (selected) closePreview();
  }, [selected]);

  function schedulePreview() {
    if (selected || timer.current !== null || previewOpen) return;
    timer.current = window.setTimeout(() => {
      timer.current = null;
      setPreviewOpen(true);
      setPreviewLoading(true);
      void onPreview(item)
        .then((detail) => {
          if (alive.current) setPreview(detail);
        })
        .catch(() => undefined)
        .finally(() => {
          if (alive.current) setPreviewLoading(false);
        });
    }, 250);
  }

  function closePreview() {
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = null;
    setPreviewOpen(false);
  }

  const TypeIcon = item.kind === "cask" ? AppWindow : SquareTerminal;
  return (
    <button
      className={selected ? "package-row selected" : "package-row"}
      onClick={() => {
        closePreview();
        onSelect(item);
      }}
      onMouseEnter={schedulePreview}
      onMouseLeave={closePreview}
      onFocus={schedulePreview}
      onBlur={closePreview}
      onKeyDown={(event) => {
        if (event.key === "Escape") closePreview();
      }}
      aria-label={`${item.name}，${item.kind === "cask" ? "Homebrew cask" : "Homebrew formula"}`}
    >
      <div className="package-heading">
        <span className={`type-icon ${item.kind}`} aria-hidden="true"><TypeIcon size={17} /></span>
        <span>
          <strong>{item.localizedName || item.name}</strong>
          {item.localizedName ? <small>{item.name}</small> : null}
          <code>{item.token}</code>
        </span>
      </div>
      <PackageDescription
        description={item.description}
        kind={item.kind}
        token={item.token}
        localizedDescription={item.localizedDescription}
        descriptionSource={item.descriptionSource}
        className="package-description"
      />
      <div className="row-meta">
        <b>{packageKindLabel(item.kind)}</b>
        {item.installed ? <em className="installed">已安装</em> : null}
        {item.outdated ? <em className="outdated">可更新</em> : null}
      </div>
      {previewOpen ? (
        <span className="package-preview" role="status">
          <strong>{item.localizedName || preview?.name || item.name}</strong>
          <PackageDescription
            description={preview?.description || item.description}
            kind={item.kind}
            token={item.token}
            localizedDescription={preview?.localizedDescription || item.localizedDescription}
            descriptionSource={preview?.descriptionSource || item.descriptionSource}
            showOriginal
            className="preview-description"
          />
          <span className="preview-grid">
            <small>{item.kind === "cask" ? "图形应用" : "命令行工具"}</small>
            <small>{preview?.version || item.version || (previewLoading ? "读取版本…" : "Homebrew 未提供版本")}</small>
            <small>{homepageDomain(preview?.homepage || item.homepage)}</small>
            <small>
              {preview?.detectedApplications.some((app) => !app.brewManaged)
                ? "检测到其他渠道安装"
                : preview?.installed || item.installed
                  ? (item.outdated ? "可更新" : "已安装")
                  : "未安装"}
            </small>
          </span>
        </span>
      ) : null}
    </button>
  );
}

function OutdatedList({
  items,
  loading,
  selected,
  pendingTargetKeys,
  bulkUpgradePending,
  onUpdate,
  onSelect
}: {
  items: OutdatedPackage[];
  loading: boolean;
  selected: PackageDetail | null;
  pendingTargetKeys: Set<string>;
  bulkUpgradePending: boolean;
  onUpdate: (item: OutdatedPackage) => void;
  onSelect: (item: OutdatedPackage) => void;
}) {
  if (loading && !items.length) {
    return (
      <div className="empty-state">
        <Loader2 className="spin" size={24} />
        <strong>正在检查更新</strong>
      </div>
    );
  }
  if (!items.length) {
    return (
      <div className="empty-state">
        <CheckCircle2 size={28} />
        <strong>当前没有可更新项目</strong>
        <span>可以先执行“刷新源数据”更新 Homebrew 索引，再回来查看是否有新版本。</span>
      </div>
    );
  }
  return (
    <div className="package-list">
      {items.map((item) => {
        const pending = bulkUpgradePending || pendingTargetKeys.has(targetKey(item) ?? "");
        return (
          <article
          key={`${item.kind}:${item.token}`}
          className={
            selected?.token === item.token && selected.kind === item.kind ? "outdated-row selected" : "outdated-row"
          }
        >
          <button className="outdated-row-main" type="button" onClick={() => onSelect(item)}>
            <span>
              <strong>{item.name}</strong>
              <code>{item.token}</code>
            </span>
            <span className="version-change">
              {item.currentVersion || "当前版本未知"} <b aria-hidden="true">→</b> {item.latestVersion || "新版本未知"}
            </span>
          </button>
          <div className="outdated-row-actions">
            <span className="outdated-row-badges">
              <b>{packageKindLabel(item.kind)}</b>
              <em className="outdated">可更新</em>
            </span>
            <button
              className="primary update-inline"
              type="button"
              onClick={() => onUpdate(item)}
              disabled={pending}
              aria-label={`更新 ${item.name}`}
            >
              {pending ? <Loader2 className="spin" size={14} /> : <Upload size={14} />}
              {pending ? "已排队" : "更新"}
            </button>
          </div>
          </article>
        );
      })}
    </div>
  );
}

const operationStages = [
  { phase: "preparing", label: "准备" },
  { phase: "resolving", label: "解析" },
  { phase: "downloading", label: "下载" },
  { phase: "installing", label: "安装" },
  { phase: "cleaning", label: "清理" },
  { phase: "verifying", label: "验证" }
] as const;

function formatBytes(bytes?: number | null) {
  if (bytes == null || !Number.isFinite(bytes)) return null;
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

function formatSpeed(bytesPerSecond?: number | null) {
  const size = formatBytes(bytesPerSecond);
  return size ? `${size}/s` : null;
}

function compactOperationItem(value: string) {
  try {
    const url = new URL(value);
    const pathParts = url.pathname.split("/").filter(Boolean);
    const name = decodeURIComponent(pathParts[pathParts.length - 1] ?? "下载内容");
    return `${url.hostname} · ${name}`;
  } catch {
    return value;
  }
}

function networkRoute(operation: BrewOperation) {
  if (operation.effectiveNetworkMode === "mirror") return { label: "备用线路", detail: "清华镜像" };
  if (operation.effectiveNetworkMode === "system-proxy") {
    let endpoint = "macOS 系统代理";
    if (operation.proxyUrl) {
      try {
        endpoint = new URL(operation.proxyUrl).host;
      } catch {
        endpoint = "macOS 系统代理";
      }
    }
    return { label: "系统代理", detail: endpoint };
  }
  return { label: "官方直连", detail: "未使用代理" };
}

function OperationProgress({ operation, compact = false }: { operation: BrewOperation; compact?: boolean }) {
  const progress = operation.progress == null ? null : Math.max(0, Math.min(100, operation.progress));
  const active = isOperationActive(operation);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!active) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [active]);
  const lastLog = operation.logs[operation.logs.length - 1];
  const lastActivityAt = operation.lastActivityAt ?? lastLog?.at ?? operation.startedAt;
  const quietSeconds = active ? Math.max(0, Math.floor((now - lastActivityAt) / 1000)) : 0;
  const retryLog = [...operation.logs].reverse().find((log) => /retrying download/i.test(log.line));
  const retrySeconds = retryLog?.line.match(/retrying download in\s+(\d+)\s*s/i)?.[1];
  const retryRemaining = retryLog && retrySeconds
    ? Math.max(0, Number(retrySeconds) - Math.floor((now - retryLog.at) / 1000))
    : null;
  const currentStageIndex = operationStages.findIndex((stage) => stage.phase === operation.phase);
  const terminalSuccess = operation.status === "succeeded";
  const route = networkRoute(operation);
  const speed = formatSpeed(operation.transferBytesPerSecond);
  const downloaded = formatBytes(operation.downloadedBytes);
  const total = formatBytes(operation.totalBytes);
  const transferAmount = downloaded ? (total ? `${downloaded} / ${total}` : downloaded) : null;
  const hasFreshTransfer = active && operation.phase === "downloading" && Boolean(speed) && quietSeconds <= 10;
  const slowTransfer = hasFreshTransfer && operation.transferBytesPerSecond != null && operation.transferBytesPerSecond < 512 * 1024;
  const liveTone = retryRemaining != null && retryRemaining > 0
    ? "retry"
    : hasFreshTransfer
      ? slowTransfer ? "slow" : "active"
      : quietSeconds >= 60
        ? "warning"
        : quietSeconds >= 20
          ? "waiting"
          : "active";
  const liveTitle = retryRemaining != null && retryRemaining > 0
    ? "下载异常，等待重试"
    : hasFreshTransfer
      ? slowTransfer
        ? operation.effectiveNetworkMode === "system-proxy" ? "代理可达，但传输偏慢" : "线路可达，但传输偏慢"
        : "正在传输"
      : quietSeconds >= 60
        ? "暂无数据，可能在等待网络"
        : quietSeconds >= 20
          ? "暂时没有新数据"
          : active
            ? "Homebrew 正在工作"
            : statusText(operation.status);
  const liveDetail = retryRemaining != null && retryRemaining > 0
    ? `Homebrew 将在 ${retryRemaining} 秒后自动重试`
    : active
      ? [
          route.label,
          hasFreshTransfer ? speed : operation.activityLabel,
          transferAmount ? `当前文件 ${transferAmount}` : null,
          `${quietSeconds} 秒前收到活动`
        ]
          .filter(Boolean)
          .join(" · ")
      : operation.phaseLabel;
  return (
    <div className={`${compact ? "operation-progress compact" : "operation-progress"} status-${operation.status}`} aria-live="polite">
      <div className="progress-copy">
        <strong>{operation.phaseLabel}</strong>
        <span>{progress == null ? (active ? "处理中" : statusText(operation.status)) : `${Math.round(progress)}% · 流程估算`}</span>
      </div>
      <div className="progress-stages" aria-label="操作阶段">
        {operationStages.map((stage, index) => {
          const done = terminalSuccess || currentStageIndex > index;
          const current = active && currentStageIndex === index;
          return (
            <span key={stage.phase} className={done ? "done" : current ? "current" : ""}>
              <i>{done ? <CheckCircle2 size={13} /> : null}</i>
              {stage.label}
            </span>
          );
        })}
      </div>
      <div
        className={progress == null && active ? "progress-track indeterminate" : "progress-track"}
        role="progressbar"
        aria-label={`${actionLabels[operation.action]}进度`}
        aria-valuemin={0}
        aria-valuemax={100}
        {...(progress == null ? { "aria-busy": active } : { "aria-valuenow": progress })}
      >
        <span style={progress == null ? undefined : { width: `${progress}%` }} />
      </div>
      <div className={`operation-live-status ${liveTone}`}>
        <span className="operation-signal" aria-hidden="true" />
        <div>
          <strong>{liveTitle}</strong>
          <span>{liveDetail}</span>
        </div>
        <span className="operation-route" title={route.detail}>{route.label}</span>
      </div>
      {active && operation.currentItem ? (
        <small className="current-operation-item" title={operation.currentItem}>
          <b>当前项目</b>
          <span>{compactOperationItem(operation.currentItem)}</span>
        </small>
      ) : null}
    </div>
  );
}

function OperationsList({
  operations,
  retrying,
  onRetryWithMirror,
  onRepairMissingCask,
  cancellingId,
  onCancel
}: {
  operations: BrewOperation[];
  retrying: boolean;
  onRetryWithMirror: (operation: BrewOperation) => void;
  onRepairMissingCask: (operation: BrewOperation) => void;
  cancellingId: string | null;
  onCancel: (operation: BrewOperation) => void;
}) {
  if (!operations.length) {
    return (
      <div className="empty-state">
        <Terminal size={28} />
        <strong>还没有操作记录</strong>
      </div>
    );
  }
  return (
    <div className="operations">
      {operations.map((operation) => (
        <article className="operation-card" key={operation.id}>
          <header>
            <div>
              <strong>{actionLabels[operation.action]}</strong>
              <code>{operation.commandPreview}</code>
            </div>
            <span className={`op-status ${operation.status}`}>{statusText(operation.status)}</span>
          </header>
          <OperationProgress operation={operation} />
          {isOperationActive(operation) ? (
            <div className="operation-actions">
              <span>取消只会停止当前 Homebrew 进程，不会删除已经安装完成的项目。</span>
              <button className="danger-subtle" type="button" onClick={() => onCancel(operation)} disabled={cancellingId === operation.id}>
                {cancellingId === operation.id ? <Loader2 className="spin" size={14} /> : <CircleStop size={14} />}
                {cancellingId === operation.id ? "正在停止" : operation.status === "queued" ? "移出队列" : "取消任务"}
              </button>
            </div>
          ) : null}
          {operation.status === "failed" && operation.networkIssue ? (
            <div className="network-retry" role="status">
              <div>
                <strong>看起来是网络连接问题</strong>
                <span>可以切换到清华备用线路后，由你确认重新执行。</span>
              </div>
              <button className="ghost" type="button" onClick={() => onRetryWithMirror(operation)} disabled={retrying}>
                <RefreshCw size={15} className={retrying ? "spin" : ""} />
                {retrying ? "正在切换" : "备用线路重试"}
              </button>
            </div>
          ) : null}
          {hasMissingCaskAppSource(operation) ? (
            <div className="network-retry repair-install" role="status">
              <div>
                <strong>应用本体已不在 Homebrew 记录的位置</strong>
                <span>安装记录仍然存在，但旧的 .app 已被移动或删除。可重新安装以恢复应用和 Homebrew 记录。</span>
              </div>
              <button className="ghost" type="button" onClick={() => onRepairMissingCask(operation)}>
                <RefreshCw size={15} />
                修复安装
              </button>
            </div>
          ) : null}
          <div className="log-box">
            {operation.logs.length ? (
              operation.logs.slice(-200).map((log, index) => (
                <p key={`${operation.id}-${index}`} className={log.stream}>
                  <b>{log.stream}</b>
                  <span>{log.line}</span>
                </p>
              ))
            ) : (
              <p className="stdout">
                <b>system</b>
                <span>等待 brew 输出...</span>
              </p>
            )}
          </div>
        </article>
      ))}
    </div>
  );
}

function SettingsPanel({
  environment,
  brewPathDraft,
  setBrewPathDraft,
  onSubmit,
  onRefresh,
  networkConfiguration,
  onNetworkModeChange,
  networkLoading,
  networkTest,
  networkTestLoading,
  onTestNetwork,
  loading,
  translationStatus,
  translationLoading,
  onPrepareTranslation,
  onRefreshTranslation
}: {
  environment: BrewEnvironment | null;
  brewPathDraft: string;
  setBrewPathDraft: (value: string) => void;
  onSubmit: (event: FormEvent) => void;
  onRefresh: () => void;
  networkConfiguration: NetworkConfiguration | null;
  onNetworkModeChange: (mode: NetworkMode) => void;
  networkLoading: boolean;
  networkTest: NetworkTestResult | null;
  networkTestLoading: boolean;
  onTestNetwork: () => void;
  loading: boolean;
  translationStatus: TranslationResult | null;
  translationLoading: boolean;
  onPrepareTranslation: () => void;
  onRefreshTranslation: () => void;
}) {
  const translationCache = getTranslationCacheStats();
  const translationReady = Boolean(translationStatus?.installed && translationStatus.searchInstalled);
  return (
    <div className="settings-panel">
      <form onSubmit={onSubmit}>
        <label htmlFor="brew-path">brew 路径</label>
        <p className="field-help">这里应填写本机 brew 可执行文件路径。Apple Silicon 默认通常是 `/opt/homebrew/bin/brew`。</p>
        <div className="path-row">
          <input id="brew-path" value={brewPathDraft} onChange={(event) => setBrewPathDraft(event.target.value)} />
          <button className="primary" type="submit">
            保存
          </button>
        </div>
      </form>
      <dl className="env-table">
        <div>
          <dt>状态</dt>
          <dd>{environment?.message ?? "未检测"}</dd>
        </div>
        <div>
          <dt>版本</dt>
          <dd>{environment?.version ?? "-"}</dd>
        </div>
        <div>
          <dt>prefix</dt>
          <dd>{environment?.prefix ?? "-"}</dd>
        </div>
        <div>
          <dt>架构</dt>
          <dd>{environment?.arch ?? "-"}</dd>
        </div>
      </dl>
      <button className="ghost" type="button" onClick={onRefresh} disabled={loading}>
        <RefreshCw size={15} className={loading ? "spin" : ""} />
        {loading ? "检测中" : "重新检测"}
      </button>
      <section className="translation-settings-card">
        <div>
          <span className={translationReady ? "translation-status installed" : "translation-status"}>
            {translationReady ? <CheckCircle2 size={15} /> : <Download size={15} />}
            {translationReady ? "已启用" : "可选功能"}
          </span>
          <h3>Apple 本机中文翻译</h3>
          <p>
            {`${translationStatus?.message ?? "正在检测 Apple 本机中英文语言能力"}。简介与中文搜索词均只在这台 Mac 上处理。`}
          </p>
          <div className="translation-storage-meta">
            <span>简介缓存 {formatBytes(translationCache.bytes) ?? "0 B"} · {translationCache.entries} 条</span>
            <span>Apple 语言包大小由 macOS 管理</span>
          </div>
        </div>
        <div className="translation-settings-actions">
          {!translationReady && translationStatus?.status !== "unsupported" && translationStatus?.status !== "unavailable" ? (
            <button className="primary" type="button" onClick={onPrepareTranslation} disabled={translationLoading}>
              {translationLoading ? <Loader2 className="spin" size={15} /> : <Download size={15} />}
              下载语言包
            </button>
          ) : null}
          <button className="ghost" type="button" onClick={onRefreshTranslation} disabled={translationLoading}>
            <RefreshCw size={15} className={translationLoading ? "spin" : ""} />
            重新检测
          </button>
        </div>
      </section>
      <fieldset className="network-mode-card">
        <legend>下载线路</legend>
        <p>推荐保留“稳态自动”：联网任务开始前会检查线路，避开“代理开启但实际不可用”的假在线状态。</p>
        <div className="network-mode-options">
          {([
            {
              mode: "auto",
              label: "稳态自动",
              description: "查询本机代理端口，再检查直连和备用线路",
              detail: null
            },
            {
              mode: "official",
              label: "官方直连",
              description: "完全不使用代理或镜像",
              detail: "不是阉割版：直接使用 Homebrew 官方目录、GitHub 和 GHCR，软件范围最及时。代价是当前网络若无法稳定访问 GitHub / GHCR，更新和 bottle 下载会超时。BrewDesk 不注入代理；若系统启用了 TUN / 虚拟网卡，流量仍可能被系统层接管。"
            },
            {
              mode: "mirror",
              label: "备用线路",
              description: "使用清华 Homebrew 镜像",
              detail: "不会主动删减软件目录，但镜像同步存在时间差，新应用或最新版可能短暂晚于官方。它主要覆盖 Homebrew API、brew Git 和 formula bottle；cask 常从软件官网或第三方 CDN 下载，这部分不会被清华镜像完整覆盖。使用镜像也意味着信任该镜像提供的同步内容。"
            }
          ] as const).map(({ mode, label, description, detail }) => {
            const selected = networkConfiguration?.mode === mode;
            const tooltipId = `network-mode-${mode}-detail`;
            return (
              <div key={mode} className={selected ? "network-mode-option-shell selected" : "network-mode-option-shell"}>
                <button
                  className="network-mode-option"
                  type="button"
                  onClick={() => onNetworkModeChange(mode)}
                  disabled={networkLoading}
                  aria-pressed={selected}
                >
                  <strong>{label}</strong>
                  <span>{description}</span>
                </button>
                {detail ? (
                  <span
                    className="network-mode-info"
                    tabIndex={0}
                    aria-label={`${label}的区别和代价`}
                    aria-describedby={tooltipId}
                  >
                    <Info size={16} aria-hidden="true" />
                    <span className="network-mode-tooltip" id={tooltipId} role="tooltip">
                      <strong>{label}的区别和代价</strong>
                      <span>{detail}</span>
                    </span>
                  </span>
                ) : null}
              </div>
            );
          })}
        </div>
        <div className="network-mode-status" role="status">
          {networkLoading ? <Loader2 className="spin" size={15} /> : <CheckCircle2 size={15} />}
          <span>{networkConfiguration?.message ?? "正在检测网络方式"}</span>
        </div>
        <div className="network-test-row">
          <button className="ghost" type="button" onClick={onTestNetwork} disabled={networkTestLoading || networkLoading}>
            {networkTestLoading ? <Loader2 className="spin" size={15} /> : <Stethoscope size={15} />}
            {networkTestLoading ? "正在测试" : "测试当前模板"}
          </button>
          {networkTest ? (
            <span className={networkTest.reachable ? "network-test-result reachable" : "network-test-result failed"} role="status">
              {networkTest.reachable ? <CheckCircle2 size={15} /> : <AlertTriangle size={15} />}
              {networkTest.message}
            </span>
          ) : (
            <span className="network-test-hint">只测试连通性，不会启动更新或修改系统代理。</span>
          )}
        </div>
      </fieldset>
      <details className="network-notes">
        <summary>网络与代理说明</summary>
        <p>
          BrewDesk 不写死代理端口，也不修改系统网络。稳态自动会查询 macOS 系统代理和本机代理进程的回环监听端口，验证可用后只注入当前 Homebrew 任务；随后再检查官方直连和清华备用线路。所有选择都不会修改万达云、系统代理或 shell 配置。
        </p>
        <div className="network-grid">
          <article>
            <strong>brew update / tap</strong>
            <span>主要访问 GitHub。大陆校园网或公司网不稳定时，优先使用美国、日本、香港或新加坡节点。</span>
          </article>
          <article>
            <strong>formula 安装</strong>
            <span>通常下载 Homebrew bottles 和源码包。美国/日本节点一般更稳；失败时重试或换节点。</span>
          </article>
          <article>
            <strong>cask 安装</strong>
            <span>会跳转到各软件官网/CDN。不同应用差异很大，日志中的下载域名比固定地区更重要。</span>
          </article>
          <article>
            <strong>开发依赖</strong>
            <span>本项目 npm 依赖来自 registry.npmjs.org；当前环境可用但偏慢，最大单包下载约 48 秒。</span>
          </article>
        </div>
      </details>
      <footer className="settings-footer" aria-label="作者信息">
        <span>BrewDesk · 公开 Alpha</span>
        <span>by Q.D.</span>
      </footer>
    </div>
  );
}

function LocalApplicationDetail({
  application,
  onReveal
}: {
  application: LocalApplication;
  onReveal: (path: string) => void;
}) {
  return (
    <div className="detail-content">
      <div className="detail-title">
        <div className="detail-eyebrow">
          <span className="kind-badge cask">其他渠道</span>
          <span className="state-pill installed">本机应用</span>
        </div>
        <h2>{application.name}</h2>
        <code>{application.bundleId || "未声明 Bundle ID"}</code>
      </div>

      <dl className="detail-table">
        <div>
          <dt>应用版本</dt>
          <dd>{application.version || "应用未声明版本"}</dd>
        </div>
        <div>
          <dt>Bundle ID</dt>
          <dd>{application.bundleId || "-"}</dd>
        </div>
        <div>
          <dt>应用路径</dt>
          <dd>{application.path}</dd>
        </div>
        <div>
          <dt>管理渠道</dt>
          <dd>其他渠道</dd>
        </div>
      </dl>

      <div className="detail-actions">
        <button className="ghost" onClick={() => onReveal(application.path)}>
          <FolderOpen size={16} />
          在访达中显示
        </button>
      </div>

      <div className="detection-box">
        <div>
          <ShieldAlert size={18} />
          <strong>只读应用信息</strong>
        </div>
        <p>BrewDesk 只展示这项应用，不会通过 Homebrew 更新或卸载它。Mac App Store 应用已从清单中排除。</p>
      </div>
    </div>
  );
}

function DetailPanel({
  detail,
  summary,
  loading,
  outdated,
  targetPending,
  queueActive,
  onAction,
  onReveal,
  onOpenHomepage
}: {
  detail: PackageDetail | null;
  summary: PackageSummary | null;
  loading: boolean;
  outdated?: OutdatedPackage;
  targetPending: boolean;
  queueActive: boolean;
  onAction: (action: BrewAction, target?: BrewTarget) => void;
  onReveal: (path: string) => void;
  onOpenHomepage: (url: string) => void;
}) {
  if (loading) {
    return (
      <div className="detail-empty">
        <Loader2 className="spin" size={26} />
        <strong>读取详情</strong>
      </div>
    );
  }
  if (!detail) {
    return (
      <div className="detail-empty">
        <ListChecks size={30} />
        <strong>{summary ? "详情不可用" : "选择一个包"}</strong>
        <span>右侧会显示它属于应用还是命令行工具、当前状态、依赖、真实命令和操作说明。</span>
      </div>
    );
  }
  const disabled = targetPending;
  const target = { token: detail.token, kind: detail.kind };
  const externalApplications = detail.detectedApplications.filter((app) => !app.brewManaged);
  const revealPath = detail.installLocation || detail.detectedApplications[0]?.path;
  return (
    <div className="detail-content">
      <div className="detail-title">
        <div className="detail-eyebrow">
          <span className={`kind-badge ${detail.kind}`}>{detail.kind}</span>
          <span className={detail.installed ? "state-pill installed" : "state-pill"}>
            {detail.installed ? (outdated ? "有更新" : "已安装") : "未安装"}
          </span>
        </div>
        <h2>{detail.name}</h2>
        <code>{detail.token}</code>
      </div>
      <PackageDescription
        description={detail.description}
        kind={detail.kind}
        token={detail.token}
        localizedDescription={detail.localizedDescription}
        descriptionSource={detail.descriptionSource}
        showOriginal
        className="detail-desc"
      />

      <dl className="detail-table">
        <div>
          <dt>版本</dt>
          <dd>{detail.version || "-"}</dd>
        </div>
        <div>
          <dt>已安装版本</dt>
          <dd>{detail.installedVersions.length ? detail.installedVersions.join(", ") : "-"}</dd>
        </div>
        <div>
          <dt>主页</dt>
          <dd>{homepageDomain(detail.homepage)}</dd>
        </div>
        <div>
          <dt>本机占用</dt>
          <dd>{detail.installed ? formatBytes(detail.installedSizeBytes) ?? "暂时无法统计" : "尚未安装"}</dd>
        </div>
        <div>
          <dt>下载大小</dt>
          <dd>由 Homebrew 解析；任务开始后实时显示</dd>
        </div>
      </dl>

      {detail.detectedApplications.length ? (
        <div className={externalApplications.length ? "detection-box conflict" : "detection-box"}>
          <div>
            {externalApplications.length ? <AlertTriangle size={18} /> : <CheckCircle2 size={18} />}
            <strong>{externalApplications.length ? "检测到其他渠道安装" : "已核对本机应用"}</strong>
          </div>
          <p>
            {externalApplications.length
              ? "以下同名应用不在当前 Homebrew 安装记录中。BrewDesk 会阻止重复安装，但无法仅凭本机文件准确判断它来自 DMG 还是 PKG。"
              : "检测到的应用与当前 Homebrew 安装记录一致。"}
          </p>
          <ul>
            {detail.detectedApplications.map((app) => (
              <li key={app.path}>
                <span>
                  <b>{app.name}</b>
                  <small>{app.version || "应用未声明版本"} · {app.source}</small>
                </span>
                <code>{app.path}</code>
                {app.bundleId ? <small>{app.bundleId}</small> : null}
              </li>
            ))}
          </ul>
        </div>
      ) : detail.kind === "cask" ? (
        <p className="detection-empty">未在 /Applications 或 ~/Applications 检测到同名应用。</p>
      ) : null}

      <div className="detail-actions">
        {detail.installed ? (
          <>
            <button className="primary" onClick={() => onAction("upgrade", target)} disabled={disabled}>
              <Upload size={16} />
              更新
            </button>
            <button className="danger" onClick={() => onAction("uninstall", target)} disabled={disabled}>
              <Trash2 size={16} />
              卸载
            </button>
          </>
        ) : (
          <button className="primary" onClick={() => onAction("install", target)} disabled={disabled}>
            <Download size={16} />
            安装
          </button>
        )}
        {revealPath ? (
          <button className="ghost" onClick={() => onReveal(revealPath)}>
            <FolderOpen size={16} />
            在访达中显示
          </button>
        ) : null}
        {detail.homepage ? (
          <button className="ghost" onClick={() => onOpenHomepage(detail.homepage)}>
            <ExternalLink size={16} />
            打开项目主页
          </button>
        ) : null}
      </div>

      <details className="detail-disclosure">
        <summary>命令与依赖</summary>
        <section className="command-panel">
          <span>命令预览</span>
          <code>
            {detail.installed
              ? brewApi.previewCommand(outdated ? "upgrade" : "uninstall", target)
              : brewApi.previewCommand("install", target)}
          </code>
        </section>
        <div className="dependency-box">
          <strong>依赖</strong>
          {detail.dependencies.length ? (
            <div>
              {detail.dependencies.slice(0, 12).map((dependency) => (
                <code key={dependency}>{dependency}</code>
              ))}
            </div>
          ) : (
            <span>无或未提供</span>
          )}
        </div>
      </details>

      {queueActive ? (
        <div className="active-lock">
          <ListChecks size={16} />
          <span>
            {targetPending
              ? "这个项目已经在任务队列中，不能重复提交。"
              : "已有任务执行中；你仍可提交这个操作，BrewDesk 会自动排队并按顺序执行。"}
          </span>
        </div>
      ) : null}
    </div>
  );
}

function ConfirmModal({
  action,
  target,
  command,
  conflicts,
  onCancel,
  onConfirm
}: {
  action: BrewAction;
  target?: BrewTarget;
  command: string;
  conflicts: DetectedApplication[];
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const commandPreview = command || brewApi.previewCommand(action, target);
  return (
    <div className="modal-backdrop" role="presentation">
      <section className="confirm-modal" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
        <header>
          <ShieldAlert size={22} />
          <div>
            <h2 id="confirm-title">逐项确认</h2>
            <p>
              {actionLabels[action]} 会调用本机 Homebrew。{actionDescription(action)}
              BrewDesk 不会请求或保存 sudo 密码。
            </p>
          </div>
        </header>
        <div className="risk-box">
          <AlertTriangle size={18} />
          <span>请确认包名、类型和命令无误。需要交互式权限时，请复制命令到终端执行。</span>
        </div>
        {conflicts.length ? (
          <div className="risk-box blocking-risk">
            <ShieldAlert size={18} />
            <span>
              已发现 {conflicts.map((app) => `${app.path}${app.version ? `（${app.version}）` : ""}`).join("、")}。
              为避免覆盖或重复安装，本次操作不可执行。
            </span>
          </div>
        ) : null}
        <div className="command-copy">
          <code>{commandPreview}</code>
          <button
            className="icon-button"
            onClick={() => {
              void navigator.clipboard?.writeText(commandPreview);
            }}
            aria-label="复制命令"
          >
            <Clipboard size={16} />
          </button>
        </div>
        <footer>
          <button className="ghost" onClick={onCancel}>
            取消
          </button>
          <button
            className={action === "uninstall" || action === "upgrade-all" ? "danger" : "primary"}
            onClick={onConfirm}
            disabled={conflicts.length > 0}
          >
            {conflicts.length ? "请先处理已有应用" : "确认执行"}
          </button>
        </footer>
      </section>
    </div>
  );
}

function CancelOperationModal({
  operation,
  busy,
  onClose,
  onConfirm
}: {
  operation: BrewOperation;
  busy: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="modal-backdrop" role="presentation">
      <section className="confirm-modal" role="dialog" aria-modal="true" aria-labelledby="cancel-operation-title">
        <header>
          <CircleStop size={22} />
          <div>
            <h2 id="cancel-operation-title">取消这项任务？</h2>
            <p>{actionLabels[operation.action]}会收到正常的终止请求，可能需要几秒钟才会完全停止。</p>
          </div>
        </header>
        <div className="risk-box">
          <AlertTriangle size={18} />
          <span>已经安装完成的项目不会回滚；未完成的下载可能保留在 Homebrew 缓存中，之后可以重新执行。</span>
        </div>
        <div className="command-copy">
          <code>{operation.commandPreview}</code>
        </div>
        <footer>
          <button className="ghost" onClick={onClose} disabled={busy}>继续等待</button>
          <button className="danger" onClick={onConfirm} disabled={busy}>
            {busy ? <Loader2 className="spin" size={15} /> : <CircleStop size={15} />}
            {busy ? "正在停止" : operation.status === "queued" ? "移出队列" : "确认取消"}
          </button>
        </footer>
      </section>
    </div>
  );
}
