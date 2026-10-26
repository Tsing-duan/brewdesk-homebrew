use super::*;

// These probes only read local Homebrew metadata. They cannot auto-update or
// wait indefinitely behind a download or an interactive password request.
fn probe(
    brew_path: &str,
    args: &[&str],
    configuration: &NetworkConfiguration,
) -> Result<String, String> {
    probe_with_timeout(brew_path, args, configuration, Duration::from_secs(30))
}

fn probe_with_timeout(
    brew_path: &str,
    args: &[&str],
    configuration: &NetworkConfiguration,
    timeout: Duration,
) -> Result<String, String> {
    let mut command = brew_command_for_configuration(brew_path, configuration);
    command.env("HOMEBREW_NO_AUTO_UPDATE", "1");
    command.process_group(0);
    let mut child = command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("无法启动结果核验：{err}"))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let read = |mut stream: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let out = read(Box::new(stdout));
    let err = read(Box::new(stderr));
    let deadline = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if deadline.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(20))
            }
            result => {
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                break Err(match result {
                    Err(err) => format!("结果核验进程异常：{err}"),
                    _ => "结果核验超时；尚不能确认安装完成，请刷新已安装列表。".to_string(),
                });
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| "核验输出线程异常".to_string())?
        .map_err(|err| format!("无法读取核验输出：{err}"))?;
    let stderr = err
        .join()
        .map_err(|_| "核验错误线程异常".to_string())?
        .map_err(|err| format!("无法读取核验错误：{err}"))?;
    if status?.success() {
        Ok(String::from_utf8_lossy(&stdout).to_string())
    } else {
        Err(format!(
            "无法核验安装结果：{}",
            String::from_utf8_lossy(&stderr).trim()
        ))
    }
}

pub(super) fn targets(
    action: &BrewAction,
    target: Option<&BrewTarget>,
    brew_path: &str,
    configuration: &NetworkConfiguration,
) -> Result<Vec<BrewTarget>, String> {
    match action {
        BrewAction::Install | BrewAction::Upgrade | BrewAction::Repair => {
            Ok(target.cloned().into_iter().collect())
        }
        BrewAction::UpgradeAll => parse_targets(&probe(
            brew_path,
            &["outdated", "--json=v2"],
            configuration,
        )?),
        _ => Ok(Vec::new()),
    }
}

fn parse_targets(json: &str) -> Result<Vec<BrewTarget>, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|err| format!("无法读取更新前清单：{err}"))?;
    let mut targets = Vec::new();
    for (key, kind) in [
        ("formulae", PackageKind::Formula),
        ("casks", PackageKind::Cask),
    ] {
        let items = value
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("更新前清单缺少 {key}，尚未开始更新。"))?;
        for item in items {
            let token = item
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "更新前清单缺少包名".to_string())?;
            validate_token(token)?;
            targets.push(BrewTarget {
                token: token.to_string(),
                kind: kind.clone(),
            });
        }
    }
    Ok(targets)
}

pub(super) fn verify(
    brew_path: &str,
    configuration: &NetworkConfiguration,
    targets: &[BrewTarget],
    require_current: bool,
) -> Result<String, String> {
    if targets.is_empty() {
        return Ok("此次操作没有需要核验的安装目标。".to_string());
    }
    let json = probe(
        brew_path,
        &["info", "--installed", "--json=v2"],
        configuration,
    )?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    verify_inventory(&json, targets, require_current, home.as_deref())?;
    Ok(format!("已核验 {} 个目标的 Homebrew 安装记录及声明的 .app 文件；尚未验证软件能否启动或原有隐私授权是否保留。", targets.len()))
}

