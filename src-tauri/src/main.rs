use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

const DEFAULT_BREW_PATH: &str = "/opt/homebrew/bin/brew";
const INTEL_BREW_PATH: &str = "/usr/local/bin/brew";
const HOMEBREW_INSTALL_COMMAND: &str =
    "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"";
const HOMEBREW_LATEST_RELEASE_URL: &str = "https://github.com/Homebrew/brew/releases/latest";
const TUNA_BREW_GIT_REMOTE: &str = "https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git";
const TUNA_API_DOMAIN: &str = "https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api";
const TUNA_BOTTLE_DOMAIN: &str = "https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles";
const TUNA_INSTALL_GIT: &str = "https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git";
// Proxied TLS handshakes can legitimately take longer than the former 4 s/6 s
// limits. Keep the probe bounded while avoiding false fallbacks on a slow but
// fully reachable local proxy route.
const NETWORK_PROBE_CONNECT_TIMEOUT_SECONDS: &str = "10";
const NETWORK_PROBE_MAX_TIME_SECONDS: &str = "15";

#[derive(Default)]
struct AppState {
    brew_path: Mutex<String>,
    network_mode: Mutex<NetworkMode>,
    operations: Mutex<HashMap<String, BrewOperation>>,
    operation_queue: Mutex<OperationQueueState>,
    active_processes: Mutex<HashMap<String, u32>>,
    cancel_requested: Mutex<HashSet<String>>,
    search_catalog: Mutex<Option<SearchCatalogCache>>,
}

#[derive(Default)]
struct OperationQueueState {
    worker_running: bool,
    pending: VecDeque<QueuedBrewCommand>,
}

struct QueuedBrewCommand {
    operation_id: String,
    brew_path: String,
    action: BrewAction,
    network_mode: NetworkMode,
    args: Vec<String>,
    target: Option<BrewTarget>,
}

impl OperationQueueState {
    fn enqueue(&mut self, command: QueuedBrewCommand) -> bool {
        self.pending.push_back(command);
        if self.worker_running {
            false
        } else {
            self.worker_running = true;
            true
        }
    }

    fn next(&mut self) -> Option<QueuedBrewCommand> {
        let next = self.pending.pop_front();
        if next.is_none() {
            self.worker_running = false;
        }
        next
    }

    fn remove(&mut self, operation_id: &str) -> bool {
        let Some(index) = self
            .pending
            .iter()
            .position(|command| command.operation_id == operation_id)
        else {
            return false;
        };
        self.pending.remove(index);
        true
    }
}

#[derive(Debug, Clone)]
struct SearchCatalogCache {
    signature: Vec<(PathBuf, Option<SystemTime>, Option<u64>)>,
    items: Arc<Vec<SearchCatalogItem>>,
}

#[derive(Debug, Clone)]
struct SearchCatalogItem {
    token: String,
    aliases: Vec<String>,
    names: Vec<String>,
    description: Option<String>,
    kind: PackageKind,
}

