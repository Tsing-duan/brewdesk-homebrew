export type PackageKind = "formula" | "cask";
export type SearchKind = PackageKind | "both";
export type SearchSource = "catalog" | "alias" | "brew" | "translation";
export type ViewKey = "search" | "installed" | "outdated" | "operations" | "settings";
export type BrewAction = "install" | "uninstall" | "upgrade" | "repair" | "upgrade-all" | "update" | "doctor";
export type OperationStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";
export type LogStream = "stdout" | "stderr";
export type NetworkMode = "auto" | "official" | "mirror";
export type EffectiveNetworkMode = "direct" | "system-proxy" | "mirror";
export type OperationPhase =
  | "preparing"
  | "resolving"
  | "downloading"
  | "installing"
  | "cleaning"
  | "verifying"
  | "completed"
  | "failed"
  | "cancelled";

export interface NetworkConfiguration {
  mode: NetworkMode;
  effectiveMode: EffectiveNetworkMode;
  proxyUrl?: string | null;
  message: string;
}

export interface NetworkTestResult {
  reachable: boolean;
  effectiveMode: EffectiveNetworkMode;
  proxyUrl?: string | null;
  latencyMs: number;
  message: string;
}

export interface BrewEnvironment {
  brewPath: string;
  version: string;
  prefix: string;
  arch: string;
  ok: boolean;
  message: string;
}

export interface HomebrewUpdateStatus {
  currentVersion: string;
  latestVersion?: string | null;
  updateAvailable: boolean;
  checked: boolean;
  message: string;
}

export interface PackageSummary {
  token: string;
  name: string;
  kind: PackageKind;
  description: string;
  installed: boolean;
  outdated: boolean;
  localizedName?: string | null;
  version?: string | null;
  homepage?: string | null;
  matchReason?: string | null;
  localizedDescription?: string | null;
  descriptionSource?: "apple" | "curated" | "original";
}

export interface SearchResponse {
  query: string;
  resolvedQuery?: string | null;
  source: SearchSource;
  warnings?: string[];
  items: PackageSummary[];
}

export interface PackageDetail {
  token: string;
  name: string;
  kind: PackageKind;
  description: string;
  homepage: string;
  version: string;
  installedVersions: string[];
  installed: boolean;
  dependencies: string[];
  commandName: string;
  detectedApplications: DetectedApplication[];
  installLocation?: string | null;
  installedSizeBytes?: number | null;
  localizedDescription?: string | null;
  descriptionSource?: "apple" | "curated" | "original";
}

export interface TranslationResult {
  available: boolean;
  installed: boolean;
  searchInstalled: boolean;
  status: "installed" | "download-required" | "unsupported" | "unavailable" | "error";
  message: string;
  translations?: string[] | null;
}

export interface DetectedApplication {
  name: string;
  path: string;
  version?: string | null;
  bundleId?: string | null;
  source: string;
  brewManaged: boolean;
}

export interface LocalApplication {
  name: string;
  path: string;
  version?: string | null;
  bundleId?: string | null;
}

export interface InstalledInventory {
  homebrew: PackageSummary[];
  other: LocalApplication[];
}

export interface OutdatedPackage {
  token: string;
  name: string;
  kind: PackageKind;
  currentVersion: string;
  latestVersion: string;
}

export interface OutdatedCacheV1 {
  brewPath: string;
  fetchedAt: number;
  items: OutdatedPackage[];
}

export interface BrewAvailability {
  status: "ready" | "missing" | "invalid";
  brewPath?: string | null;
  arch: string;
  recommendedPath: string;
  commandLineToolsAvailable: boolean;
  message: string;
}

export interface HomebrewInstallLaunch {
  started: boolean;
  commandPreview: string;
  networkMode: EffectiveNetworkMode;
}

export interface BrewTarget {
  token: string;
  kind: PackageKind;
}

export interface OperationLog {
  stream: LogStream;
  line: string;
  at: number;
}

export interface BrewOperation {
  id: string;
  action: BrewAction;
  target?: BrewTarget | null;
  commandPreview: string;
  status: OperationStatus;
  phase: OperationPhase;
  phaseLabel: string;
  progress?: number | null;
  currentItem?: string | null;
  networkIssue?: boolean;
  effectiveNetworkMode?: EffectiveNetworkMode;
  proxyUrl?: string | null;
  lastActivityAt?: number;
  transferBytesPerSecond?: number | null;
  downloadedBytes?: number | null;
  totalBytes?: number | null;
  activityLabel?: string | null;
  startedAt: number;
  finishedAt?: number | null;
  exitCode?: number | null;
  logs: OperationLog[];
}

export interface OperationEvent {
  operationId: string;
  status: OperationStatus;
  phase?: OperationPhase | null;
  phaseLabel?: string | null;
  progress?: number | null;
  currentItem?: string | null;
  networkIssue?: boolean | null;
  effectiveNetworkMode?: EffectiveNetworkMode | null;
  proxyUrl?: string | null;
  lastActivityAt?: number | null;
  transferBytesPerSecond?: number | null;
  downloadedBytes?: number | null;
  totalBytes?: number | null;
  activityLabel?: string | null;
  log?: OperationLog | null;
  exitCode?: number | null;
  finishedAt?: number | null;
}