fn verify_inventory(
    json: &str,
    targets: &[BrewTarget],
    require_current: bool,
    home: Option<&Path>,
) -> Result<(), String> {
    let value: Value =
        serde_json::from_str(json).map_err(|err| format!("结果核验数据无法解析：{err}"))?;
    let mut failures = Vec::new();
    for target in targets {
        let key = if target.kind == PackageKind::Cask {
            "casks"
        } else {
            "formulae"
        };
        let item = value
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|item| {
                ["name", "full_name", "token", "full_token"]
                    .iter()
                    .any(|key| item.get(key).and_then(Value::as_str) == Some(target.token.as_str()))
                    || item
                        .get("aliases")
                        .and_then(Value::as_array)
                        .is_some_and(|aliases| {
                            aliases
                                .iter()
                                .any(|alias| alias.as_str() == Some(target.token.as_str()))
                        })
            });
        let Some(item) = item else {
            failures.push(format!("{}：没有安装记录", target.token));
            continue;
        };
        let installed = item
            .get("installed")
            .is_some_and(|installed| match installed {
                Value::Array(items) => !items.is_empty(),
                Value::String(version) => !version.trim().is_empty(),
                _ => false,
            });
        if !installed {
            failures.push(format!("{}：没有已安装版本", target.token));
            continue;
        }
        if require_current && item.get("outdated").and_then(Value::as_bool) != Some(false) {
            failures.push(format!("{}：仍可更新或版本状态未知", target.token));
        }
        if target.kind == PackageKind::Cask {
            // Each declared app must exist; candidates for the same app are
            // alternate locations, never substitutes for a different artifact.
            if let Some(artifacts) = item.get("artifacts").and_then(Value::as_array) {
                for artifact in artifacts
                    .iter()
                    .filter(|artifact| artifact.get("app").is_some())
                {
                    let single = serde_json::json!({ "artifacts": [artifact] });
                    let present = cask_app_candidates(&single, home).iter().any(|(path, _)| {
                        plist_value(path, "CFBundleExecutable").is_some_and(|executable| {
                            !executable.contains('/')
                                && path.join("Contents/MacOS").join(executable).is_file()
                        })
                    });
                    if !present {
                        failures.push(format!("{}：声明的 .app 或主程序缺失", target.token));
                    }
                }
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "brew 已退出，但安装结果核验未通过：{}。请刷新列表检查；不要将它视为全部安装成功。",
            failures.join("；")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_exit_cannot_substitute_for_installation_evidence() {
        let targets = vec![BrewTarget {
            token: "demo".into(),
            kind: PackageKind::Cask,
        }];
        let missing = r#"{"casks":[{"token":"demo","installed":"2","outdated":false,"artifacts":[{"app":["DefinitelyMissingBrewDeskTest.app"]}]}]}"#;
        assert!(verify_inventory(missing, &targets, true, None)
            .unwrap_err()
            .contains("主程序缺失"));
        assert!(verify_inventory(r#"{"casks":[]}"#, &targets, false, None).is_err());
        let formula = vec![BrewTarget {
            token: "demo".into(),
            kind: PackageKind::Formula,
        }];
        assert!(verify_inventory(
            r#"{"formulae":[{"name":"demo","installed":[{"version":"1"}],"outdated":true}]}"#,
            &formula,
            true,
            None
        )
        .is_err());
        assert!(verify_inventory(
            r#"{"formulae":[{"name":"demo","installed":[{"version":"2"}],"outdated":false}]}"#,
            &formula,
            true,
            None
        )
        .is_ok());
    }

    #[test]
    fn captures_formula_and_cask_bulk_targets_and_rejects_invalid_metadata() {
        let targets =
            parse_targets(r#"{"formulae":[{"name":"node"}],"casks":[{"name":"demo"}]}"#).unwrap();
        assert_eq!(targets.len(), 2);
        assert!(parse_targets(r#"{"formulae":[],"casks":[{"name":"demo;invalid"}]}"#).is_err());
        assert!(parse_targets("{}").is_err());
    }

    #[test]
    fn probe_is_read_only_and_terminates_a_stuck_process() {
        let configuration = network_configuration_for_mode(NetworkMode::Official);
        assert_eq!(
            probe(
                "/bin/sh",
                &[
                    "-c",
                    "test \"$HOMEBREW_NO_AUTO_UPDATE\" = 1 && printf readonly"
                ],
                &configuration
            )
            .unwrap(),
            "readonly"
        );
        let start = Instant::now();
        let error = probe_with_timeout(
            "/bin/sh",
            &["-c", "while :; do :; done"],
            &configuration,
            Duration::from_millis(30),
        )
        .unwrap_err();
        assert!(error.contains("核验超时"));
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn cask_requires_every_declared_app_and_its_main_program() {
        let root = std::env::temp_dir().join(format!("brewdesk-verification-{}", Uuid::new_v4()));
        let app = root.join("Demo.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleExecutable</key><string>demo</string></dict></plist>"#).unwrap();
        let targets = vec![BrewTarget {
            token: "demo".into(),
            kind: PackageKind::Cask,
        }];
        let mut inventory = serde_json::json!({"casks":[{"token":"demo","installed":"2","outdated":false,"artifacts":[{"app":["Demo.app"],"target":app}]}]});
        assert!(verify_inventory(&inventory.to_string(), &targets, true, None).is_err());
        fs::write(app.join("Contents/MacOS/demo"), "fixture").unwrap();
        assert!(verify_inventory(&inventory.to_string(), &targets, true, None).is_ok());
        inventory["casks"][0]["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"app":["MissingBrewDeskCompanion.app"]}));
        assert!(verify_inventory(&inventory.to_string(), &targets, true, None).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