#[derive(Debug, Clone, Deserialize)]
struct CaskCatalogItem {
    token: String,
    #[serde(default)]
    old_tokens: Vec<String>,
    #[serde(default)]
    name: Vec<String>,
    #[serde(default)]
    desc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrewEnvironment {
    brew_path: String,
    version: String,
    prefix: String,
    arch: String,
    ok: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HomebrewUpdateStatus {
    current_version: String,
    latest_version: Option<String>,
    update_available: bool,
    checked: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslationResult {
    available: bool,
    installed: bool,
    #[serde(default)]
    search_installed: bool,
    status: String,
    message: String,
    translations: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageSummary {
    token: String,
    name: String,
    kind: PackageKind,
    description: String,
    installed: bool,
    outdated: bool,
    localized_name: Option<String>,
    version: Option<String>,
    homepage: Option<String>,
    match_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    query: String,
    resolved_query: Option<String>,
    source: SearchSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    items: Vec<PackageSummary>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SearchSource {
    Catalog,
    Alias,
    Brew,
    Translation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrewAvailability {
    status: BrewAvailabilityStatus,
    brew_path: Option<String>,
    arch: String,
    recommended_path: String,
    command_line_tools_available: bool,
    message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum BrewAvailabilityStatus {
    Ready,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HomebrewInstallLaunch {
    started: bool,
    command_preview: String,
    network_mode: EffectiveNetworkMode,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum NetworkMode {
    #[default]
    Auto,
    Official,
    Mirror,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum EffectiveNetworkMode {
    Direct,
    SystemProxy,
    Mirror,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkConfiguration {
    mode: NetworkMode,
    effective_mode: EffectiveNetworkMode,
    proxy_url: Option<String>,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkTestResult {
    reachable: bool,
    effective_mode: EffectiveNetworkMode,
    proxy_url: Option<String>,
    latency_ms: u64,
    message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SystemProxy {
    http_url: Option<String>,
    https_url: Option<String>,
    socks_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageDetail {
    token: String,
    name: String,
    kind: PackageKind,
    description: String,
    homepage: String,
    version: String,
    installed_versions: Vec<String>,
    installed: bool,
    dependencies: Vec<String>,
    command_name: String,
    detected_applications: Vec<DetectedApplication>,
    install_location: Option<String>,
    installed_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetectedApplication {
    name: String,
    path: String,
    version: Option<String>,
    bundle_id: Option<String>,
    source: String,
    brew_managed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalApplication {
    name: String,
    path: String,
    version: Option<String>,
    bundle_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledInventory {
    homebrew: Vec<PackageSummary>,
    other: Vec<LocalApplication>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutdatedPackage {
    token: String,
    name: String,
    kind: PackageKind,
    current_version: String,
    latest_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrewTarget {
    token: String,
    kind: PackageKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrewOperation {
    id: String,
    action: BrewAction,
    target: Option<BrewTarget>,
    command_preview: String,
    status: OperationStatus,
    phase: OperationPhase,
    phase_label: String,
    progress: Option<u8>,
    current_item: Option<String>,
    network_issue: bool,
    effective_network_mode: EffectiveNetworkMode,
    proxy_url: Option<String>,
    last_activity_at: u128,
    transfer_bytes_per_second: Option<u64>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    activity_label: Option<String>,
    started_at: u128,
    finished_at: Option<u128>,
    exit_code: Option<i32>,
    logs: Vec<OperationLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationLog {
    stream: String,
    line: String,
    at: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationEvent {
    operation_id: String,
    status: OperationStatus,
    phase: Option<OperationPhase>,
    phase_label: Option<String>,
    progress: Option<u8>,
    current_item: Option<String>,
    network_issue: Option<bool>,
    effective_network_mode: Option<EffectiveNetworkMode>,
    proxy_url: Option<String>,
    last_activity_at: Option<u128>,
    transfer_bytes_per_second: Option<u64>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    activity_label: Option<String>,
    log: Option<OperationLog>,
    exit_code: Option<i32>,
    finished_at: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
enum PackageKind {
    Formula,
    Cask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SearchKind {
    Formula,
    Cask,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum BrewAction {
    Install,
    Uninstall,
    Upgrade,
    Repair,
    UpgradeAll,
    Update,
    Doctor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum OperationStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum OperationPhase {
    Preparing,
    Resolving,
    Downloading,
    Installing,
    Cleaning,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[derive(Debug, Clone, Default)]
struct DownloadSnapshot {
    total_incomplete_bytes: u64,
    active_file: Option<String>,
    active_file_bytes: Option<u64>,
    newest_modified: Option<SystemTime>,
}

fn display_download_name(file_name: &str) -> String {
    file_name
        .split_once("--")
        .map(|(_, value)| value)
        .unwrap_or(file_name)
        .trim_end_matches(".incomplete")
        .to_string()
}

fn homebrew_download_snapshot(app: &AppHandle) -> DownloadSnapshot {
    let Some(downloads_dir) = app
        .path()
        .home_dir()
        .ok()
        .map(|home| home.join("Library/Caches/Homebrew/downloads"))
    else {
        return DownloadSnapshot::default();
    };
    let Ok(entries) = fs::read_dir(downloads_dir) else {
        return DownloadSnapshot::default();
    };
    let mut snapshot = DownloadSnapshot::default();
    for entry in entries.filter_map(Result::ok) {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".incomplete") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        snapshot.total_incomplete_bytes = snapshot
            .total_incomplete_bytes
            .saturating_add(metadata.len());
        let modified = metadata.modified().ok();
        if modified.is_some() && modified > snapshot.newest_modified {
            snapshot.newest_modified = modified;
            snapshot.active_file = Some(display_download_name(&file_name));
            snapshot.active_file_bytes = Some(metadata.len());
        }
    }
    snapshot
}

fn parse_scutil_proxy(output: &str) -> SystemProxy {
    let mut values = HashMap::new();
    for line in output.lines() {
        if let Some((key, value)) = line.trim().split_once(" : ") {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    let enabled = |key: &str| values.get(key).is_some_and(|value| value == "1");
    let endpoint = |host_key: &str, port_key: &str, scheme: &str| {
        let host = values.get(host_key)?;
        let port = values.get(port_key)?;
        Some(format!("{scheme}://{host}:{port}"))
    };
    SystemProxy {
        http_url: enabled("HTTPEnable")
            .then(|| endpoint("HTTPProxy", "HTTPPort", "http"))
            .flatten(),
        https_url: enabled("HTTPSEnable")
            .then(|| endpoint("HTTPSProxy", "HTTPSPort", "http"))
            .flatten(),
        socks_url: enabled("SOCKSEnable")
            .then(|| endpoint("SOCKSProxy", "SOCKSPort", "socks5h"))
            .flatten(),
    }
}

fn detected_system_proxy() -> SystemProxy {
    Command::new("/usr/sbin/scutil")
        .arg("--proxy")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| parse_scutil_proxy(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default()
}

fn system_proxy_urls(proxy: &SystemProxy) -> Vec<String> {
    let mut urls = Vec::new();
    for url in [
        proxy.https_url.as_ref(),
        proxy.http_url.as_ref(),
        proxy.socks_url.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        let endpoint = url.split_once("://").map(|(_, value)| value).unwrap_or(url);
        if !urls.iter().any(|existing: &String| {
            existing
                .split_once("://")
                .map(|(_, value)| value)
                .unwrap_or(existing)
                == endpoint
        }) {
            urls.push(url.clone());
        }
    }
    urls
}

fn parse_loopback_proxy_ports(output: &str) -> Vec<u16> {
    let proxy_processes = [
        "wandacloudcore",
        "mihomo",
        "clash",
        "sing-box",
        "v2ray",
        "xray",
        "surge",
    ];
    let mut ports = Vec::new();
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        if !lower.contains("listen")
            || !proxy_processes
                .iter()
                .any(|process| lower.contains(process))
        {
            continue;
        }
        let Some(endpoint) = line.split_whitespace().nth(3) else {
            continue;
        };
        let port = endpoint
            .strip_prefix("127.0.0.1.")
            .or_else(|| endpoint.strip_prefix("::1."))
            .and_then(|value| value.parse::<u16>().ok());
        if let Some(port) = port {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }
    ports.sort_unstable();
    ports
}

fn detected_local_proxy_urls() -> Vec<String> {
    Command::new("/usr/sbin/netstat")
        .args(["-anv", "-p", "tcp"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            parse_loopback_proxy_ports(&String::from_utf8_lossy(&output.stdout))
                .into_iter()
                .map(|port| format!("http://127.0.0.1:{port}"))
                .collect()
        })
        .unwrap_or_default()
}

fn discovered_proxy_urls(proxy: &SystemProxy) -> Vec<String> {
    let mut urls = system_proxy_urls(proxy);
    for url in detected_local_proxy_urls() {
        let endpoint = url
            .split_once("://")
            .map(|(_, value)| value)
            .unwrap_or(&url);
        if !urls.iter().any(|existing| {
            existing
                .split_once("://")
                .map(|(_, value)| value)
                .unwrap_or(existing)
                == endpoint
        }) {
            urls.push(url);
        }
    }
    urls
}

fn network_configuration_for_mode(mode: NetworkMode) -> NetworkConfiguration {
    if mode == NetworkMode::Mirror {
        return NetworkConfiguration {
            mode,
            effective_mode: EffectiveNetworkMode::Mirror,
            proxy_url: None,
            message: "备用线路：本次 Homebrew 操作使用清华镜像".to_string(),
        };
    }
    if mode == NetworkMode::Official {
        return NetworkConfiguration {
            mode,
            effective_mode: EffectiveNetworkMode::Direct,
            proxy_url: None,
            message: "官方直连：不使用系统代理或镜像".to_string(),
        };
    }
    let proxy = detected_system_proxy();
    let system_urls = system_proxy_urls(&proxy);
    let proxy_url = system_urls
        .first()
        .cloned()
        .or_else(|| detected_local_proxy_urls().into_iter().next());
    if let Some(proxy_url) = proxy_url {
        let from_system_settings = system_urls.contains(&proxy_url);
        NetworkConfiguration {
            mode,
            effective_mode: EffectiveNetworkMode::SystemProxy,
            proxy_url: Some(proxy_url.clone()),
            message: if from_system_settings {
                format!(
                    "自动模式：已读取系统代理端口 {}",
                    proxy_url.rsplit(':').next().unwrap_or("")
                )
            } else {
                format!(
                    "自动模式：已发现本机代理端口 {}，执行前会验证",
                    proxy_url.rsplit(':').next().unwrap_or("")
                )
            },
        }
    } else {
        NetworkConfiguration {
            mode,
            effective_mode: EffectiveNetworkMode::Direct,
            proxy_url: None,
            message: "自动模式：未检测到系统代理，使用官方直连".to_string(),
        }
    }
}

fn network_configuration_for_effective_mode(
    mode: NetworkMode,
    effective_mode: EffectiveNetworkMode,
    proxy_url: Option<String>,
    message: impl Into<String>,
) -> NetworkConfiguration {
    NetworkConfiguration {
        mode,
        effective_mode,
        proxy_url,
        message: message.into(),
    }
}

fn auto_network_candidates_with_local(
    proxy: SystemProxy,
    local_proxy_urls: Vec<String>,
) -> Vec<NetworkConfiguration> {
    let mut candidates = Vec::new();
    let system_urls = system_proxy_urls(&proxy);
    for proxy_url in system_urls.iter().chain(local_proxy_urls.iter()) {
        let endpoint = proxy_url
            .split_once("://")
            .map(|(_, value)| value)
            .unwrap_or(proxy_url);
        if candidates.iter().any(|candidate: &NetworkConfiguration| {
            candidate.proxy_url.as_ref().is_some_and(|existing| {
                existing
                    .split_once("://")
                    .map(|(_, value)| value)
                    .unwrap_or(existing)
                    == endpoint
            })
        }) {
            continue;
        }
        let from_system_settings = system_urls.contains(proxy_url);
        candidates.push(network_configuration_for_effective_mode(
            NetworkMode::Auto,
            EffectiveNetworkMode::SystemProxy,
            Some(proxy_url.clone()),
            if from_system_settings {
                "稳态自动：系统代理端口健康，已优先使用"
            } else {
                "稳态自动：已发现并接入健康的本机代理端口"
            },
        ));
    }
    candidates.push(network_configuration_for_effective_mode(
        NetworkMode::Auto,
        EffectiveNetworkMode::Direct,
        None,
        "稳态自动：官方直连健康，已自动使用",
    ));
    candidates.push(network_configuration_for_effective_mode(
        NetworkMode::Auto,
        EffectiveNetworkMode::Mirror,
        None,
        "稳态自动：前序线路不可用，已切换清华备用线路",
    ));
    candidates
}

#[cfg(test)]
fn auto_network_candidates(proxy: SystemProxy) -> Vec<NetworkConfiguration> {
    auto_network_candidates_with_local(proxy, Vec::new())
}

fn clear_network_environment(command: &mut Command) {
    for key in [
        "ALL_PROXY",
        "all_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "HOMEBREW_API_DOMAIN",
        "HOMEBREW_ARTIFACT_DOMAIN",
        "HOMEBREW_ARTIFACT_DOMAIN_NO_FALLBACK",
        "HOMEBREW_BOTTLE_DOMAIN",
        "HOMEBREW_BREW_GIT_REMOTE",
        "HOMEBREW_CORE_GIT_REMOTE",
        "HOMEBREW_CURL_RETRIES",
        "HOMEBREW_DOWNLOAD_CONCURRENCY",
    ] {
        command.env_remove(key);
    }
}

fn apply_network_configuration(command: &mut Command, configuration: &NetworkConfiguration) {
    clear_network_environment(command);
    command.env("HOMEBREW_CURL_RETRIES", "5");
    match configuration.effective_mode {
        EffectiveNetworkMode::Direct => {
            command.env("HOMEBREW_DOWNLOAD_CONCURRENCY", "2");
        }
        EffectiveNetworkMode::Mirror => {
            command
                .env("HOMEBREW_API_DOMAIN", TUNA_API_DOMAIN)
                .env("HOMEBREW_BOTTLE_DOMAIN", TUNA_BOTTLE_DOMAIN)
                .env("HOMEBREW_BREW_GIT_REMOTE", TUNA_BREW_GIT_REMOTE)
                .env("HOMEBREW_DOWNLOAD_CONCURRENCY", "1");
        }
        EffectiveNetworkMode::SystemProxy => {
            if let Some(url) = configuration.proxy_url.as_ref() {
                if url.starts_with("socks") {
                    command.env("ALL_PROXY", url).env("all_proxy", url);
                } else {
                    command
                        .env("HTTP_PROXY", url)
                        .env("http_proxy", url)
                        .env("HTTPS_PROXY", url)
                        .env("https_proxy", url);
                }
            }
            command.env("HOMEBREW_DOWNLOAD_CONCURRENCY", "1");
        }
    }
}

fn apply_network_environment(command: &mut Command, mode: NetworkMode) {
    let configuration = network_configuration_for_mode(mode);
    apply_network_configuration(command, &configuration);
}

fn brew_command_for_configuration(
    brew_path: &str,
    configuration: &NetworkConfiguration,
) -> Command {
    let mut command = Command::new(brew_path);
    apply_network_configuration(&mut command, configuration);
    command
}

fn brew_command(brew_path: &str, mode: NetworkMode) -> Command {
    let mut command = Command::new(brew_path);
    apply_network_environment(&mut command, mode);
    command
}

fn brew_read_command(brew_path: &str, mode: NetworkMode) -> Command {
    let mut command = brew_command(brew_path, mode);
    command.env("HOMEBREW_NO_AUTO_UPDATE", "1");
    command
}

fn command_output(brew_path: &str, args: &[&str], mode: NetworkMode) -> Result<String, String> {
    let output = brew_read_command(brew_path, mode)
        .args(args)
        .output()
        .map_err(|err| format!("无法执行 brew：{err}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if err.is_empty() {
            format!("brew {:?} 执行失败", args)
        } else {
            err
        })
    }
}

fn current_brew_path(state: &State<AppState>) -> String {
    state
        .brew_path
        .lock()
        .map(|path| {
            if path.is_empty() {
                DEFAULT_BREW_PATH.to_string()
            } else {
                path.clone()
            }
        })
        .unwrap_or_else(|_| DEFAULT_BREW_PATH.to_string())
}

fn current_network_mode(state: &State<AppState>) -> NetworkMode {
    state
        .network_mode
        .lock()
        .map(|mode| *mode)
        .unwrap_or_default()
}

fn validate_token(token: &str) -> Result<(), String> {
    if token.is_empty() || token.len() > 180 {
        return Err("包名为空或过长".to_string());
    }
    let ok = token
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '+' | '@' | '/'));
    if ok {
        Ok(())
    } else {
        Err("包名包含不允许的字符".to_string())
    }
}

fn validate_search_text(query: &str) -> Result<(), String> {
    let length = query.chars().count();
    if !(2..=100).contains(&length) {
        return Err("搜索词需要 2 到 100 个字符".to_string());
    }
    if query.chars().any(char::is_control) {
        return Err("搜索词包含不允许的控制字符".to_string());
    }
    Ok(())
}

fn contains_han(value: &str) -> bool {
    value.chars().any(|ch| {
        matches!(
            ch as u32,
            0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
        )
    })
}

fn normalize_search_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct ChineseAlias {
    resolved_query: &'static str,
    tokens: &'static [&'static str],
}

fn chinese_alias(query: &str) -> Option<ChineseAlias> {
    let normalized = normalize_search_text(query);
    let alias = match normalized.as_str() {
        "安卓" | "android" => ChineseAlias {
            resolved_query: "android",
            tokens: &[
                "android-studio",
                "android-platform-tools",
                "android-commandlinetools",
                "android-file-transfer",
                "android-ndk",
            ],
        },
        "安卓工具" | "安卓调试工具" | "adb" | "fastboot" => ChineseAlias {
            resolved_query: "android",
            tokens: &[
                "android-platform-tools",
                "android-commandlinetools",
                "android-studio",
                "android-ndk",
            ],
        },
        "安卓开发" | "安卓开发工具" | "android开发" => ChineseAlias {
            resolved_query: "android",
            tokens: &[
                "android-studio",
                "android-commandlinetools",
                "android-platform-tools",
                "android-ndk",
            ],
        },
        "微信" | "微信电脑版" | "微信mac版" => ChineseAlias {
            resolved_query: "wechat",
            tokens: &["wechat"],
        },
        "谷歌浏览器" | "chrome浏览器" | "谷歌" => ChineseAlias {
            resolved_query: "google-chrome",
            tokens: &["google-chrome"],
        },
        "火狐" | "火狐浏览器" => ChineseAlias {
            resolved_query: "firefox",
            tokens: &["firefox"],
        },
        "浏览器" | "网页浏览器" => ChineseAlias {
            resolved_query: "google-chrome",
            tokens: &[
                "google-chrome",
                "firefox",
                "brave-browser",
                "arc",
                "vivaldi",
            ],
        },
        "解压" | "解压软件" | "压缩软件" => ChineseAlias {
            resolved_query: "keka",
            tokens: &["keka", "the-unarchiver", "unar", "sevenzip"],
        },
        "代码编辑器" | "编程软件" => ChineseAlias {
            resolved_query: "visual-studio-code",
            tokens: &["visual-studio-code", "zed", "sublime-text"],
        },
        "终端" | "终端工具" => ChineseAlias {
            resolved_query: "iterm2",
            tokens: &["iterm2", "warp", "ghostty"],
        },
        "视频播放器" | "播放器" => ChineseAlias {
            resolved_query: "iina",
            tokens: &["iina", "vlc"],
        },
        "音乐播放器" | "音乐软件" | "听歌" => ChineseAlias {
            resolved_query: "qqmusic",
            tokens: &["qqmusic", "neteasemusic", "spotify", "vox"],
        },
        "截图" | "截图软件" => ChineseAlias {
            resolved_query: "shottr",
            tokens: &["shottr", "ishot", "xnip"],
        },
        "钉钉" => ChineseAlias {
            resolved_query: "dingtalk",
            tokens: &["dingtalk"],
        },
        "飞书" => ChineseAlias {
            resolved_query: "feishu",
            tokens: &["feishu"],
        },
        "腾讯会议" => ChineseAlias {
            resolved_query: "tencent-meeting",
            tokens: &["tencent-meeting"],
        },
        "网易云音乐" => ChineseAlias {
            resolved_query: "neteasemusic",
            tokens: &["neteasemusic"],
        },
        "网盘" | "云盘" => ChineseAlias {
            resolved_query: "baidunetdisk",
            tokens: &["baidunetdisk", "aliyunpan"],
        },
        "密码管理" | "密码管理器" => ChineseAlias {
            resolved_query: "1password",
            tokens: &["1password", "bitwarden", "keepassxc"],
        },
        "数据库" | "数据库客户端" => ChineseAlias {
            resolved_query: "tableplus",
            tokens: &["tableplus", "dbeaver-community", "sequel-ace", "datagrip"],
        },
        "api调试" | "接口调试" => ChineseAlias {
            resolved_query: "postman",
            tokens: &["postman", "insomnia", "bruno"],
        },
        "办公软件" | "办公套件" => ChineseAlias {
            resolved_query: "libreoffice",
            tokens: &["libreoffice", "wpsoffice-cn", "onlyoffice"],
        },
        "vpn" | "代理软件" | "网络代理" => ChineseAlias {
            resolved_query: "clash-verge-rev",
            tokens: &["clash-verge-rev", "surge", "v2rayu"],
        },
        "图片编辑" | "修图软件" => ChineseAlias {
            resolved_query: "gimp",
            tokens: &["gimp", "krita", "pixelmator-pro"],
        },
        "输入法" | "搜狗输入法" => ChineseAlias {
            resolved_query: "sogouinput",
            tokens: &["sogouinput"],
        },
        _ => return None,
    };
    Some(alias)
}

fn package_name_from_token(token: &str) -> String {
    token
        .rsplit('/')
        .next()
        .unwrap_or(token)
        .replace('-', " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_search_output(output: &str, kind: PackageKind) -> Vec<PackageSummary> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("==>"))
        .map(|token| PackageSummary {
            token: token.to_string(),
            name: package_name_from_token(token),
            kind: kind.clone(),
            description: "点击查看详情".to_string(),
            installed: false,
            outdated: false,
            localized_name: None,
            version: None,
            homepage: None,
            match_reason: None,
        })
        .collect()
}

fn parse_cask_catalog(raw: &str) -> Result<Vec<CaskCatalogItem>, String> {
    let envelope: Value = serde_json::from_str(raw)
        .map_err(|err| format!("Homebrew cask 目录外层解析失败：{err}"))?;
    let payload = envelope
        .get("payload")
        .and_then(Value::as_str)
        .ok_or_else(|| "Homebrew cask 目录缺少 payload".to_string())?;
    serde_json::from_str::<Vec<CaskCatalogItem>>(payload)
        .map_err(|err| format!("Homebrew cask 目录解析失败：{err}"))
}

fn homebrew_search_catalog_paths(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
    let home = app
        .path()
        .home_dir()
        .map_err(|_| "无法定位 Homebrew 目录缓存".to_string())?;
    let cache = home.join("Library/Caches/Homebrew");
    Ok(vec![
        cache.join("api/cask_names.txt"),
        cache.join("cask_descriptions.json"),
        cache.join("api/formula_names.txt"),
        cache.join("api/formula_aliases.txt"),
        cache.join("api/cask.jws.json"),
    ])
}

fn search_catalog_signature(paths: &[PathBuf]) -> Vec<(PathBuf, Option<SystemTime>, Option<u64>)> {
    paths
        .iter()
        .map(|path| {
            let metadata = fs::metadata(path).ok();
            (
                path.clone(),
                metadata.as_ref().and_then(|value| value.modified().ok()),
                metadata.map(|value| value.len()),
            )
        })
        .collect()
}

fn nonempty_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn cask_description_fields(value: &Value) -> (Vec<String>, Option<String>) {
    let fields = value.as_array();
    let display_name = fields
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut names = display_name
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if names.is_empty() && !display_name.is_empty() {
        names.push(display_name.to_string());
    }
    let description = fields
        .and_then(|items| items.get(1))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(str::to_string);
    (names, description)
}

fn build_search_catalog(paths: &[PathBuf]) -> Result<Vec<SearchCatalogItem>, String> {
    let mut items = HashMap::<(PackageKind, String), SearchCatalogItem>::new();
    let cask_names = fs::read_to_string(&paths[0])
        .map(|raw| nonempty_lines(&raw))
        .unwrap_or_default();
    let descriptions = fs::read_to_string(&paths[1])
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    for token in cask_names.into_iter().chain(descriptions.keys().cloned()) {
        let key = (PackageKind::Cask, token.clone());
        items.entry(key).or_insert_with(|| {
            let (names, description) = descriptions
                .get(&token)
                .map(cask_description_fields)
                .unwrap_or_default();
            SearchCatalogItem {
                token,
                aliases: Vec::new(),
                names,
                description,
                kind: PackageKind::Cask,
            }
        });
    }

    if let Ok(raw) = fs::read_to_string(&paths[4]) {
        if let Ok(legacy) = parse_cask_catalog(&raw) {
            for item in legacy {
                let key = (PackageKind::Cask, item.token.clone());
                let entry = items.entry(key).or_insert_with(|| SearchCatalogItem {
                    token: item.token.clone(),
                    aliases: Vec::new(),
                    names: Vec::new(),
                    description: None,
                    kind: PackageKind::Cask,
                });
                entry.aliases.extend(item.old_tokens);
                if entry.names.is_empty() {
                    entry.names = item.name;
                }
                if entry.description.is_none() {
                    entry.description = item.desc;
                }
            }
        }
    }

    let formula_names = fs::read_to_string(&paths[2])
        .map(|raw| nonempty_lines(&raw))
        .unwrap_or_default();
    for token in formula_names {
        let key = (PackageKind::Formula, token.clone());
        items.entry(key).or_insert_with(|| SearchCatalogItem {
            names: vec![package_name_from_token(&token)],
            token,
            aliases: Vec::new(),
            description: None,
            kind: PackageKind::Formula,
        });
    }
    if let Ok(raw) = fs::read_to_string(&paths[3]) {
        for line in nonempty_lines(&raw) {
            let Some((alias, token)) = line.split_once('|') else {
                continue;
            };
            let token = token.trim().to_string();
            let key = (PackageKind::Formula, token.clone());
            items
                .entry(key)
                .or_insert_with(|| SearchCatalogItem {
                    names: vec![package_name_from_token(&token)],
                    token,
                    aliases: Vec::new(),
                    description: None,
                    kind: PackageKind::Formula,
                })
                .aliases
                .push(alias.trim().to_string());
        }
    }

    if items.is_empty() {
        return Err("本机 Homebrew 搜索目录尚未生成".to_string());
    }
    let mut items = items.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| (&left.kind, &left.token).cmp(&(&right.kind, &right.token)));
    Ok(items)
}

fn load_search_catalog(app: &AppHandle) -> Result<Arc<Vec<SearchCatalogItem>>, String> {
    let paths = homebrew_search_catalog_paths(app)?;
    let signature = search_catalog_signature(&paths);
    let state = app.state::<AppState>();
    if let Ok(cache) = state.search_catalog.lock() {
        if let Some(cache) = cache.as_ref() {
            if cache.signature == signature {
                return Ok(cache.items.clone());
            }
        }
    }
    let items = Arc::new(build_search_catalog(&paths)?);
    if let Ok(mut cache) = state.search_catalog.lock() {
        *cache = Some(SearchCatalogCache {
            signature,
            items: items.clone(),
        });
    }
    Ok(items)
}

fn catalog_kind_matches(kind: &SearchKind, item_kind: &PackageKind) -> bool {
    matches!(kind, SearchKind::Both)
        || matches!(
            (kind, item_kind),
            (SearchKind::Formula, PackageKind::Formula)
        )
        || matches!((kind, item_kind), (SearchKind::Cask, PackageKind::Cask))
}

fn search_terms(value: &str) -> Vec<String> {
    const GENERIC: &[&str] = &[
        "app",
        "application",
        "software",
        "tool",
        "tools",
        "the",
        "for",
    ];
    let mut terms = value
        .split(|character: char| {
            !(character.is_alphanumeric() || character == '-' || character == '+')
        })
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
        .filter(|term| !GENERIC.contains(&term.as_str()))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        let normalized = normalize_search_text(value);
        if !normalized.is_empty() {
            terms.push(normalized);
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

fn summary_from_search_catalog(item: &SearchCatalogItem, reason: &str) -> PackageSummary {
    PackageSummary {
        token: item.token.clone(),
        name: item
            .names
            .first()
            .cloned()
            .unwrap_or_else(|| package_name_from_token(&item.token)),
        kind: item.kind.clone(),
        description: item
            .description
            .clone()
            .unwrap_or_else(|| "点击查看详情".to_string()),
        installed: false,
        outdated: false,
        localized_name: item.names.iter().find(|name| contains_han(name)).cloned(),
        version: None,
        homepage: None,
        match_reason: Some(reason.to_string()),
    }
}

fn search_local_catalog(
    catalog: &[SearchCatalogItem],
    query: &str,
    alias: Option<ChineseAlias>,
    kind: &SearchKind,
) -> Vec<PackageSummary> {
    let normalized_query = normalize_search_text(query);
    let terms = search_terms(alias.map(|value| value.resolved_query).unwrap_or(query));
    let preferred_tokens = alias.map(|value| value.tokens).unwrap_or_default();
    let mut matches = catalog
        .iter()
        .filter(|item| catalog_kind_matches(kind, &item.kind))
        .filter_map(|item| {
            let token = item.token.to_lowercase();
            let aliases = item
                .aliases
                .iter()
                .map(|value| value.to_lowercase())
                .collect::<Vec<_>>();
            let names = item
                .names
                .iter()
                .map(|value| value.to_lowercase())
                .collect::<Vec<_>>();
            let normalized_names = item
                .names
                .iter()
                .map(|value| normalize_search_text(value))
                .collect::<Vec<_>>();
            let description = item
                .description
                .as_deref()
                .unwrap_or_default()
                .to_lowercase();
            let preferred = preferred_tokens
                .iter()
                .position(|value| *value == item.token);
            let direct_token = token == query.to_lowercase();
            let term_prefix = terms.iter().any(|term| {
                token.starts_with(term)
                    || aliases.iter().any(|value| value.starts_with(term))
                    || names.iter().any(|value| value.starts_with(term))
            });
            let term_contains = terms.iter().any(|term| {
                token.contains(term)
                    || aliases.iter().any(|value| value.contains(term))
                    || names.iter().any(|value| value.contains(term))
            });
            let description_match = terms.iter().any(|term| description.contains(term));
            let (rank, reason) = if normalized_names
                .iter()
                .any(|name| name == &normalized_query)
            {
                (0_u16, "中文名称精确匹配")
            } else if let Some(position) = preferred {
                (10 + position as u16, "中文意图匹配")
            } else if direct_token {
                (25, "包名精确匹配")
            } else if term_prefix {
                (40, "名称前缀匹配")
            } else if term_contains {
                (60, "名称包含匹配")
            } else if description_match {
                (80, "描述匹配")
            } else {
                return None;
            };
            Some((
                rank,
                item.token.clone(),
                summary_from_search_catalog(item, reason),
            ))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
    let mut seen = HashSet::new();
    matches
        .into_iter()
        .filter_map(|(_, _, item)| {
            let key = (item.kind.clone(), item.token.clone());
            seen.insert(key).then_some(item)
        })
        .take(50)
        .collect()
}

fn enrich_search_results(results: &mut [PackageSummary], catalog: &[SearchCatalogItem]) {
    let by_key = catalog
        .iter()
        .map(|item| ((item.kind.clone(), item.token.as_str()), item))
        .collect::<HashMap<_, _>>();
    for result in results {
        let Some(item) = by_key.get(&(result.kind.clone(), result.token.as_str())) else {
            continue;
        };
        if let Some(name) = item.names.first() {
            result.name = name.clone();
        }
        if let Some(description) = item.description.as_ref() {
            result.description = description.clone();
        }
        result.localized_name = item.names.iter().find(|name| contains_han(name)).cloned();
    }
}

fn merge_search_results(
    primary: Vec<PackageSummary>,
    additional: Vec<PackageSummary>,
) -> Vec<PackageSummary> {
    let mut seen = HashSet::new();
    primary
        .into_iter()
        .chain(additional)
        .filter(|item| seen.insert((item.kind.clone(), item.token.clone())))
        .take(50)
        .collect()
}

fn plist_value(app_path: &Path, key: &str) -> Option<String> {
    let plist = app_path.join("Contents/Info.plist");
    if !plist.is_file() {
        return None;
    }
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(&plist)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty() && value != "null").then_some(value)
}

fn cask_app_candidates(item: &Value, home: Option<&Path>) -> Vec<(PathBuf, bool)> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let Some(artifacts) = item.get("artifacts").and_then(Value::as_array) else {
        return candidates;
    };

    for artifact in artifacts {
        let Some(app_values) = artifact.get("app").and_then(Value::as_array) else {
            continue;
        };
        let Some(source_name) = app_values.first().and_then(Value::as_str) else {
            continue;
        };
        let target_name = app_values
            .get(1)
            .and_then(Value::as_object)
            .and_then(|options| options.get("target"))
            .and_then(Value::as_str)
            .unwrap_or(source_name);
        let primary = artifact
            .get("target")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/Applications").join(target_name));

        if seen.insert(primary.clone()) {
            candidates.push((primary, true));
        }
        let system_candidate = PathBuf::from("/Applications").join(target_name);
        if seen.insert(system_candidate.clone()) {
            candidates.push((system_candidate, false));
        }
        if let Some(home) = home {
            let user_candidate = home.join("Applications").join(target_name);
            if seen.insert(user_candidate.clone()) {
                candidates.push((user_candidate, false));
            }
        }
    }
    candidates
}

fn detect_cask_applications(
    json: &str,
    brew_installed: bool,
) -> Result<Vec<DetectedApplication>, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|err| format!("cask 应用检测解析失败：{err}"))?;
    let item = value
        .get("casks")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| "没有找到 cask 信息".to_string())?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut detected = Vec::new();

    for (path, primary_target) in cask_app_candidates(item, home.as_deref()) {
        if !path.is_dir() {
            continue;
        }
        let brew_managed = brew_installed && primary_target;
        let source = if brew_managed {
            "Homebrew"
        } else if path.join("Contents/_MASReceipt/receipt").is_file() {
            "Mac App Store"
        } else {
            "其他渠道或来源未知"
        };
        let version = plist_value(&path, "CFBundleShortVersionString")
            .or_else(|| plist_value(&path, "CFBundleVersion"));
        detected.push(DetectedApplication {
            name: path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("应用")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            version,
            bundle_id: plist_value(&path, "CFBundleIdentifier"),
            source: source.to_string(),
            brew_managed,
        });
    }
    Ok(detected)
}

fn collect_application_paths(root: &Path, remaining_depth: u8, apps: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("app") {
            apps.push(path);
            continue;
        }
        if remaining_depth > 0
            && !entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with('.'))
        {
            collect_application_paths(&path, remaining_depth - 1, apps);
        }
    }
}

fn managed_cask_application_paths(value: &Value, home: Option<&Path>) -> HashSet<PathBuf> {
    value
        .get("casks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|item| cask_app_candidates(item, home))
        .filter_map(|(path, primary)| primary.then_some(path))
        .collect()
}

fn is_mac_app_store_application(path: &Path) -> bool {
    path.join("Contents/_MASReceipt/receipt").is_file()
}

fn scan_other_applications(value: &Value) -> Vec<LocalApplication> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let managed_paths = managed_cask_application_paths(value, home.as_deref());
    let mut app_paths = Vec::new();
    collect_application_paths(Path::new("/Applications"), 2, &mut app_paths);
    if let Some(home) = home.as_deref() {
        collect_application_paths(&home.join("Applications"), 2, &mut app_paths);
    }

    app_paths.sort();
    app_paths.dedup();
    let mut applications = app_paths
        .into_iter()
        .filter(|path| !managed_paths.contains(path))
        .filter(|path| !is_mac_app_store_application(path))
        .map(|path| LocalApplication {
            name: plist_value(&path, "CFBundleDisplayName")
                .or_else(|| plist_value(&path, "CFBundleName"))
                .or_else(|| {
                    path.file_stem()
                        .and_then(|name| name.to_str())
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| "未命名应用".to_string()),
            version: plist_value(&path, "CFBundleShortVersionString")
                .or_else(|| plist_value(&path, "CFBundleVersion")),
            bundle_id: plist_value(&path, "CFBundleIdentifier"),
            path: path.to_string_lossy().to_string(),
        })
        .collect::<Vec<_>>();
    applications.sort_by_key(|app| app.name.to_lowercase());
    applications
}

fn parse_info_detail(json: &str, kind: PackageKind) -> Result<PackageDetail, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|err| format!("brew info JSON 解析失败：{err}"))?;
    match kind {
        PackageKind::Formula => {
            let item = value
                .get("formulae")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .ok_or_else(|| "没有找到 formula 信息".to_string())?;
            let token = item.get("name").and_then(Value::as_str).unwrap_or_default();
            let version = item
                .get("versions")
                .and_then(|versions| versions.get("stable"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let installed_versions = item
                .get("installed")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|installed| installed.get("version").and_then(Value::as_str))
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let dependencies = item
                .get("dependencies")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_default();
            Ok(PackageDetail {
                token: token.to_string(),
                name: package_name_from_token(token),
                kind,
                description: item
                    .get("desc")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                homepage: item
                    .get("homepage")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                version: version.to_string(),
                installed: !installed_versions.is_empty(),
                installed_versions,
                dependencies,
                command_name: token.to_string(),
                detected_applications: Vec::new(),
                install_location: None,
                installed_size_bytes: None,
            })
        }
        PackageKind::Cask => {
            let item = value
                .get("casks")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .ok_or_else(|| "没有找到 cask 信息".to_string())?;
            let token = item
                .get("token")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let name = item
                .get("name")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str)
                .unwrap_or(token);
            let installed_versions = item
                .get("installed")
                .and_then(Value::as_str)
                .filter(|installed| !installed.is_empty())
                .map(|installed| vec![installed.to_string()])
                .unwrap_or_default();
            Ok(PackageDetail {
                token: token.to_string(),
                name: name.to_string(),
                kind,
                description: item
                    .get("desc")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                homepage: item
                    .get("homepage")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                version: item
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                installed: !installed_versions.is_empty(),
                installed_versions,
                dependencies: Vec::new(),
                command_name: token.to_string(),
                detected_applications: Vec::new(),
                install_location: None,
                installed_size_bytes: None,
            })
        }
    }
}

fn command_for_action(
    action: &BrewAction,
    target: Option<&BrewTarget>,
) -> Result<Vec<String>, String> {
    match action {
        BrewAction::Install => {
            let target = target.ok_or_else(|| "安装需要指定包".to_string())?;
            validate_token(&target.token)?;
            let mut args = vec!["install".to_string()];
            if target.kind == PackageKind::Cask {
                args.push("--cask".to_string());
            }
            args.push(target.token.clone());
            Ok(args)
        }
        BrewAction::Uninstall => {
            let target = target.ok_or_else(|| "卸载需要指定包".to_string())?;
            validate_token(&target.token)?;
            let mut args = vec!["uninstall".to_string()];
            if target.kind == PackageKind::Cask {
                args.push("--cask".to_string());
            }
            args.push(target.token.clone());
            Ok(args)
        }
        BrewAction::Upgrade => {
            let target = target.ok_or_else(|| "更新需要指定项目".to_string())?;
            validate_token(&target.token)?;
            let mut args = vec!["upgrade".to_string()];
            if target.kind == PackageKind::Cask {
                args.push("--cask".to_string());
            }
            args.push(target.token.clone());
            Ok(args)
        }
        BrewAction::Repair => {
            let target = target.ok_or_else(|| "修复安装需要指定项目".to_string())?;
            validate_token(&target.token)?;
            if target.kind != PackageKind::Cask {
                return Err("修复安装仅适用于 cask 应用".to_string());
            }
            Ok(vec![
                "reinstall".to_string(),
                "--cask".to_string(),
                "--force".to_string(),
                target.token.clone(),
            ])
        }
        BrewAction::UpgradeAll => Ok(vec!["upgrade".to_string()]),
        BrewAction::Update => Ok(vec!["update".to_string()]),
        BrewAction::Doctor => Ok(vec!["doctor".to_string()]),
    }
}

fn command_preview(brew_path: &str, args: &[String]) -> String {
    std::iter::once(brew_path.to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ")
}

#[tauri::command]
fn network_configuration(state: State<AppState>) -> NetworkConfiguration {
    network_configuration_for_mode(current_network_mode(&state))
}

fn network_mode_label(mode: EffectiveNetworkMode) -> &'static str {
    match mode {
        EffectiveNetworkMode::Direct => "官方直连",
        EffectiveNetworkMode::SystemProxy => "系统代理",
        EffectiveNetworkMode::Mirror => "备用线路",
    }
}

fn connection_failure_reason(stderr: &str) -> &'static str {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        "连接超时"
    } else if lower.contains("could not resolve") {
        "域名解析失败"
    } else if lower.contains("failed to connect") || lower.contains("connection refused") {
        "无法建立连接"
    } else if lower.contains("ssl") || lower.contains("certificate") {
        "安全连接失败"
    } else {
        "连接失败"
    }
}

#[derive(Debug, Clone)]
struct NetworkProbe {
    reachable: bool,
    latency_ms: u64,
    covered_endpoints: usize,
    total_endpoints: usize,
    failure_reason: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct NetworkProbeTarget {
    label: &'static str,
    url: &'static str,
    head_only: bool,
}

fn network_probe_targets(configuration: &NetworkConfiguration) -> Vec<NetworkProbeTarget> {
    match configuration.effective_mode {
        EffectiveNetworkMode::Mirror => vec![
            NetworkProbeTarget {
                label: "镜像 API",
                url: "https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api/formula.jws.json",
                head_only: true,
            },
            NetworkProbeTarget {
                label: "镜像 bottle",
                url: "https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/",
                head_only: true,
            },
            NetworkProbeTarget {
                label: "镜像 Git",
                url: "https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git/",
                head_only: true,
            },
        ],
        EffectiveNetworkMode::Direct | EffectiveNetworkMode::SystemProxy => vec![
            NetworkProbeTarget {
                label: "Homebrew API",
                url: "https://formulae.brew.sh/api/formula.jws.json",
                head_only: true,
            },
            NetworkProbeTarget {
                label: "GitHub",
                url: "https://github.com/Homebrew/brew",
                head_only: true,
            },
            NetworkProbeTarget {
                label: "GHCR",
                url: "https://ghcr.io/v2/",
                head_only: false,
            },
        ],
    }
}

fn probe_network_target(
    configuration: &NetworkConfiguration,
    target: NetworkProbeTarget,
) -> Result<u64, String> {
    let mut command = Command::new("/usr/bin/curl");
    apply_network_configuration(&mut command, configuration);
    let started = Instant::now();
    command.args([
        "--silent",
        "--show-error",
        "--output",
        "/dev/null",
        "--connect-timeout",
        NETWORK_PROBE_CONNECT_TIMEOUT_SECONDS,
        "--max-time",
        NETWORK_PROBE_MAX_TIME_SECONDS,
    ]);
    if target.head_only {
        command.arg("--head");
    }
    let output = command.arg(target.url).output();
    let latency_ms = started.elapsed().as_millis().max(1) as u64;
    match output {
        Ok(output) if output.status.success() => Ok(latency_ms),
        Ok(output) => {
            Err(connection_failure_reason(&String::from_utf8_lossy(&output.stderr)).to_string())
        }
        Err(_) => Err("测试无法启动".to_string()),
    }
}

fn probe_network_configuration(configuration: &NetworkConfiguration) -> NetworkProbe {
    let targets = network_probe_targets(configuration);
    let started = Instant::now();
    let results = std::thread::scope(|scope| {
        let handles = targets
            .iter()
            .copied()
            .map(|target| {
                let handle = scope.spawn(move || probe_network_target(configuration, target));
                (target, handle)
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|(target, handle)| {
                (
                    target,
                    handle
                        .join()
                        .unwrap_or_else(|_| Err("测试线程异常".to_string())),
                )
            })
            .collect::<Vec<_>>()
    });
    let covered_endpoints = results.iter().filter(|(_, result)| result.is_ok()).count();
    let total_endpoints = results.len();
    let failures = results
        .iter()
        .filter_map(|(target, result)| {
            result
                .as_ref()
                .err()
                .map(|reason| format!("{}{}", target.label, reason))
        })
        .collect::<Vec<_>>();
    NetworkProbe {
        reachable: covered_endpoints == total_endpoints,
        latency_ms: started.elapsed().as_millis().max(1) as u64,
        covered_endpoints,
        total_endpoints,
        failure_reason: (!failures.is_empty()).then(|| failures.join("、")),
    }
}

fn select_automatic_network_configuration() -> Result<(NetworkConfiguration, NetworkProbe), String>
{
    let system_proxy = detected_system_proxy();
    let proxy_urls = discovered_proxy_urls(&system_proxy);
    let candidates = auto_network_candidates_with_local(system_proxy, proxy_urls);
    let results = std::thread::scope(|scope| {
        let handles = candidates
            .into_iter()
            .map(|configuration| {
                let probe_configuration = configuration.clone();
                let handle = scope.spawn(move || probe_network_configuration(&probe_configuration));
                (configuration, handle)
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|(configuration, handle)| {
                let probe = handle.join().unwrap_or(NetworkProbe {
                    reachable: false,
                    latency_ms: 0,
                    covered_endpoints: 0,
                    total_endpoints: 3,
                    failure_reason: Some("测试线程异常".to_string()),
                });
                (configuration, probe)
            })
            .collect::<Vec<_>>()
    });
    let mut failures = Vec::new();
    for (configuration, probe) in results {
        if probe.reachable {
            return Ok((configuration, probe));
        }
        failures.push(format!(
            "{}：{}",
            network_mode_label(configuration.effective_mode),
            probe.failure_reason.as_deref().unwrap_or("连接失败")
        ));
    }
    Err(format!("自动选路失败（{}）", failures.join("；")))
}

fn test_network_connection_for_mode(mode: NetworkMode) -> NetworkTestResult {
    if mode == NetworkMode::Auto {
        return match select_automatic_network_configuration() {
            Ok((configuration, probe)) => NetworkTestResult {
                reachable: true,
                effective_mode: configuration.effective_mode,
                proxy_url: configuration.proxy_url,
                latency_ms: probe.latency_ms,
                message: format!(
                    "{} · 核心链路 {}/{} · {} ms",
                    configuration.message,
                    probe.covered_endpoints,
                    probe.total_endpoints,
                    probe.latency_ms
                ),
            },
            Err(message) => NetworkTestResult {
                reachable: false,
                effective_mode: EffectiveNetworkMode::Direct,
                proxy_url: None,
                latency_ms: 0,
                message,
            },
        };
    }
    let configuration = network_configuration_for_mode(mode);
    let probe = probe_network_configuration(&configuration);
    let latency_ms = probe.latency_ms;
    let label = network_mode_label(configuration.effective_mode);
    if probe.reachable {
        let speed_note = if latency_ms > 5_000 {
            "可达，但响应很慢"
        } else if latency_ms > 1_500 {
            "可达，但响应偏慢"
        } else {
            "可达"
        };
        NetworkTestResult {
            reachable: true,
            effective_mode: configuration.effective_mode,
            proxy_url: configuration.proxy_url,
            latency_ms,
            message: format!(
                "{label}{speed_note} · 核心链路 {}/{} · {latency_ms} ms",
                probe.covered_endpoints, probe.total_endpoints
            ),
        }
    } else {
        NetworkTestResult {
            reachable: false,
            effective_mode: configuration.effective_mode,
            proxy_url: configuration.proxy_url,
            latency_ms,
            message: format!(
                "{label}不可用 · {} · {latency_ms} ms",
                probe.failure_reason.as_deref().unwrap_or("连接失败")
            ),
        }
    }
}

#[tauri::command]
async fn test_network_connection(state: State<'_, AppState>) -> Result<NetworkTestResult, String> {
    let mode = current_network_mode(&state);
    tauri::async_runtime::spawn_blocking(move || test_network_connection_for_mode(mode))
        .await
        .map_err(|err| format!("线路测试线程异常：{err}"))
}

#[tauri::command]
fn set_network_mode(
    mode: NetworkMode,
    state: State<AppState>,
) -> Result<NetworkConfiguration, String> {
    *state
        .network_mode
        .lock()
        .map_err(|_| "无法保存下载线路".to_string())? = mode;
    Ok(network_configuration_for_mode(mode))
}

#[tauri::command]
fn set_brew_path(path: String, state: State<AppState>) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("brew 路径不能为空".to_string());
    }
    let mut brew_path = state
        .brew_path
        .lock()
        .map_err(|_| "无法保存 brew 路径".to_string())?;
    *brew_path = path;
    if let Ok(mut cache) = state.search_catalog.lock() {
        *cache = None;
    }
    Ok(())
}

fn brew_environment_for_path(brew_path: String, mode: NetworkMode) -> BrewEnvironment {
    let version = command_output(&brew_path, &["--version"], mode)
        .map(|output| output.lines().next().unwrap_or_default().to_string());
    let prefix = command_output(&brew_path, &["--prefix"], mode);
    let arch = std::env::consts::ARCH.to_string();

    match (version, prefix) {
        (Ok(version), Ok(prefix)) => BrewEnvironment {
            brew_path,
            version,
            prefix,
            arch,
            ok: true,
            message: "Homebrew 可用".to_string(),
        },
        (version, prefix) => BrewEnvironment {
            brew_path,
            version: version.unwrap_or_else(|err| err),
            prefix: prefix.unwrap_or_default(),
            arch,
            ok: false,
            message: "无法检测 Homebrew，请检查路径".to_string(),
        },
    }
}

#[tauri::command]
async fn brew_environment(app: AppHandle) -> BrewEnvironment {
    let state = app.state::<AppState>();
    let brew_path = current_brew_path(&state);
    let mode = current_network_mode(&state);
    tauri::async_runtime::spawn_blocking(move || brew_environment_for_path(brew_path, mode))
        .await
        .unwrap_or_else(|err| BrewEnvironment {
            brew_path: String::new(),
            version: String::new(),
            prefix: String::new(),
            arch: std::env::consts::ARCH.to_string(),
            ok: false,
            message: format!("Homebrew 检测线程异常：{err}"),
        })
}

fn parse_homebrew_version(value: &str) -> Option<((u64, u64, u64), String)> {
    let start = value
        .char_indices()
        .find_map(|(index, character)| character.is_ascii_digit().then_some(index))?;
    let numeric = value[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect::<String>();
    let mut components = numeric
        .trim_end_matches('.')
        .split('.')
        .map(str::parse::<u64>);
    let major = components.next()?.ok()?;
    let minor = components.next()?.ok()?;
    let patch = components.next().transpose().ok()?.unwrap_or(0);
    Some(((major, minor, patch), format!("{major}.{minor}.{patch}")))
}

fn homebrew_update_status_for_path(brew_path: String, mode: NetworkMode) -> HomebrewUpdateStatus {
    let current_output = command_output(&brew_path, &["--version"], mode)
        .map(|output| output.lines().next().unwrap_or_default().to_string());
    let Ok(current_output) = current_output else {
        return HomebrewUpdateStatus {
            current_version: String::new(),
            latest_version: None,
            update_available: false,
            checked: false,
            message: "暂时无法读取 Homebrew 版本".to_string(),
        };
    };
    let Some((current, current_label)) = parse_homebrew_version(&current_output) else {
        return HomebrewUpdateStatus {
            current_version: current_output,
            latest_version: None,
            update_available: false,
            checked: false,
            message: "无法识别当前 Homebrew 版本".to_string(),
        };
    };

    let configuration = network_configuration_for_mode(mode);
    let mut command = Command::new("/usr/bin/curl");
    apply_network_configuration(&mut command, &configuration);
    let output = command
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--head",
            "--output",
            "/dev/null",
            "--write-out",
            "%{url_effective}",
            "--connect-timeout",
            NETWORK_PROBE_CONNECT_TIMEOUT_SECONDS,
            "--max-time",
            NETWORK_PROBE_MAX_TIME_SECONDS,
            "--user-agent",
            "BrewDesk/0.1.0",
            HOMEBREW_LATEST_RELEASE_URL,
        ])
        .output();
    let Ok(output) = output else {
        return HomebrewUpdateStatus {
            current_version: current_label,
            latest_version: None,
            update_available: false,
            checked: false,
            message: "暂时无法检查 Homebrew 新版本".to_string(),
        };
    };
    if !output.status.success() {
        return HomebrewUpdateStatus {
            current_version: current_label,
            latest_version: None,
            update_available: false,
            checked: false,
            message: "暂时无法检查 Homebrew 新版本".to_string(),
        };
    }

    let final_url = String::from_utf8_lossy(&output.stdout);
    let Some((latest, latest_label)) = parse_homebrew_version(final_url.trim()) else {
        return HomebrewUpdateStatus {
            current_version: current_label,
            latest_version: None,
            update_available: false,
            checked: false,
            message: "无法识别 Homebrew 最新版本".to_string(),
        };
    };
    let update_available = latest > current;
    HomebrewUpdateStatus {
        current_version: current_label,
        latest_version: Some(latest_label.clone()),
        update_available,
        checked: true,
        message: if update_available {
            format!("Homebrew {latest_label} 可更新")
        } else {
            "Homebrew 已是最新版本".to_string()
        },
    }
}

#[tauri::command]
async fn homebrew_update_status(app: AppHandle) -> HomebrewUpdateStatus {
    let state = app.state::<AppState>();
    let brew_path = current_brew_path(&state);
    let mode = current_network_mode(&state);
    tauri::async_runtime::spawn_blocking(move || homebrew_update_status_for_path(brew_path, mode))
        .await
        .unwrap_or_else(|_| HomebrewUpdateStatus {
            current_version: String::new(),
            latest_version: None,
            update_available: false,
            checked: false,
            message: "Homebrew 版本检查线程异常".to_string(),
        })
}

fn detect_brew_availability(configured_path: &str) -> BrewAvailability {
    let arch = std::env::consts::ARCH.to_string();
    let recommended_path = if arch == "x86_64" {
        INTEL_BREW_PATH
    } else {
        DEFAULT_BREW_PATH
    }
    .to_string();
    let command_line_tools_available =
        Path::new("/Library/Developer/CommandLineTools/usr/bin/clang").is_file()
            || Path::new("/Applications/Xcode.app").is_dir();
    let mut candidates = Vec::new();
    if !configured_path.trim().is_empty() {
        candidates.push(configured_path.trim().to_string());
    }
    candidates.push(recommended_path.clone());
    candidates.push(if recommended_path == DEFAULT_BREW_PATH {
        INTEL_BREW_PATH.to_string()
    } else {
        DEFAULT_BREW_PATH.to_string()
    });
    let brew_path = candidates
        .into_iter()
        .find(|path| Path::new(path).is_file());
    let status = if brew_path.is_some() {
        BrewAvailabilityStatus::Ready
    } else if configured_path.trim().is_empty() || configured_path == DEFAULT_BREW_PATH {
        BrewAvailabilityStatus::Missing
    } else {
        BrewAvailabilityStatus::Invalid
    };
    let message = match status {
        BrewAvailabilityStatus::Ready => "Homebrew 已就绪".to_string(),
        BrewAvailabilityStatus::Missing => "这台 Mac 尚未安装 Homebrew".to_string(),
        BrewAvailabilityStatus::Invalid => "配置的 brew 路径不可用，且未找到默认安装".to_string(),
    };
    BrewAvailability {
        status,
        brew_path,
        arch,
        recommended_path,
        command_line_tools_available,
        message,
    }
}

#[tauri::command]
fn brew_availability(state: State<AppState>) -> BrewAvailability {
    let configured = state
        .brew_path
        .lock()
        .map(|path| path.clone())
        .unwrap_or_default();
    let availability = detect_brew_availability(&configured);
    if let Some(found) = availability.brew_path.as_ref() {
        if let Ok(mut path) = state.brew_path.lock() {
            *path = found.clone();
        }
    }
    availability
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn install_body(network_mode: NetworkMode) -> (String, String, EffectiveNetworkMode) {
    match network_mode {
        NetworkMode::Mirror => (
            format!(
                r#"export HOMEBREW_BREW_GIT_REMOTE={}
export HOMEBREW_API_DOMAIN={}
export HOMEBREW_BOTTLE_DOMAIN={}
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
git clone --depth=1 {} "$tmpdir/install"
/bin/bash "$tmpdir/install/install.sh""#,
                shell_single_quote(TUNA_BREW_GIT_REMOTE),
                shell_single_quote(TUNA_API_DOMAIN),
                shell_single_quote(TUNA_BOTTLE_DOMAIN),
                shell_single_quote(TUNA_INSTALL_GIT),
            ),
            "清华大学开源软件镜像站安装脚本".to_string(),
            EffectiveNetworkMode::Mirror,
        ),
        NetworkMode::Official => (
            format!(
                "unset ALL_PROXY all_proxy HTTPS_PROXY https_proxy HTTP_PROXY http_proxy\n{HOMEBREW_INSTALL_COMMAND}"
            ),
            HOMEBREW_INSTALL_COMMAND.to_string(),
            EffectiveNetworkMode::Direct,
        ),
        NetworkMode::Auto => {
            let proxy = detected_system_proxy();
            let mut exports = Vec::new();
            if let Some(url) = proxy.http_url.as_ref() {
                exports.push(format!("export HTTP_PROXY={} http_proxy={}", shell_single_quote(url), shell_single_quote(url)));
            }
            if let Some(url) = proxy.https_url.as_ref().or(proxy.http_url.as_ref()) {
                exports.push(format!("export HTTPS_PROXY={} https_proxy={}", shell_single_quote(url), shell_single_quote(url)));
            }
            if let Some(url) = proxy.socks_url.as_ref() {
                exports.push(format!("export ALL_PROXY={} all_proxy={}", shell_single_quote(url), shell_single_quote(url)));
            }
            let effective = if exports.is_empty() {
                exports.push("unset ALL_PROXY all_proxy HTTPS_PROXY https_proxy HTTP_PROXY http_proxy".to_string());
                EffectiveNetworkMode::Direct
            } else {
                EffectiveNetworkMode::SystemProxy
            };
            exports.push(HOMEBREW_INSTALL_COMMAND.to_string());
            (exports.join("\n"), HOMEBREW_INSTALL_COMMAND.to_string(), effective)
        }
    }
}

fn write_homebrew_install_script(
    path: &Path,
    network_mode: NetworkMode,
) -> Result<EffectiveNetworkMode, String> {
    let (body, _, effective_mode) = install_body(network_mode);
    let route_label = match effective_mode {
        EffectiveNetworkMode::Direct => "官方直连",
        EffectiveNetworkMode::SystemProxy => "macOS 系统代理",
        EffectiveNetworkMode::Mirror => "清华备用线路",
    };
    let script = format!(
        r#"#!/bin/bash
clear
printf '\nBrewDesk 将启动 Homebrew 官方安装程序。\n'
printf '安装过程会先说明改动，并由 macOS Terminal 读取管理员密码。\n\n'
printf '当前下载线路：%s\n\n' {route_label:?}
{body}
status=$?
if [ "$status" -eq 0 ]; then
  printf '\nHomebrew 安装完成。请返回 BrewDesk，应用会自动检测。\n'
else
  printf '\n安装未完成（状态码 %s）。可返回 BrewDesk 重试或使用官方 PKG。\n' "$status"
fi
printf '\n按任意键关闭此窗口...'
read -n 1 -s -r
exit "$status"
"#
    );
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("无法创建安装脚本目录：{err}"))?;
    }
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true).mode(0o700);
    let mut file = options
        .open(path)
        .map_err(|err| format!("无法创建 Homebrew 安装脚本：{err}"))?;
    std::io::Write::write_all(&mut file, script.as_bytes())
        .map_err(|err| format!("无法写入 Homebrew 安装脚本：{err}"))?;
    Ok(effective_mode)
}

#[tauri::command]
async fn start_homebrew_install(
    network_mode: NetworkMode,
    app: AppHandle,
) -> Result<HomebrewInstallLaunch, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|err| format!("无法定位 BrewDesk 缓存目录：{err}"))?;
    let script_path = cache_dir.join("install-homebrew.command");
    tauri::async_runtime::spawn_blocking(move || {
        let resolved_mode = if network_mode == NetworkMode::Auto {
            match select_automatic_network_configuration()?.0.effective_mode {
                EffectiveNetworkMode::Direct => NetworkMode::Official,
                EffectiveNetworkMode::SystemProxy => NetworkMode::Auto,
                EffectiveNetworkMode::Mirror => NetworkMode::Mirror,
            }
        } else {
            network_mode
        };
        let (_, command_preview, _) = install_body(resolved_mode);
        let effective_mode = write_homebrew_install_script(&script_path, resolved_mode)?;
        let status = Command::new("/usr/bin/open")
            .args(["-a", "Terminal"])
            .arg(&script_path)
            .status()
            .map_err(|err| format!("无法打开 Terminal：{err}"))?;
        if !status.success() {
            return Err("Terminal 未能打开 Homebrew 安装脚本".to_string());
        }
        Ok(HomebrewInstallLaunch {
            started: true,
            command_preview,
            network_mode: effective_mode,
        })
    })
    .await
    .map_err(|err| format!("Homebrew 安装引导线程异常：{err}"))?
}

#[tauri::command]
async fn open_homebrew_pkg() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let status = Command::new("/usr/bin/open")
            .arg("https://github.com/Homebrew/brew/releases/latest")
            .status()
            .map_err(|err| format!("无法打开 Homebrew 官方下载页：{err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("Homebrew 官方下载页未能打开".to_string())
        }
    })
    .await
    .map_err(|err| format!("打开下载页的后台线程异常：{err}"))?
}

fn validated_http_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.len() > 2048
        || trimmed.chars().any(char::is_control)
        || !(trimmed.starts_with("https://") || trimmed.starts_with("http://"))
    {
        return Err("只能打开有效的 HTTP 或 HTTPS 主页".to_string());
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
async fn open_external_url(url: String) -> Result<(), String> {
    let url = validated_http_url(&url)?;
    tauri::async_runtime::spawn_blocking(move || {
        let status = Command::new("/usr/bin/open")
            .arg(url)
            .status()
            .map_err(|err| format!("无法打开主页：{err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("主页未能打开".to_string())
        }
    })
    .await
    .map_err(|err| format!("打开主页的后台线程异常：{err}"))?
}

fn native_helper_path(name: &str) -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe().map_err(|err| format!("无法定位 BrewDesk：{err}"))?;
    let executable_dir = current_exe
        .parent()
        .ok_or_else(|| "无法定位 BrewDesk 可执行目录".to_string())?;
    let bundled_candidates = [
        executable_dir.join(name),
        executable_dir.join("../Resources").join(name),
    ];
    if let Some(path) = bundled_candidates.into_iter().find(|path| path.is_file()) {
        return Ok(path);
    }

    #[cfg(debug_assertions)]
    {
        let development_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
        fs::read_dir(&development_dir)
            .map_err(|_| "BrewDesk 本机翻译组件尚未构建".to_string())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.starts_with(&format!("{name}-")))
            })
            .ok_or_else(|| "BrewDesk 本机翻译组件不存在".to_string())
    }

    #[cfg(not(debug_assertions))]
    Err("BrewDesk 本机翻译组件不存在".to_string())
}

fn run_translation_helper(
    helper_path: PathBuf,
    argument: &str,
    input: Option<&[u8]>,
    timeout: Duration,
) -> Result<TranslationResult, String> {
    let mut command = Command::new(helper_path);
    command
        .arg(argument)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command
        .spawn()
        .map_err(|err| format!("无法启动本机翻译组件：{err}"))?;
    if let (Some(data), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin
            .write_all(data)
            .map_err(|err| format!("无法发送待翻译内容：{err}"))?;
    }
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(60)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Apple 本机翻译暂时没有响应，请稍后重试".to_string());
            }
            Err(err) => return Err(format!("无法读取本机翻译状态：{err}")),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("无法读取本机翻译结果：{err}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            "本机翻译组件执行失败".to_string()
        } else {
            message
        });
    }
    serde_json::from_slice(&output.stdout).map_err(|err| format!("本机翻译结果无效：{err}"))
}

#[tauri::command]
async fn translation_status() -> Result<TranslationResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        run_translation_helper(
            native_helper_path("brewdesk-translate")?,
            "--status",
            None,
            Duration::from_secs(8),
        )
    })
    .await
    .map_err(|err| format!("翻译状态线程异常：{err}"))?
}

#[tauri::command]
async fn translate_descriptions(texts: Vec<String>) -> Result<TranslationResult, String> {
    if texts.len() > 20 || texts.iter().any(|text| text.len() > 800) {
        return Err("单次最多翻译 20 条简介，每条不超过 800 个字符".to_string());
    }
    let input = serde_json::to_vec(&serde_json::json!({ "texts": texts }))
        .map_err(|err| format!("无法准备翻译内容：{err}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        run_translation_helper(
            native_helper_path("brewdesk-translate")?,
            "--translate",
            Some(&input),
            Duration::from_secs(60),
        )
    })
    .await
    .map_err(|err| format!("翻译线程异常：{err}"))?
}

#[tauri::command]
async fn prepare_translation() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        Command::new(native_helper_path("brewdesk-translation-setup")?)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("无法打开 Apple 语言包设置：{err}"))
    })
    .await
    .map_err(|err| format!("语言包设置线程异常：{err}"))?
}

