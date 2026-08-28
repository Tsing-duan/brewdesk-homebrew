#!/usr/bin/env python3
"""Run a dynamic Gitleaks hit/no-hit Canary without persisting the complete token."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import tempfile

EXPECTED_VERSION = "8.30.0"
RULE_ID = "brewdesk-stage-a-canary"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def private_write(path: Path, value: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as handle:
        handle.write(value)
        handle.flush()
        os.fsync(handle.fileno())


def run_scan(binary: Path, config: Path, target: Path, report: Path) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [
            str(binary),
            "dir",
            "--config",
            str(config),
            "--redact=100",
            "--no-color",
            "--no-banner",
            "--report-format",
            "json",
            "--report-path",
            str(report),
            str(target),
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def erase_tree(root: Path) -> None:
    if not root.exists():
        return
    for path in root.rglob("*"):
        if not path.is_file() or path.is_symlink():
            continue
        try:
            length = path.stat().st_size
            with path.open("r+b", buffering=0) as handle:
                handle.write(b"\0" * length)
                os.fsync(handle.fileno())
        except OSError:
            pass
    shutil.rmtree(root, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--temp-parent", required=True, type=Path)
    args = parser.parse_args()

    binary = args.binary.resolve(strict=True)
    config = args.config.resolve(strict=True)
    parent = args.temp_parent.resolve(strict=True)
    version = subprocess.run(
        [str(binary), "version"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout.strip()
    if version != EXPECTED_VERSION:
        raise SystemExit(f"Expected Gitleaks {EXPECTED_VERSION}, found {version}")

    root = Path(tempfile.mkdtemp(prefix="brewdesk-gitleaks-canary-", dir=parent))
    os.chmod(root, 0o700)
    token = b"BREWDESK_GITLEAKS_CANARY_" + secrets.token_hex(32).upper().encode("ascii")
    try:
        hit_dir = root / "hit"
        clean_dir = root / "clean"
        hit_dir.mkdir(mode=0o700)
        clean_dir.mkdir(mode=0o700)
        private_write(hit_dir / "canary.txt", token + b"\n")
        private_write(clean_dir / "control.txt", b"BREWDESK_GITLEAKS_CONTROL_NO_SECRET\n")
        hit_report = root / "hit-report.json"
        clean_report = root / "clean-report.json"
        hit = run_scan(binary, config, hit_dir, hit_report)
        clean = run_scan(binary, config, clean_dir, clean_report)
        hit_report_bytes = hit_report.read_bytes() if hit_report.exists() else b""
        clean_report_bytes = clean_report.read_bytes() if clean_report.exists() else b""
        findings = json.loads(hit_report_bytes or b"[]")
        matched_rule = any(finding.get("RuleID") == RULE_ID for finding in findings)
        surfaces = [hit.stdout, hit.stderr, clean.stdout, clean.stderr, hit_report_bytes, clean_report_bytes]
        redacted = all(token not in surface for surface in surfaces)
        passed = hit.returncode == 1 and clean.returncode == 0 and matched_rule and redacted
        result = {
            "gitleaks_version": version,
            "binary_sha256": sha256(binary),
            "config_sha256": sha256(config),
            "hit_exit_code": hit.returncode,
            "clean_exit_code": clean.returncode,
            "expected_rule_matched": matched_rule,
            "complete_token_absent_from_outputs": redacted,
            "GITLEAKS_CANARY_PASS": passed,
            "GITLEAKS_OUTPUT_REDACTION_VERIFIED": redacted,
        }
        print(json.dumps(result, sort_keys=True))
        return 0 if passed else 1
    finally:
        erase_tree(root)
        token = b""


if __name__ == "__main__":
    raise SystemExit(main())
