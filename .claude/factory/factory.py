#!/usr/bin/env python3
"""The software factory's configuration and GitHub App token (CLM-24).

    factory.py config   validates the configuration and prints it, merged, as JSON
    factory.py token    prints an installation token of the GitHub App (about an hour), for GH_TOKEN

The factory reads .claude/factory/config.json (Linear, caps, rounds, worktrees) and, from .github/issue-flow.json, the
App, how to run the app, how to capture evidence and which workflow deploys. It acts on GitHub as the App, never as the
person, so that the person can approve its pull requests. Exit 0 on success; 1 with every problem listed otherwise.
Standard library and the openssl command only; scripts/include/tests/test_factory_config.py tests it.
"""

from __future__ import annotations

import base64
import json
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Callable, List, Optional, Tuple

ROOT = Path(__file__).resolve().parents[2]


class ConfigError(Exception):
    def __init__(self, problems: List[str]):
        super().__init__("; ".join(problems))
        self.problems = problems


def _read(path: Path, problems: List[str]) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as why:
        problems.append(f"cannot read {path}: {why}")
        return {}
    return value if isinstance(value, dict) else {}


def _get(config: dict, dotted: str):
    value = config
    for part in dotted.split("."):
        if not isinstance(value, dict) or part not in value:
            return None
        value = value[part]
    return value


def load_config(root: Path = ROOT) -> dict:
    problems: List[str] = []
    own = _read(Path(root) / ".claude" / "factory" / "config.json", problems)
    flow = _read(Path(root) / ".github" / "issue-flow.json", problems)
    config = dict(own)
    for key in ("app", "run", "evidence", "deploy"):
        if key in flow:
            config[key] = flow[key]

    texts = [
        "linear.team", "linear.intake", "linear.states.working", "linear.states.review", "linear.states.done",
        "linear.labels.feasible", "linear.labels.needsReview", "linear.labels.needsSetup",
        "linear.labels.deployFailed", "linear.labels.blocked",
        "github.repo", "github.base", "github.evidenceBranch", "worktrees", "run", "evidence", "deploy.workflow",
    ]
    counts = [
        "caps.inFlight", "caps.awaitingApproval", "rounds.red", "rounds.green", "rounds.review", "rounds.evidence",
        "app.id", "app.installationId", "deploy.timeoutMinutes",
    ]
    for key in texts + counts + ["app.privateKeyPath"]:
        value = _get(config, key)
        if value is None:
            problems.append(f"{key} is missing")
        elif key in counts and (not isinstance(value, int) or isinstance(value, bool) or value <= 0):
            problems.append(f"{key} must be a whole number above 0")
        elif key in texts and (not isinstance(value, str) or not value.strip()):
            problems.append(f"{key} must be a text that is not blank")
    key_path = _get(config, "app.privateKeyPath")
    if key_path is not None and (not isinstance(key_path, str) or not key_path.startswith("/")):
        problems.append("app.privateKeyPath must be an absolute path")
    if problems:
        raise ConfigError(problems)
    return config


def _b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def _json_part(value: dict) -> str:
    return _b64url(json.dumps(value, separators=(",", ":")).encode("utf-8"))


def app_jwt(app_id: int, key_path: str, now: Optional[int] = None) -> str:
    """A GitHub App JWT: RS256, backdated 60 seconds for clock drift, valid ten minutes less the backdating."""
    now = int(time.time()) if now is None else now
    signing_input = f"{_json_part({'alg': 'RS256', 'typ': 'JWT'})}.{_json_part({'iat': now - 60, 'exp': now + 540, 'iss': app_id})}"
    signed = subprocess.run(
        ["openssl", "dgst", "-sha256", "-sign", key_path], input=signing_input.encode("ascii"), capture_output=True
    )
    if signed.returncode != 0:
        raise ConfigError([f"cannot sign with the App's private key {key_path}: {signed.stderr.decode().strip()}"])
    return f"{signing_input}.{_b64url(signed.stdout)}"


Post = Callable[[str, dict], Tuple[int, str]]


def _post(url: str, headers: dict) -> Tuple[int, str]:
    request = urllib.request.Request(url, data=b"", headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=30) as answer:
            return answer.status, answer.read().decode("utf-8")
    except urllib.error.HTTPError as answer:
        return answer.code, answer.read().decode("utf-8", "replace")


def installation_token(app: dict, post: Post = _post, now: Optional[int] = None) -> str:
    key_path = app["privateKeyPath"]
    if not Path(key_path).is_file():
        raise ConfigError([f"cannot read the App's private key at {key_path}"])
    jwt = app_jwt(app["id"], key_path, now)
    status, body = post(
        f"https://api.github.com/app/installations/{app['installationId']}/access_tokens",
        {
            "Authorization": f"Bearer {jwt}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "cityloom-factory",
        },
    )
    if status != 201:
        raise ConfigError([f"GitHub refused to mint an installation token: HTTP {status} {body[:200]}"])
    token = json.loads(body).get("token")
    if not isinstance(token, str):
        raise ConfigError(["GitHub's answer has no token"])
    return token


def main(argv: Optional[List[str]] = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if argv not in (["config"], ["token"]):
        print(__doc__.strip().split("\n\n")[1], file=sys.stderr)
        return 1
    try:
        config = load_config()
        if argv == ["config"]:
            print(json.dumps(config, ensure_ascii=False, indent=2))
        else:
            print(installation_token(config["app"]))
    except ConfigError as error:
        for problem in error.problems:
            print(problem, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