fn validated_reveal_path(path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);
    if !candidate.is_absolute() {
        return Err("只能在访达中显示绝对路径".to_string());
    }
    candidate
        .canonicalize()
        .map_err(|_| "要显示的文件或目录已经不存在".to_string())
}

#[tauri::command]
async fn reveal_in_finder(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = validated_reveal_path(&path)?;
        let status = Command::new("/usr/bin/open")
            .arg("-R")
            .arg(&path)
            .status()
            .map_err(|err| format!("无法打开访达：{err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("访达未能显示该项目".to_string())
        }
    })
    .await
    .map_err(|err| format!("打开访达的后台线程异常：{err}"))?
}

#[derive(Debug)]
enum BrewSearchOutcome {
    Matches(Vec<PackageSummary>),
    NoMatches,
    Failed(String),
}

fn is_brew_no_match(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("no formulae or casks found")
        || lower.contains("no formulae found")
        || lower.contains("no casks found")
}

fn search_brew_kind(
    brew_path: &str,
    query: &str,
    kind: PackageKind,
    mode: NetworkMode,
) -> BrewSearchOutcome {
    let kind_arg = if kind == PackageKind::Formula {
        "--formula"
    } else {
        "--cask"
    };
    let output = match brew_read_command(brew_path, mode)
        .args(["search", kind_arg, query])
        .output()
    {
        Ok(output) => output,
        Err(err) => return BrewSearchOutcome::Failed(format!("无法执行 brew：{err}")),
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        let matches = parse_search_output(&stdout, kind);
        if matches.is_empty() {
            BrewSearchOutcome::NoMatches
        } else {
            BrewSearchOutcome::Matches(matches)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if is_brew_no_match(&stderr) {
            BrewSearchOutcome::NoMatches
        } else {
            BrewSearchOutcome::Failed(if stderr.is_empty() {
                format!("brew search {kind_arg} 执行失败")
            } else {
                stderr
            })
        }
    }
}

fn combine_brew_search_outcomes(
    outcomes: Vec<(PackageKind, BrewSearchOutcome)>,
) -> Result<(Vec<PackageSummary>, Vec<String>), String> {
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let mut failures = Vec::new();
    let mut successful_branches = 0_usize;
    for (kind, outcome) in outcomes {
        let label = if kind == PackageKind::Formula {
            "formula"
        } else {
            "cask"
        };
        match outcome {
            BrewSearchOutcome::Matches(mut matches) => {
                successful_branches += 1;
                items.append(&mut matches);
            }
            BrewSearchOutcome::NoMatches => successful_branches += 1,
            BrewSearchOutcome::Failed(message) => {
                failures.push(format!("{label}：{message}"));
                warnings.push(format!("{label} 查询失败，本次结果可能不完整"));
            }
        }
    }
    if successful_branches == 0 {
        Err(failures.join("；"))
    } else {
        Ok((items, warnings))
    }
}

fn search_with_brew(
    brew_path: &str,
    query: &str,
    kind: SearchKind,
    mode: NetworkMode,
) -> Result<(Vec<PackageSummary>, Vec<String>), String> {
    match kind {
        SearchKind::Formula => combine_brew_search_outcomes(vec![(
            PackageKind::Formula,
            search_brew_kind(brew_path, query, PackageKind::Formula, mode),
        )]),
        SearchKind::Cask => combine_brew_search_outcomes(vec![(
            PackageKind::Cask,
            search_brew_kind(brew_path, query, PackageKind::Cask, mode),
        )]),
        SearchKind::Both => {
            let (formulae, casks) = std::thread::scope(|scope| {
                let formula =
                    scope.spawn(|| search_brew_kind(brew_path, query, PackageKind::Formula, mode));
                let cask =
                    scope.spawn(|| search_brew_kind(brew_path, query, PackageKind::Cask, mode));
                let formulae = formula
                    .join()
                    .unwrap_or_else(|_| BrewSearchOutcome::Failed("搜索线程异常".to_string()));
                let casks = cask
                    .join()
                    .unwrap_or_else(|_| BrewSearchOutcome::Failed("搜索线程异常".to_string()));
                (formulae, casks)
            });
            combine_brew_search_outcomes(vec![
                (PackageKind::Formula, formulae),
                (PackageKind::Cask, casks),
            ])
        }
    }
}

fn translate_search_query(query: &str) -> (Option<String>, Option<String>) {
    let input = match serde_json::to_vec(&serde_json::json!({ "texts": [query] })) {
        Ok(input) => input,
        Err(err) => return (None, Some(format!("无法准备本机翻译：{err}"))),
    };
    let helper = match native_helper_path("brewdesk-translate") {
        Ok(path) => path,
        Err(err) => return (None, Some(err)),
    };
    match run_translation_helper(
        helper,
        "--translate-query",
        Some(&input),
        Duration::from_secs(20),
    ) {
        Ok(result) if result.search_installed => {
            let translated = result
                .translations
                .and_then(|items| items.into_iter().next())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty() && value != query);
            (translated, None)
        }
        Ok(result) => (None, Some(result.message)),
        Err(err) => (None, Some(err)),
    }
}

#[tauri::command]
async fn search_packages(
    query: String,
    kind: SearchKind,
    app: AppHandle,
) -> Result<SearchResponse, String> {
    let query = query.trim().to_string();
    validate_search_text(&query)?;
    let state = app.state::<AppState>();
    let brew_path = current_brew_path(&state);
    let mode = current_network_mode(&state);
    let app_for_task = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let alias = chinese_alias(&query);
        let catalog = load_search_catalog(&app_for_task);
        let mut catalog_warning = catalog.as_ref().err().cloned();
        let catalog = catalog.ok();

        if let Some(alias) = alias {
            let mut items = catalog
                .as_deref()
                .map(|items| search_local_catalog(items, &query, Some(alias), &kind))
                .unwrap_or_default();
            let mut warnings = Vec::new();
            if items.is_empty() {
                match search_with_brew(&brew_path, alias.resolved_query, kind.clone(), mode) {
                    Ok((brew_items, brew_warnings)) => {
                        items = brew_items;
                        warnings.extend(brew_warnings);
                    }
                    Err(err) => warnings.push(format!("Homebrew 查询失败：{err}")),
                }
            }
            if let Some(catalog) = catalog.as_deref() {
                enrich_search_results(&mut items, catalog);
            }
            if items.is_empty() {
                if let Some(message) = catalog_warning.take() {
                    warnings.push(message);
                }
            }
            return Ok(SearchResponse {
                query,
                resolved_query: Some(alias.resolved_query.to_string()),
                source: SearchSource::Alias,
                warnings,
                items,
            });
        }

        let local_items = catalog
            .as_deref()
            .map(|items| search_local_catalog(items, &query, None, &kind))
            .unwrap_or_default();

        if contains_han(&query) {
            if !local_items.is_empty() {
                return Ok(SearchResponse {
                    query,
                    resolved_query: None,
                    source: SearchSource::Catalog,
                    warnings: Vec::new(),
                    items: local_items,
                });
            }
            let (translated, translation_warning) = translate_search_query(&query);
            if let Some(translated) = translated {
                let mut items = catalog
                    .as_deref()
                    .map(|catalog| search_local_catalog(catalog, &translated, None, &kind))
                    .unwrap_or_default();
                let mut warnings = Vec::new();
                if items.is_empty() {
                    match search_with_brew(&brew_path, &translated, kind.clone(), mode) {
                        Ok((brew_items, brew_warnings)) => {
                            items = brew_items;
                            warnings.extend(brew_warnings);
                        }
                        Err(err) => warnings.push(format!("Homebrew 查询失败：{err}")),
                    }
                }
                if let Some(catalog) = catalog.as_deref() {
                    enrich_search_results(&mut items, catalog);
                }
                return Ok(SearchResponse {
                    query,
                    resolved_query: Some(translated),
                    source: SearchSource::Translation,
                    warnings,
                    items,
                });
            }
            let mut warnings = Vec::new();
            warnings.push(translation_warning.unwrap_or_else(|| {
                "暂未识别这个中文查询；可在设置中启用 Apple 本机翻译后重试".to_string()
            }));
            if let Some(message) = catalog_warning.take() {
                warnings.push(message);
            }
            return Ok(SearchResponse {
                query,
                resolved_query: None,
                source: SearchSource::Catalog,
                warnings,
                items: Vec::new(),
            });
        }

        let (brew_items, mut warnings) = match search_with_brew(&brew_path, &query, kind, mode) {
            Ok(result) => result,
            Err(err) if !local_items.is_empty() => (
                Vec::new(),
                vec![format!("Homebrew 查询失败，正在显示本机目录结果：{err}")],
            ),
            Err(err) => return Err(err),
        };
        let mut brew_items = brew_items;
        if let Some(catalog) = catalog.as_deref() {
            enrich_search_results(&mut brew_items, catalog);
        } else if let Some(message) = catalog_warning.take() {
            warnings.push(message);
        }
        let items = merge_search_results(local_items, brew_items);
        Ok(SearchResponse {
            query,
            resolved_query: None,
            source: SearchSource::Brew,
            warnings,
            items,
        })
    })
    .await
    .map_err(|err| format!("搜索线程异常：{err}"))?
}

fn package_info_for_path(
    brew_path: String,
    token: String,
    kind: PackageKind,
    mode: NetworkMode,
) -> Result<PackageDetail, String> {
    validate_token(&token)?;
    let kind_arg = if kind == PackageKind::Cask {
        "--cask"
    } else {
        "--formula"
    };
    let output = command_output(&brew_path, &["info", "--json=v2", kind_arg, &token], mode)?;
    let mut detail = parse_info_detail(&output, kind.clone())?;
    if kind == PackageKind::Cask {
        detail.detected_applications = detect_cask_applications(&output, detail.installed)?;
        if detail.version.trim().is_empty() {
            detail.version = detail
                .detected_applications
                .iter()
                .find_map(|app| app.version.clone())
                .unwrap_or_default();
        }
        if detail.installed {
            detail.install_location = detail
                .detected_applications
                .iter()
                .find(|app| app.brew_managed)
                .map(|app| app.path.clone())
                .or_else(|| {
                    command_output(&brew_path, &["--caskroom", &token], mode)
                        .ok()
                        .filter(|path| Path::new(path).exists())
                });
        }
    } else if detail.installed {
        detail.install_location = command_output(&brew_path, &["--prefix", &token], mode)
            .ok()
            .filter(|path| Path::new(path).exists());
    }
    detail.installed_size_bytes = detail
        .install_location
        .as_deref()
        .and_then(|path| disk_usage_bytes(Path::new(path)));
    Ok(detail)
}

fn disk_usage_bytes(path: &Path) -> Option<u64> {
    let output = Command::new("/usr/bin/du")
        .arg("-sk")
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)
}

#[tauri::command]
async fn package_info(
    token: String,
    kind: PackageKind,
    app: AppHandle,
) -> Result<PackageDetail, String> {
    let state = app.state::<AppState>();
    let brew_path = current_brew_path(&state);
    let mode = current_network_mode(&state);
    tauri::async_runtime::spawn_blocking(move || {
        package_info_for_path(brew_path, token, kind, mode)
    })
    .await
    .map_err(|err| format!("包详情线程异常：{err}"))?
}

fn parse_installed_packages(output: &str) -> Result<Vec<PackageSummary>, String> {
    let value: Value =
        serde_json::from_str(output).map_err(|err| format!("已安装列表解析失败：{err}"))?;
    let mut packages = Vec::new();
    if let Some(formulae) = value.get("formulae").and_then(Value::as_array) {
        for item in formulae {
            if let Some(token) = item.get("name").and_then(Value::as_str) {
                packages.push(PackageSummary {
                    token: token.to_string(),
                    name: package_name_from_token(token),
                    kind: PackageKind::Formula,
                    description: item
                        .get("desc")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    installed: true,
                    outdated: false,
                    localized_name: None,
                    version: item
                        .get("installed")
                        .and_then(Value::as_array)
                        .and_then(|versions| versions.last())
                        .and_then(|installed| installed.get("version"))
                        .and_then(Value::as_str)
                        .or_else(|| {
                            item.get("versions")
                                .and_then(|versions| versions.get("stable"))
                                .and_then(Value::as_str)
                        })
                        .map(ToString::to_string),
                    homepage: item
                        .get("homepage")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    match_reason: None,
                });
            }
        }
    }
    if let Some(casks) = value.get("casks").and_then(Value::as_array) {
        for item in casks {
            if let Some(token) = item.get("token").and_then(Value::as_str) {
                packages.push(PackageSummary {
                    token: token.to_string(),
                    name: item
                        .get("name")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                        .unwrap_or(token)
                        .to_string(),
                    kind: PackageKind::Cask,
                    description: item
                        .get("desc")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    installed: true,
                    outdated: false,
                    localized_name: item
                        .get("name")
                        .and_then(Value::as_array)
                        .and_then(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .find(|name| contains_han(name))
                        })
                        .map(ToString::to_string),
                    version: item
                        .get("installed")
                        .and_then(Value::as_str)
                        .filter(|version| !version.is_empty())
                        .or_else(|| item.get("version").and_then(Value::as_str))
                        .map(ToString::to_string),
                    homepage: item
                        .get("homepage")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    match_reason: None,
                });
            }
        }
    }
    Ok(packages)
}

fn installed_packages_for_path(
    brew_path: String,
    mode: NetworkMode,
) -> Result<Vec<PackageSummary>, String> {
    let output = command_output(&brew_path, &["info", "--installed", "--json=v2"], mode)?;
    parse_installed_packages(&output)
}

fn installed_inventory_for_path(
    brew_path: String,
    mode: NetworkMode,
) -> Result<InstalledInventory, String> {
    let output = command_output(&brew_path, &["info", "--installed", "--json=v2"], mode)?;
    let value: Value =
        serde_json::from_str(&output).map_err(|err| format!("已安装清单解析失败：{err}"))?;
    Ok(InstalledInventory {
        homebrew: parse_installed_packages(&output)?,
        other: scan_other_applications(&value),
    })
}

#[tauri::command]
async fn installed_packages(app: AppHandle) -> Result<Vec<PackageSummary>, String> {
    let state = app.state::<AppState>();
    let brew_path = current_brew_path(&state);
    let mode = current_network_mode(&state);
    tauri::async_runtime::spawn_blocking(move || installed_packages_for_path(brew_path, mode))
        .await
        .map_err(|err| format!("已安装列表线程异常：{err}"))?
}

#[tauri::command]
async fn installed_inventory(app: AppHandle) -> Result<InstalledInventory, String> {
    let state = app.state::<AppState>();
    let brew_path = current_brew_path(&state);
    let mode = current_network_mode(&state);
    tauri::async_runtime::spawn_blocking(move || installed_inventory_for_path(brew_path, mode))
        .await
        .map_err(|err| format!("本机应用扫描线程异常：{err}"))?
}

fn outdated_packages_for_path(
    brew_path: String,
    mode: NetworkMode,
) -> Result<Vec<OutdatedPackage>, String> {
    let output = command_output(&brew_path, &["outdated", "--json=v2"], mode)?;
    let value: Value =
        serde_json::from_str(&output).map_err(|err| format!("可更新列表解析失败：{err}"))?;
    let mut packages = Vec::new();
    if let Some(formulae) = value.get("formulae").and_then(Value::as_array) {
        for item in formulae {
            if let Some(token) = item.get("name").and_then(Value::as_str) {
                packages.push(OutdatedPackage {
                    token: token.to_string(),
                    name: package_name_from_token(token),
                    kind: PackageKind::Formula,
                    current_version: item
                        .get("installed_versions")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default(),
                    latest_version: item
                        .get("current_version")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }
    }
    if let Some(casks) = value.get("casks").and_then(Value::as_array) {
        for item in casks {
            if let Some(token) = item.get("name").and_then(Value::as_str) {
                packages.push(OutdatedPackage {
                    token: token.to_string(),
                    name: package_name_from_token(token),
                    kind: PackageKind::Cask,
                    current_version: item
                        .get("installed_versions")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    latest_version: item
                        .get("current_version")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }
    }
    Ok(packages)
}

#[tauri::command]
async fn outdated_packages(app: AppHandle) -> Result<Vec<OutdatedPackage>, String> {
    let state = app.state::<AppState>();
    let brew_path = current_brew_path(&state);
    let mode = current_network_mode(&state);
    tauri::async_runtime::spawn_blocking(move || outdated_packages_for_path(brew_path, mode))
        .await
        .map_err(|err| format!("可更新列表线程异常：{err}"))?
}

#[tauri::command]
fn operation_events(
    operation_id: String,
    state: State<AppState>,
) -> Result<Option<BrewOperation>, String> {
    let operations = state
        .operations
        .lock()
        .map_err(|_| "无法读取操作记录".to_string())?;
    Ok(operations.get(&operation_id).cloned())
}

fn percentage_from_line(line: &str) -> Option<u8> {
    line.split_whitespace().find_map(|part| {
        part.trim_matches(|ch: char| !ch.is_ascii_digit() && ch != '%')
            .strip_suffix('%')
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|value| *value <= 100)
    })
}

fn bytes_from_human_size(value: &str) -> Option<u64> {
    let cleaned = value
        .trim()
        .trim_matches(|character: char| matches!(character, '(' | ')' | '[' | ']' | ','));
    let split_at = cleaned
        .find(|character: char| character.is_ascii_alphabetic())
        .unwrap_or(cleaned.len());
    let number = cleaned[..split_at].trim().parse::<f64>().ok()?;
    let unit = cleaned[split_at..].trim().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "b" | "" => 1_f64,
        "kb" | "kib" => 1024_f64,
        "mb" | "mib" => 1024_f64.powi(2),
        "gb" | "gib" => 1024_f64.powi(3),
        _ => return None,
    };
    (number.is_finite() && number >= 0.0).then_some((number * multiplier).round() as u64)
}

fn has_explicit_byte_unit(value: &str) -> bool {
    let cleaned = value
        .trim()
        .trim_matches(|character: char| matches!(character, '(' | ')' | '[' | ']' | ','));
    cleaned
        .find(|character: char| character.is_ascii_alphabetic())
        .is_some()
}

fn transfer_sizes_from_line(line: &str) -> Option<(u64, u64)> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        let pair = if let Some((downloaded, total)) = part.split_once('/') {
            if total.is_empty() {
                parts.get(index + 1).map(|next| (downloaded, *next))
            } else {
                Some((downloaded, total))
            }
        } else if part.ends_with('/') {
            parts
                .get(index + 1)
                .map(|next| (part.trim_end_matches('/'), *next))
        } else {
            None
        };
        let Some((downloaded, total)) = pair else {
            continue;
        };
        // Homebrew also prints ordinary counters such as `3/3`. Without an
        // explicit byte unit that is item progress, not a transfer amount.
        if !has_explicit_byte_unit(downloaded) || !has_explicit_byte_unit(total) {
            continue;
        }
        let Some(downloaded) = bytes_from_human_size(downloaded) else {
            continue;
        };
        let Some(total) = bytes_from_human_size(total) else {
            continue;
        };
        if total > 0 && downloaded <= total {
            return Some((downloaded, total));
        }
    }
    None
}

fn log_progress(
    line: &str,
) -> (
    Option<OperationPhase>,
    Option<String>,
    Option<u8>,
    Option<String>,
    bool,
) {
    let lower = line.to_ascii_lowercase();
    let network_issue = [
        "curl:",
        "protocol_error",
        "http/2 stream",
        "could not resolve",
        "connection refused",
        "connection reset",
        "connection timed out",
        "operation timed out",
        "failed to connect",
        "ssl_error",
        "http 403",
        "http 404",
        "http 429",
        "http 5",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    if lower.contains("retrying download")
        || (lower.contains("retrying") && lower.contains("second"))
    {
        return (
            Some(OperationPhase::Downloading),
            Some("等待下载重试".to_string()),
            None,
            None,
            true,
        );
    }

    if lower.contains("downloading") || percentage_from_line(line).is_some() {
        let download_percent = percentage_from_line(line);
        let progress = download_percent
            .map(|value| 24 + ((value as u16 * 46) / 100) as u8)
            .or(Some(24));
        let item = line
            .split_once("Downloading")
            .map(|(_, value)| value.trim().trim_start_matches(':').trim().to_string())
            .filter(|value| !value.is_empty());
        return (
            Some(OperationPhase::Downloading),
            Some("正在下载".to_string()),
            progress,
            item,
            network_issue,
        );
    }
    if lower.contains("installing") || lower.contains("pouring") || lower.contains("linking") {
        return (
            Some(OperationPhase::Installing),
            Some("正在安装".to_string()),
            Some(76),
            None,
            network_issue,
        );
    }
    if lower.contains("cleaning up") || lower.contains("cleanup") {
        return (
            Some(OperationPhase::Cleaning),
            Some("正在清理".to_string()),
            Some(90),
            None,
            network_issue,
        );
    }
    if lower.contains("verifying") || lower.contains("checksum") {
        return (
            Some(OperationPhase::Verifying),
            Some("正在验证".to_string()),
            Some(95),
            None,
            network_issue,
        );
    }
    if lower.contains("fetching")
        || lower.contains("dependencies")
        || lower.contains("updating homebrew")
    {
        return (
            Some(OperationPhase::Resolving),
            Some("正在解析依赖".to_string()),
            Some(16),
            None,
            network_issue,
        );
    }
    (None, None, None, None, network_issue)
}

fn stream_operation_output<R: Read>(
    mut reader: R,
    stream: &str,
    app: &AppHandle,
    operation_id: &str,
) {
    let mut chunk = [0_u8; 1024];
    let mut pending = Vec::new();
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                for byte in &chunk[..read] {
                    if *byte == b'\n' || *byte == b'\r' {
                        if !pending.is_empty() {
                            let line = String::from_utf8_lossy(&pending).trim().to_string();
                            pending.clear();
                            if !line.is_empty() {
                                let _ = push_operation_log(
                                    app,
                                    operation_id,
                                    OperationLog {
                                        stream: stream.to_string(),
                                        line,
                                        at: now_ms(),
                                    },
                                );
                            }
                        }
                    } else {
                        pending.push(*byte);
                    }
                }
            }
            Err(_) => break,
        }
    }
    if !pending.is_empty() {
        let line = String::from_utf8_lossy(&pending).trim().to_string();
        if !line.is_empty() {
            let _ = push_operation_log(
                app,
                operation_id,
                OperationLog {
                    stream: stream.to_string(),
                    line,
                    at: now_ms(),
                },
            );
        }
    }
}

fn record_download_activity(
    app: &AppHandle,
    operation_id: &str,
    snapshot: &DownloadSnapshot,
    bytes_per_second: Option<u64>,
    completed_file: bool,
) -> Result<(), String> {
    let at = now_ms();
    let activity_label = if completed_file {
        "一个下载已完成，正在继续处理".to_string()
    } else {
        "持续收到下载数据".to_string()
    };
    let state = app.state::<AppState>();
    let status = {
        let mut operations = state
            .operations
            .lock()
            .map_err(|_| "无法更新下载活动".to_string())?;
        let operation = operations
            .get_mut(operation_id)
            .ok_or_else(|| "操作记录不存在".to_string())?;
        operation.last_activity_at = at;
        operation.transfer_bytes_per_second = bytes_per_second;
        operation.downloaded_bytes = snapshot.active_file_bytes;
        operation.activity_label = Some(activity_label.clone());
        if let Some(item) = snapshot.active_file.as_ref() {
            operation.current_item = Some(item.clone());
        }
        operation.status.clone()
    };
    app.emit(
        "brew-operation",
        OperationEvent {
            operation_id: operation_id.to_string(),
            status,
            phase: None,
            phase_label: None,
            progress: None,
            current_item: snapshot.active_file.clone(),
            network_issue: None,
            effective_network_mode: None,
            proxy_url: None,
            last_activity_at: Some(at),
            transfer_bytes_per_second: bytes_per_second,
            downloaded_bytes: snapshot.active_file_bytes,
            total_bytes: None,
            activity_label: Some(activity_label),
            log: None,
            exit_code: None,
            finished_at: None,
        },
    )
    .map_err(|err| err.to_string())
}

#[tauri::command]
fn run_brew_action(
    action: BrewAction,
    target: Option<BrewTarget>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<BrewOperation, String> {
    let args = command_for_action(&action, target.as_ref())?;
    let brew_path = current_brew_path(&state);
    let network_mode = current_network_mode(&state);
    let network_configuration = network_configuration_for_mode(network_mode);
    let started_at = now_ms();
    let operation = BrewOperation {
        id: Uuid::new_v4().to_string(),
        action: action.clone(),
        target: target.clone(),
        command_preview: command_preview(&brew_path, &args),
        status: OperationStatus::Queued,
        phase: OperationPhase::Preparing,
        phase_label: "正在准备".to_string(),
        progress: Some(5),
        current_item: target.as_ref().map(|value| value.token.clone()),
        network_issue: false,
        effective_network_mode: network_configuration.effective_mode,
        proxy_url: network_configuration.proxy_url,
        last_activity_at: started_at,
        transfer_bytes_per_second: None,
        downloaded_bytes: None,
        total_bytes: None,
        activity_label: Some("任务已加入队列".to_string()),
        started_at,
        finished_at: None,
        exit_code: None,
        logs: Vec::new(),
    };
    state
        .operations
        .lock()
        .map_err(|_| "无法保存操作记录".to_string())?
        .insert(operation.id.clone(), operation.clone());

    let start_worker = state
        .operation_queue
        .lock()
        .map_err(|_| "无法加入 Homebrew 操作队列".to_string())?
        .enqueue(QueuedBrewCommand {
            operation_id: operation.id.clone(),
            brew_path,
            action,
            network_mode,
            args,
            target,
        });
    if start_worker {
        std::thread::spawn(move || process_operation_queue(app));
    }

    Ok(operation)
}

fn process_operation_queue(app: AppHandle) {
    loop {
        let next = {
            let state = app.state::<AppState>();
            let Ok(mut queue) = state.operation_queue.lock() else {
                return;
            };
            queue.next()
        };
        let Some(command) = next else {
            return;
        };
        execute_queued_brew_command(&app, command);
    }
}

fn execute_queued_brew_command(app: &AppHandle, command: QueuedBrewCommand) {
    let QueuedBrewCommand {
        operation_id,
        brew_path,
        action,
        network_mode,
        args,
        target,
    } = command;
    let cancelled_before_start = app
        .state::<AppState>()
        .cancel_requested
        .lock()
        .map(|requests| requests.contains(&operation_id))
        .unwrap_or(false);
    if cancelled_before_start {
        let _ = finish_operation(app, &operation_id, OperationStatus::Cancelled, None);
        return;
    }
    let _ = update_operation_status(app, &operation_id, OperationStatus::Running, None);
    let _ = update_operation_progress(
        app,
        &operation_id,
        OperationPhase::Resolving,
        "正在解析依赖",
        Some(14),
        target.as_ref().map(|value| value.token.clone()),
    );
    let network_configuration = if network_mode == NetworkMode::Auto
        && matches!(
            action,
            BrewAction::Install
                | BrewAction::Upgrade
                | BrewAction::Repair
                | BrewAction::UpgradeAll
                | BrewAction::Update
        ) {
        match select_automatic_network_configuration() {
            Ok((configuration, probe)) => {
                let _ = update_operation_network_route(app, &operation_id, &configuration);
                let _ = push_operation_log(
                    app,
                    &operation_id,
                    OperationLog {
                        stream: "stdout".to_string(),
                        line: format!(
                            "{} · 核心链路 {}/{} · {} ms",
                            configuration.message,
                            probe.covered_endpoints,
                            probe.total_endpoints,
                            probe.latency_ms
                        ),
                        at: now_ms(),
                    },
                );
                configuration
            }
            Err(message) => {
                let _ = mark_operation_network_issue(app, &operation_id);
                let _ = push_operation_log(
                    app,
                    &operation_id,
                    OperationLog {
                        stream: "stderr".to_string(),
                        line: message,
                        at: now_ms(),
                    },
                );
                let _ = finish_operation(app, &operation_id, OperationStatus::Failed, Some(1));
                return;
            }
        }
    } else {
        network_configuration_for_mode(network_mode)
    };
    let mut brew = brew_command_for_configuration(&brew_path, &network_configuration);
    brew.process_group(0);
    let mut child = match brew
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            let log = OperationLog {
                stream: "stderr".to_string(),
                line: format!("无法启动 brew：{err}"),
                at: now_ms(),
            };
            let _ = push_operation_log(app, &operation_id, log);
            let _ = finish_operation(app, &operation_id, OperationStatus::Failed, Some(1));
            return;
        }
    };

    if let Ok(mut active) = app.state::<AppState>().active_processes.lock() {
        active.insert(operation_id.clone(), child.id());
    }
    let cancel_after_spawn = app
        .state::<AppState>()
        .cancel_requested
        .lock()
        .map(|requests| requests.contains(&operation_id))
        .unwrap_or(false);
    if cancel_after_spawn {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGTERM);
        }
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_handle = stdout.map(|stdout| {
        let app = app.clone();
        let operation_id = operation_id.clone();
        std::thread::spawn(move || {
            stream_operation_output(stdout, "stdout", &app, &operation_id);
        })
    });
    let stderr_handle = stderr.map(|stderr| {
        let app = app.clone();
        let operation_id = operation_id.clone();
        std::thread::spawn(move || {
            stream_operation_output(stderr, "stderr", &app, &operation_id);
        })
    });

    let mut previous_download = homebrew_download_snapshot(app);
    let mut previous_sample_at = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if previous_sample_at.elapsed() >= Duration::from_secs(2) {
            let current_download = homebrew_download_snapshot(app);
            let changed = current_download.total_incomplete_bytes
                != previous_download.total_incomplete_bytes
                || current_download.active_file != previous_download.active_file
                || current_download.active_file_bytes != previous_download.active_file_bytes;
            if changed {
                let elapsed = previous_sample_at.elapsed().as_secs_f64().max(0.1);
                let added_bytes = current_download
                    .total_incomplete_bytes
                    .saturating_sub(previous_download.total_incomplete_bytes);
                let completed_file = current_download.total_incomplete_bytes
                    < previous_download.total_incomplete_bytes;
                let bytes_per_second = if added_bytes > 0 {
                    Some((added_bytes as f64 / elapsed).round() as u64)
                } else if completed_file {
                    Some(0)
                } else {
                    None
                };
                let _ = record_download_activity(
                    app,
                    &operation_id,
                    &current_download,
                    bytes_per_second,
                    completed_file,
                );
            }
            previous_download = current_download;
            previous_sample_at = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    if let Some(handle) = stdout_handle {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }
    let exit_code = status.and_then(|status| status.code());
    if let Ok(mut active) = app.state::<AppState>().active_processes.lock() {
        active.remove(&operation_id);
    }
    let was_cancelled = app
        .state::<AppState>()
        .cancel_requested
        .lock()
        .map(|requests| requests.contains(&operation_id))
        .unwrap_or(false);
    if was_cancelled {
        let _ = finish_operation(app, &operation_id, OperationStatus::Cancelled, exit_code);
        return;
    }
    if exit_code == Some(0) {
        let _ = update_operation_progress(
            app,
            &operation_id,
            OperationPhase::Verifying,
            "正在验证结果",
            Some(97),
            None,
        );
    }
    let final_status = if exit_code == Some(0) {
        OperationStatus::Succeeded
    } else {
        OperationStatus::Failed
    };
    let _ = finish_operation(app, &operation_id, final_status, exit_code);
}

#[tauri::command]
fn cancel_operation(
    operation_id: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    let status = state
        .operations
        .lock()
        .map_err(|_| "无法读取操作状态".to_string())?
        .get(&operation_id)
        .map(|operation| operation.status.clone())
        .ok_or_else(|| "操作记录不存在".to_string())?;
    if !matches!(status, OperationStatus::Queued | OperationStatus::Running) {
        return Ok(());
    }

    state
        .cancel_requested
        .lock()
        .map_err(|_| "无法记录取消请求".to_string())?
        .insert(operation_id.clone());

    let removed_from_queue = state
        .operation_queue
        .lock()
        .map_err(|_| "无法访问操作队列".to_string())?
        .remove(&operation_id);
    if removed_from_queue {
        return finish_operation(&app, &operation_id, OperationStatus::Cancelled, None);
    }

    let process_id = state
        .active_processes
        .lock()
        .map_err(|_| "无法读取运行中的进程".to_string())?
        .get(&operation_id)
        .copied();
    if let Some(process_id) = process_id {
        let result = unsafe { libc::kill(-(process_id as i32), libc::SIGTERM) };
        if result != 0 {
            return Err(format!(
                "无法停止 Homebrew 任务：{}",
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
}

fn update_operation_status(
    app: &AppHandle,
    operation_id: &str,
    status: OperationStatus,
    exit_code: Option<i32>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    if let Some(operation) = state
        .operations
        .lock()
        .map_err(|_| "无法更新操作状态".to_string())?
        .get_mut(operation_id)
    {
        operation.status = status.clone();
        operation.exit_code = exit_code;
        operation.last_activity_at = now_ms();
    }
    let event = OperationEvent {
        operation_id: operation_id.to_string(),
        status,
        phase: None,
        phase_label: None,
        progress: None,
        current_item: None,
        network_issue: None,
        effective_network_mode: None,
        proxy_url: None,
        last_activity_at: Some(now_ms()),
        transfer_bytes_per_second: None,
        downloaded_bytes: None,
        total_bytes: None,
        activity_label: None,
        log: None,
        exit_code,
        finished_at: None,
    };
    app.emit("brew-operation", event)
        .map_err(|err| err.to_string())
}

fn update_operation_network_route(
    app: &AppHandle,
    operation_id: &str,
    configuration: &NetworkConfiguration,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let status = {
        let mut operations = state
            .operations
            .lock()
            .map_err(|_| "无法更新网络线路".to_string())?;
        let operation = operations
            .get_mut(operation_id)
            .ok_or_else(|| "操作记录不存在".to_string())?;
        operation.effective_network_mode = configuration.effective_mode;
        operation.proxy_url = configuration.proxy_url.clone();
        operation.activity_label = Some(configuration.message.clone());
        operation.last_activity_at = now_ms();
        operation.status.clone()
    };
    app.emit(
        "brew-operation",
        OperationEvent {
            operation_id: operation_id.to_string(),
            status,
            phase: None,
            phase_label: None,
            progress: None,
            current_item: None,
            network_issue: None,
            effective_network_mode: Some(configuration.effective_mode),
            proxy_url: configuration.proxy_url.clone(),
            last_activity_at: Some(now_ms()),
            transfer_bytes_per_second: None,
            downloaded_bytes: None,
            total_bytes: None,
            activity_label: Some(configuration.message.clone()),
            log: None,
            exit_code: None,
            finished_at: None,
        },
    )
    .map_err(|err| err.to_string())
}

fn mark_operation_network_issue(app: &AppHandle, operation_id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let status = {
        let mut operations = state
            .operations
            .lock()
            .map_err(|_| "无法更新网络状态".to_string())?;
        let operation = operations
            .get_mut(operation_id)
            .ok_or_else(|| "操作记录不存在".to_string())?;
        operation.network_issue = true;
        operation.status.clone()
    };
    app.emit(
        "brew-operation",
        OperationEvent {
            operation_id: operation_id.to_string(),
            status,
            phase: None,
            phase_label: None,
            progress: None,
            current_item: None,
            network_issue: Some(true),
            effective_network_mode: None,
            proxy_url: None,
            last_activity_at: Some(now_ms()),
            transfer_bytes_per_second: None,
            downloaded_bytes: None,
            total_bytes: None,
            activity_label: Some("自动选路未找到可用线路".to_string()),
            log: None,
            exit_code: None,
            finished_at: None,
        },
    )
    .map_err(|err| err.to_string())
}

fn update_operation_progress(
    app: &AppHandle,
    operation_id: &str,
    phase: OperationPhase,
    phase_label: &str,
    progress: Option<u8>,
    current_item: Option<String>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let status = {
        let mut operations = state
            .operations
            .lock()
            .map_err(|_| "无法更新操作进度".to_string())?;
        let operation = operations
            .get_mut(operation_id)
            .ok_or_else(|| "操作记录不存在".to_string())?;
        operation.phase = phase.clone();
        operation.phase_label = phase_label.to_string();
        operation.last_activity_at = now_ms();
        if let Some(value) = progress {
            operation.progress = Some(value);
        }
        if current_item.is_some() {
            operation.current_item = current_item.clone();
        }
        operation.status.clone()
    };
    app.emit(
        "brew-operation",
        OperationEvent {
            operation_id: operation_id.to_string(),
            status,
            phase: Some(phase),
            phase_label: Some(phase_label.to_string()),
            progress,
            current_item,
            network_issue: None,
            effective_network_mode: None,
            proxy_url: None,
            last_activity_at: Some(now_ms()),
            transfer_bytes_per_second: None,
            downloaded_bytes: None,
            total_bytes: None,
            activity_label: None,
            log: None,
            exit_code: None,
            finished_at: None,
        },
    )
    .map_err(|err| err.to_string())
}

fn push_operation_log(
    app: &AppHandle,
    operation_id: &str,
    log: OperationLog,
) -> Result<(), String> {
    let (phase, phase_label, progress, current_item, network_issue) = log_progress(&log.line);
    let transfer_sizes = transfer_sizes_from_line(&log.line);
    let state = app.state::<AppState>();
    let status = {
        let mut operations = state
            .operations
            .lock()
            .map_err(|_| "无法写入操作日志".to_string())?;
        let operation = operations
            .get_mut(operation_id)
            .ok_or_else(|| "操作记录不存在".to_string())?;
        if operation.logs.len() >= 1000 {
            let excess = operation.logs.len() + 1 - 1000;
            operation.logs.drain(0..excess);
        }
        operation.logs.push(log.clone());
        operation.last_activity_at = log.at;
        operation.activity_label = Some("Homebrew 输出了新日志".to_string());
        if let Some(value) = phase.as_ref() {
            operation.phase = value.clone();
        }
        if let Some(value) = phase_label.as_ref() {
            operation.phase_label = value.clone();
        }
        if let Some(value) = progress {
            operation.progress = Some(value);
        }
        if let Some(value) = current_item.as_ref() {
            operation.current_item = Some(value.clone());
        }
        if let Some((downloaded, total)) = transfer_sizes {
            operation.downloaded_bytes = Some(downloaded);
            operation.total_bytes = Some(total);
        }
        operation.network_issue |= network_issue;
        operation.status.clone()
    };
    let event = OperationEvent {
        operation_id: operation_id.to_string(),
        status,
        phase,
        phase_label,
        progress,
        current_item,
        network_issue: network_issue.then_some(true),
        effective_network_mode: None,
        proxy_url: None,
        last_activity_at: Some(log.at),
        transfer_bytes_per_second: None,
        downloaded_bytes: transfer_sizes.map(|(downloaded, _)| downloaded),
        total_bytes: transfer_sizes.map(|(_, total)| total),
        activity_label: Some("Homebrew 输出了新日志".to_string()),
        log: Some(log),
        exit_code: None,
        finished_at: None,
    };
    app.emit("brew-operation", event)
        .map_err(|err| err.to_string())
}

fn finish_operation(
    app: &AppHandle,
    operation_id: &str,
    status: OperationStatus,
    exit_code: Option<i32>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let finished_at = now_ms();
    if let Some(operation) = state
        .operations
        .lock()
        .map_err(|_| "无法完成操作".to_string())?
        .get_mut(operation_id)
    {
        operation.status = status.clone();
        operation.phase = match &status {
            OperationStatus::Succeeded => OperationPhase::Completed,
            OperationStatus::Cancelled => OperationPhase::Cancelled,
            _ => OperationPhase::Failed,
        };
        operation.phase_label = match &status {
            OperationStatus::Succeeded => "操作完成".to_string(),
            OperationStatus::Cancelled => "已取消".to_string(),
            _ => "操作失败".to_string(),
        };
        if status == OperationStatus::Succeeded {
            operation.progress = Some(100);
        }
        operation.finished_at = Some(finished_at);
        operation.exit_code = exit_code;
        operation.last_activity_at = finished_at;
        operation.current_item = None;
        operation.transfer_bytes_per_second = None;
        operation.downloaded_bytes = None;
        operation.total_bytes = None;
        operation.activity_label = Some(operation.phase_label.clone());
    }
    let event = OperationEvent {
        operation_id: operation_id.to_string(),
        status: status.clone(),
        phase: Some(match &status {
            OperationStatus::Succeeded => OperationPhase::Completed,
            OperationStatus::Cancelled => OperationPhase::Cancelled,
            _ => OperationPhase::Failed,
        }),
        phase_label: Some(match &status {
            OperationStatus::Succeeded => "操作完成".to_string(),
            OperationStatus::Cancelled => "已取消".to_string(),
            _ => "操作失败".to_string(),
        }),
        progress: (status == OperationStatus::Succeeded).then_some(100),
        current_item: None,
        network_issue: None,
        effective_network_mode: None,
        proxy_url: None,
        last_activity_at: Some(finished_at),
        transfer_bytes_per_second: None,
        downloaded_bytes: None,
        total_bytes: None,
        activity_label: None,
        log: None,
        exit_code,
        finished_at: Some(finished_at),
    };
    if let Ok(mut requests) = state.cancel_requested.lock() {
        requests.remove(operation_id);
    }
    app.emit("brew-operation", event)
        .map_err(|err| err.to_string())
}

fn app_builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .manage(AppState {
            brew_path: Mutex::new(DEFAULT_BREW_PATH.to_string()),
            network_mode: Mutex::new(NetworkMode::Auto),
            operations: Mutex::new(HashMap::new()),
            operation_queue: Mutex::new(OperationQueueState::default()),
            active_processes: Mutex::new(HashMap::new()),
            cancel_requested: Mutex::new(HashSet::new()),
            search_catalog: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            brew_availability,
            brew_environment,
            homebrew_update_status,
            set_brew_path,
            network_configuration,
            set_network_mode,
            test_network_connection,
            start_homebrew_install,
            open_homebrew_pkg,
            open_external_url,
            translation_status,
            translate_descriptions,
            prepare_translation,
            reveal_in_finder,
            search_packages,
            package_info,
            installed_packages,
            installed_inventory,
            outdated_packages,
            run_brew_action,
            cancel_operation,
            operation_events
        ])
}

#[cfg(not(test))]
fn main() {
    app_builder()
        .run(tauri::generate_context!())
        .expect("error while running BrewDesk");
}

#[cfg(test)]
fn main() {
    let _ = app_builder();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::os::unix::fs::PermissionsExt;

    fn search_item(
        token: &str,
        names: &[&str],
        description: &str,
        kind: PackageKind,
    ) -> SearchCatalogItem {
        SearchCatalogItem {
            token: token.to_string(),
            aliases: Vec::new(),
            names: names.iter().map(|name| name.to_string()).collect(),
            description: Some(description.to_string()),
            kind,
        }
    }

    #[test]
    fn validates_safe_homebrew_tokens() {
        assert!(validate_token("homebrew/cask/visual-studio-code").is_ok());
        assert!(validate_token("node@22").is_ok());
        assert!(validate_token("bad;rm").is_err());
        assert!(validate_token("").is_err());
    }

    #[test]
    fn parses_homebrew_versions_from_cli_and_release_urls() {
        assert_eq!(
            parse_homebrew_version("Homebrew 6.0.12-3-gabc"),
            Some(((6, 0, 12), "6.0.12".to_string()))
        );
        assert_eq!(
            parse_homebrew_version("https://github.com/Homebrew/brew/releases/tag/6.1.0"),
            Some(((6, 1, 0), "6.1.0".to_string()))
        );
        assert_eq!(
            parse_homebrew_version("Homebrew 6.2"),
            Some(((6, 2, 0), "6.2.0".to_string()))
        );
        assert_eq!(parse_homebrew_version("Homebrew unknown"), None);
    }

    #[test]
    fn accepts_utf8_search_text_but_keeps_action_tokens_strict() {
        assert!(validate_search_text("微信").is_ok());
        assert!(validate_search_text("谷歌浏览器").is_ok());
        assert!(validate_token("微信").is_err());
        assert!(validate_token("wechat;rm").is_err());
    }

    #[test]
    fn rejects_shell_injection_before_argv_construction() {
        for token in [
            "wget;open /Applications",
            "wget && whoami",
            "$(whoami)",
            "`whoami`",
            "wget\nupdate",
        ] {
            let result = command_for_action(
                &BrewAction::Install,
                Some(&BrewTarget {
                    token: token.to_string(),
                    kind: PackageKind::Formula,
                }),
            );
            assert!(result.is_err(), "unsafe token reached argv: {token:?}");
        }
    }

    #[test]
    fn maps_curated_chinese_aliases() {
        assert_eq!(chinese_alias("微信").unwrap().resolved_query, "wechat");
        assert_eq!(
            chinese_alias("谷歌浏览器").unwrap().resolved_query,
            "google-chrome"
        );
        let archive_tokens = chinese_alias("解压").unwrap().tokens;
        assert!(archive_tokens.contains(&"keka"));
        assert!(archive_tokens.contains(&"the-unarchiver"));
        let android = chinese_alias("安卓").unwrap();
        assert_eq!(android.resolved_query, "android");
        assert!(android.tokens.contains(&"android-platform-tools"));
    }

    #[test]
    fn broad_android_query_returns_related_tools_and_respects_kind() {
        let catalog = vec![
            search_item(
                "android-studio",
                &["Android Studio"],
                "Tools for building Android applications",
                PackageKind::Cask,
            ),
            search_item(
                "android-platform-tools",
                &["Android SDK Platform-Tools"],
                "Android SDK component including adb and fastboot",
                PackageKind::Cask,
            ),
            search_item(
                "android-commandlinetools",
                &["Android SDK Command-line Tools"],
                "Command-line tools for Android apps",
                PackageKind::Cask,
            ),
            search_item(
                "android-formula",
                &["Android Formula"],
                "Android helper",
                PackageKind::Formula,
            ),
        ];
        let results =
            search_local_catalog(&catalog, "安卓", chinese_alias("安卓"), &SearchKind::Both);
        let tokens = results
            .iter()
            .map(|item| item.token.as_str())
            .collect::<Vec<_>>();
        assert!(tokens.contains(&"android-platform-tools"));
        assert!(tokens.contains(&"android-studio"));
        assert!(tokens.contains(&"android-commandlinetools"));

        let formulae = search_local_catalog(
            &catalog,
            "安卓",
            chinese_alias("安卓"),
            &SearchKind::Formula,
        );
        assert!(formulae
            .iter()
            .all(|item| item.kind == PackageKind::Formula));
    }

    #[test]
    fn android_debugging_query_prioritizes_platform_tools() {
        let catalog = vec![
            search_item(
                "android-studio",
                &["Android Studio"],
                "Android IDE",
                PackageKind::Cask,
            ),
            search_item(
                "android-platform-tools",
                &["Android SDK Platform-Tools"],
                "adb and fastboot",
                PackageKind::Cask,
            ),
            search_item(
                "android-commandlinetools",
                &["Android SDK Command-line Tools"],
                "Android command-line tools",
                PackageKind::Cask,
            ),
        ];
        let results = search_local_catalog(
            &catalog,
            "安卓调试工具",
            chinese_alias("安卓调试工具"),
            &SearchKind::Both,
        );
        assert_eq!(results[0].token, "android-platform-tools");
    }

    #[test]
    fn combines_independent_formula_and_cask_search_outcomes() {
        let cask = parse_search_output("android-platform-tools", PackageKind::Cask);
        let (items, warnings) = combine_brew_search_outcomes(vec![
            (PackageKind::Formula, BrewSearchOutcome::NoMatches),
            (PackageKind::Cask, BrewSearchOutcome::Matches(cask)),
        ])
        .unwrap();
        assert_eq!(items[0].token, "android-platform-tools");
        assert!(warnings.is_empty());

        let (items, warnings) = combine_brew_search_outcomes(vec![
            (
                PackageKind::Formula,
                BrewSearchOutcome::Failed("network unavailable".to_string()),
            ),
            (PackageKind::Cask, BrewSearchOutcome::NoMatches),
        ])
        .unwrap();
        assert!(items.is_empty());
        assert_eq!(warnings.len(), 1);

        assert!(combine_brew_search_outcomes(vec![
            (
                PackageKind::Formula,
                BrewSearchOutcome::Failed("formula failed".to_string()),
            ),
            (
                PackageKind::Cask,
                BrewSearchOutcome::Failed("cask failed".to_string()),
            ),
        ])
        .is_err());
    }

    #[test]
    fn recognizes_homebrew_no_match_as_an_empty_result() {
        assert!(is_brew_no_match(
            "Error: No formulae or casks found for \"android-platform-tools\"."
        ));
    }

    #[test]
    fn corrupted_catalog_is_rejected_without_panicking() {
        assert!(parse_cask_catalog("not-json").is_err());
        assert!(parse_cask_catalog(r#"{"payload":"not-json"}"#).is_err());
    }

    #[test]
    fn installer_script_is_fixed_and_owner_executable() {
        let path = std::env::temp_dir().join(format!("brewdesk-{}.command", Uuid::new_v4()));
        write_homebrew_install_script(&path, NetworkMode::Official).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert!(contents.contains(HOMEBREW_INSTALL_COMMAND));
        assert!(!contents.contains("NONINTERACTIVE"));
        assert_eq!(mode, 0o700);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn mirror_installer_is_session_scoped_and_uses_tuna() {
        let path = std::env::temp_dir().join(format!("brewdesk-mirror-{}.command", Uuid::new_v4()));
        write_homebrew_install_script(&path, NetworkMode::Mirror).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains(TUNA_INSTALL_GIT));
        assert!(contents.contains(TUNA_API_DOMAIN));
        assert!(!contents.contains(".zprofile"));
        assert!(!contents.contains(".zshrc"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_enabled_macos_system_proxy() {
        let proxy = parse_scutil_proxy(
            r#"<dictionary> {
  HTTPEnable : 1
  HTTPPort : 7892
  HTTPProxy : 127.0.0.1
  HTTPSEnable : 1
  HTTPSPort : 7892
  HTTPSProxy : 127.0.0.1
  SOCKSEnable : 1
  SOCKSPort : 7892
  SOCKSProxy : 127.0.0.1
}"#,
        );
        assert_eq!(proxy.http_url.as_deref(), Some("http://127.0.0.1:7892"));
        assert_eq!(proxy.https_url.as_deref(), Some("http://127.0.0.1:7892"));
        assert_eq!(proxy.socks_url.as_deref(), Some("socks5h://127.0.0.1:7892"));
    }

    #[test]
    fn discovers_only_loopback_ports_owned_by_proxy_processes() {
        let output = r#"
tcp4 0 0 127.0.0.1.7892 *.* LISTEN 0 0 131072 131072 wandacloudCore:1115
tcp4 0 0 127.0.0.1.7892 127.0.0.1.64929 ESTABLISHED 0 0 wandacloudCore:1115
tcp4 0 0 127.0.0.1.27060 *.* LISTEN 0 0 131072 131072 steam_osx:51338
tcp6 0 0 ::1.6153 *.* LISTEN 0 0 131072 131072 mihomo:2200
tcp4 0 0 198.18.0.1.50592 *.* LISTEN 0 0 131072 131072 wandacloudCore:1115
"#;
        assert_eq!(parse_loopback_proxy_ports(output), vec![6153, 7892]);
    }

    #[test]
    fn automatic_network_template_adds_discovered_proxy_port_without_duplicates() {
        let candidates = auto_network_candidates_with_local(
            SystemProxy::default(),
            vec!["http://127.0.0.1:7892".to_string()],
        );
        assert_eq!(candidates.len(), 3);
        assert_eq!(
            candidates[0].proxy_url.as_deref(),
            Some("http://127.0.0.1:7892")
        );
        assert!(candidates[0].message.contains("发现"));

        let deduplicated = auto_network_candidates_with_local(
            SystemProxy {
                http_url: Some("http://127.0.0.1:7892".to_string()),
                https_url: Some("http://127.0.0.1:7892".to_string()),
                socks_url: None,
            },
            vec!["http://127.0.0.1:7892".to_string()],
        );
        assert_eq!(
            deduplicated
                .iter()
                .filter(|candidate| candidate.proxy_url.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn automatic_network_template_orders_proxy_direct_then_mirror() {
        let candidates = auto_network_candidates(SystemProxy {
            http_url: Some("http://127.0.0.1:7892".to_string()),
            https_url: Some("http://127.0.0.1:7892".to_string()),
            socks_url: Some("socks5h://127.0.0.1:7892".to_string()),
        });
        assert_eq!(candidates.len(), 3);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.effective_mode)
                .collect::<Vec<_>>(),
            vec![
                EffectiveNetworkMode::SystemProxy,
                EffectiveNetworkMode::Direct,
                EffectiveNetworkMode::Mirror,
            ]
        );
        assert_eq!(
            candidates[0].proxy_url.as_deref(),
            Some("http://127.0.0.1:7892")
        );
    }

    #[test]
    fn automatic_network_template_skips_missing_system_proxy() {
        let candidates = auto_network_candidates(SystemProxy::default());
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].effective_mode, EffectiveNetworkMode::Direct);
        assert_eq!(candidates[1].effective_mode, EffectiveNetworkMode::Mirror);
    }

    #[test]
    fn mirror_network_settings_are_child_process_scoped() {
        let tracked_keys = [
            "HOMEBREW_API_DOMAIN",
            "HOMEBREW_BOTTLE_DOMAIN",
            "HOMEBREW_BREW_GIT_REMOTE",
            "HTTP_PROXY",
            "HTTPS_PROXY",
        ];
        let before = tracked_keys
            .iter()
            .map(|key| ((*key).to_string(), std::env::var_os(key)))
            .collect::<HashMap<_, _>>();
        let configuration = network_configuration_for_effective_mode(
            NetworkMode::Mirror,
            EffectiveNetworkMode::Mirror,
            None,
            "test",
        );
        let mut command = Command::new("/usr/bin/true");
        apply_network_configuration(&mut command, &configuration);
        let child_environment = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|item| item.to_string_lossy().into_owned()),
                )
            })
            .collect::<HashMap<_, _>>();

        assert_eq!(
            child_environment
                .get("HOMEBREW_API_DOMAIN")
                .and_then(Option::as_deref),
            Some(TUNA_API_DOMAIN)
        );
        assert_eq!(
            child_environment
                .get("HOMEBREW_BOTTLE_DOMAIN")
                .and_then(Option::as_deref),
            Some(TUNA_BOTTLE_DOMAIN)
        );
        assert_eq!(
            child_environment
                .get("HOMEBREW_BREW_GIT_REMOTE")
                .and_then(Option::as_deref),
            Some(TUNA_BREW_GIT_REMOTE)
        );
        assert_eq!(child_environment.get("HTTP_PROXY"), Some(&None));
        assert_eq!(child_environment.get("HTTPS_PROXY"), Some(&None));
        for key in tracked_keys {
            assert_eq!(std::env::var_os(key), before[key]);
        }
    }

    #[test]
    fn direct_network_mode_clears_inherited_routes_for_child_only() {
        let before_proxy = std::env::var_os("HTTPS_PROXY");
        let configuration = network_configuration_for_effective_mode(
            NetworkMode::Official,
            EffectiveNetworkMode::Direct,
            None,
            "test",
        );
        let mut command = Command::new("/usr/bin/true");
        apply_network_configuration(&mut command, &configuration);
        let child_environment = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|item| item.to_string_lossy().into_owned()),
                )
            })
            .collect::<HashMap<_, _>>();

        assert_eq!(child_environment.get("HTTPS_PROXY"), Some(&None));
        assert_eq!(child_environment.get("HOMEBREW_API_DOMAIN"), Some(&None));
        assert_eq!(
            child_environment
                .get("HOMEBREW_DOWNLOAD_CONCURRENCY")
                .and_then(Option::as_deref),
            Some("2")
        );
        assert_eq!(std::env::var_os("HTTPS_PROXY"), before_proxy);
    }

    #[test]
    fn coverage_probe_checks_three_core_endpoints_per_route() {
        let direct = network_configuration_for_effective_mode(
            NetworkMode::Official,
            EffectiveNetworkMode::Direct,
            None,
            "direct",
        );
        let direct_targets = network_probe_targets(&direct);
        assert_eq!(direct_targets.len(), 3);
        assert_eq!(
            direct_targets
                .iter()
                .map(|target| target.label)
                .collect::<Vec<_>>(),
            vec!["Homebrew API", "GitHub", "GHCR"]
        );

        let mirror = network_configuration_for_effective_mode(
            NetworkMode::Mirror,
            EffectiveNetworkMode::Mirror,
            None,
            "mirror",
        );
        let mirror_targets = network_probe_targets(&mirror);
        assert_eq!(mirror_targets.len(), 3);
        assert!(mirror_targets.iter().all(|target| target.head_only));
    }

    #[test]
    fn coverage_probe_allows_slow_proxy_tls_handshakes() {
        assert_eq!(NETWORK_PROBE_CONNECT_TIMEOUT_SECONDS, "10");
        assert_eq!(NETWORK_PROBE_MAX_TIME_SECONDS, "15");
    }

    #[test]
    fn maps_homebrew_output_to_stage_progress() {
        let (phase, label, progress, item, network_issue) =
            log_progress("==> Downloading https://example.com/node.tar.gz 50%");
        assert_eq!(phase, Some(OperationPhase::Downloading));
        assert_eq!(label.as_deref(), Some("正在下载"));
        assert_eq!(progress, Some(47));
        assert!(item.unwrap().contains("node.tar.gz"));
        assert!(!network_issue);

        let (_, _, _, _, network_issue) = log_progress("curl: (28) Operation timed out");
        assert!(network_issue);

        let (phase, label, progress, _, network_issue) =
            log_progress("Retrying download in 25 s... (one try left)");
        assert_eq!(phase, Some(OperationPhase::Downloading));
        assert_eq!(label.as_deref(), Some("等待下载重试"));
        assert_eq!(progress, None);
        assert!(network_issue);
    }

    #[test]
    fn parses_exact_homebrew_transfer_amounts_without_estimating() {
        assert_eq!(
            transfer_sizes_from_line("✔︎ Cask demo Verified 25.9MB/ 25.9MB"),
            Some((27_158_118, 27_158_118))
        );
        assert_eq!(transfer_sizes_from_line("Downloading bottles 3/3"), None);
        assert_eq!(transfer_sizes_from_line("Downloading ######## 57.8%"), None);
    }

    #[test]
    fn shortens_homebrew_incomplete_download_names() {
        assert_eq!(
            display_download_name("abc123--rust--1.97.1.arm64_tahoe.bottle.tar.gz.incomplete"),
            "rust--1.97.1.arm64_tahoe.bottle.tar.gz"
        );
        assert_eq!(
            display_download_name("googlechrome.dmg.incomplete"),
            "googlechrome.dmg"
        );
    }

    #[test]
    fn builds_install_cask_command_without_shell() {
        let args = command_for_action(
            &BrewAction::Install,
            Some(&BrewTarget {
                token: "visual-studio-code".to_string(),
                kind: PackageKind::Cask,
            }),
        )
        .unwrap();
        assert_eq!(args, vec!["install", "--cask", "visual-studio-code"]);
    }

    #[test]
    fn builds_forced_cask_reinstall_for_repair() {
        let args = command_for_action(
            &BrewAction::Repair,
            Some(&BrewTarget {
                token: "wpsoffice-cn".to_string(),
                kind: PackageKind::Cask,
            }),
        )
        .unwrap();
        assert_eq!(args, vec!["reinstall", "--cask", "--force", "wpsoffice-cn"]);
    }

    #[test]
    fn parses_formula_detail() {
        let json = r#"{"formulae":[{"name":"wget","desc":"Internet file retriever","homepage":"https://www.gnu.org/software/wget/","versions":{"stable":"1.25.0"},"installed":[{"version":"1.25.0"}],"dependencies":["openssl@3"]}],"casks":[]}"#;
        let detail = parse_info_detail(json, PackageKind::Formula).unwrap();
        assert_eq!(detail.token, "wget");
        assert_eq!(detail.description, "Internet file retriever");
        assert!(detail.installed);
        assert_eq!(detail.dependencies, vec!["openssl@3"]);
    }

    #[test]
    fn installed_list_prefers_actual_installed_versions() {
        let json = r#"{
          "formulae":[{"name":"demo","versions":{"stable":"2.0"},"installed":[{"version":"1.8"}]}],
          "casks":[{"token":"demo-app","name":["Demo App"],"version":"2.0","installed":"1.9"}]
        }"#;
        let packages = parse_installed_packages(json).unwrap();
        assert_eq!(packages[0].version.as_deref(), Some("1.8"));
        assert_eq!(packages[1].version.as_deref(), Some("1.9"));
    }

    #[test]
    fn cask_detection_checks_system_and_user_application_folders() {
        let item: Value = serde_json::from_str(
            r#"{"artifacts":[{"app":["Visual Studio Code.app"],"target":"/Applications/Visual Studio Code.app"}]}"#,
        )
        .unwrap();
        let candidates = cask_app_candidates(&item, Some(Path::new("/Users/test")));
        assert!(candidates.contains(&(PathBuf::from("/Applications/Visual Studio Code.app"), true)));
        assert!(candidates.contains(&(
            PathBuf::from("/Users/test/Applications/Visual Studio Code.app"),
            false
        )));
    }

    #[test]
    fn application_scan_stops_at_app_bundles_and_identifies_app_store_receipts() {
        let root = std::env::temp_dir().join(format!("brewdesk-app-scan-{}", Uuid::new_v4()));
        let direct = root.join("Direct.app");
        let nested = root.join("Vendor/Nested.app");
        let helper = direct.join("Contents/Helpers/Should Not Appear.app");
        fs::create_dir_all(&helper).unwrap();
        fs::create_dir_all(&nested).unwrap();

        let mut apps = Vec::new();
        collect_application_paths(&root, 2, &mut apps);
        assert!(apps.contains(&direct));
        assert!(apps.contains(&nested));
        assert!(!apps.contains(&helper));

        let receipt = direct.join("Contents/_MASReceipt/receipt");
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        fs::write(&receipt, b"receipt").unwrap();
        assert!(is_mac_app_store_application(&direct));
        assert!(!is_mac_app_store_application(&nested));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unconfirmed_cask_application_is_not_treated_as_homebrew_managed() {
        let root = std::env::temp_dir().join(format!("brewdesk-origin-test-{}", Uuid::new_v4()));
        let foreign_app = root.join("Foreign.app");
        fs::create_dir_all(&foreign_app).unwrap();
        let json = serde_json::json!({
            "casks": [{
                "artifacts": [{
                    "app": ["Foreign.app"],
                    "target": foreign_app.to_string_lossy()
                }]
            }]
        })
        .to_string();

        let detected = detect_cask_applications(&json, false).unwrap();
        let application = detected
            .iter()
            .find(|item| item.path == foreign_app.to_string_lossy())
            .unwrap();
        assert!(!application.brew_managed);
        assert_eq!(application.source, "其他渠道或来源未知");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finder_reveal_accepts_only_existing_absolute_paths() {
        assert!(validated_reveal_path("relative/path").is_err());
        assert!(validated_reveal_path("/definitely/missing/brewdesk/path").is_err());
        assert!(validated_reveal_path(std::env::temp_dir().to_string_lossy().as_ref()).is_ok());
    }

    #[test]
    fn opens_only_http_homepage_urls() {
        assert!(validated_http_url("https://example.com/app").is_ok());
        assert!(validated_http_url("http://example.com").is_ok());
        assert!(validated_http_url("file:///Applications").is_err());
        assert!(validated_http_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn operation_queue_runs_fifo_with_only_one_worker() {
        let command = |id: &str| QueuedBrewCommand {
            operation_id: id.to_string(),
            brew_path: DEFAULT_BREW_PATH.to_string(),
            action: BrewAction::Upgrade,
            network_mode: NetworkMode::Auto,
            args: vec!["upgrade".to_string(), id.to_string()],
            target: None,
        };
        let mut queue = OperationQueueState::default();
        assert!(queue.enqueue(command("first")));
        assert!(!queue.enqueue(command("second")));
        assert!(!queue.enqueue(command("third")));
        assert!(queue.remove("second"));
        assert!(!queue.remove("missing"));
        assert_eq!(queue.next().unwrap().operation_id, "first");
        assert_eq!(queue.next().unwrap().operation_id, "third");
        assert!(queue.worker_running);
        assert!(queue.next().is_none());
        assert!(!queue.worker_running);
    }
}
