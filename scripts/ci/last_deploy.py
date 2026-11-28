#!/usr/bin/env python3
"""Prints the commit main last deployed, for postsubmit.yml's Changes job to diff a push from.

    GITHUB_REPOSITORY=<owner/name> GITHUB_RUN_ID=<this run> scripts/ci/last_deploy.py

The commit is the head of the newest Postsubmit run on main, for a push, older than this run, whose `Deploy to Railway`
job succeeded. A run that succeeded with its deploy skipped (nothing it changed is served) deployed nothing, so the job
is read, not the run. Diffing from that commit, not from the push's parent, keeps a site change whose own run failed in
the diff of the next push.

Prints nothing when it finds none or GitHub cannot be asked (the reason goes to stderr), and always exits 0: an unknown
base makes scripts/ci/ci_changes.py deploy. Needs `gh` with a token that can read Actions (GH_TOKEN).

Standard library only; scripts/include/tests/test_last_deploy.py tests it.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from collections.abc import Callable

DEPLOY_JOB = "Deploy to Railway"
RUNS = "repos/{repo}/actions/workflows/postsubmit.yml/runs?branch=main&event=push&per_page=30"
JOBS = "repos/{repo}/actions/runs/{run}/jobs?per_page=100"


def last_deployed(runs: list[dict], deploy_conclusion: Callable[[int], str | None], before: int) -> str:
    """The head commit of the newest run older than `before` whose deploy succeeded; "" when none did.

    `runs` is GitHub's `workflow_runs`, newest first; `deploy_conclusion(run_id)` is that run's deploy job's conclusion.
    """
    for run in runs:
        if run["id"] < before and deploy_conclusion(run["id"]) == "success":
            return run["head_sha"]
    return ""


def gh_api(path: str) -> dict:
    done = subprocess.run(["gh", "api", path], capture_output=True, text=True, check=True)
    return json.loads(done.stdout)


def main() -> int:
    repo, run = os.environ.get("GITHUB_REPOSITORY", ""), os.environ.get("GITHUB_RUN_ID", "")
    try:
        runs = gh_api(RUNS.format(repo=repo))["workflow_runs"]

        def conclusion(run_id: int) -> str | None:
            jobs = gh_api(JOBS.format(repo=repo, run=run_id))["jobs"]
            return next((j["conclusion"] for j in jobs if j["name"] == DEPLOY_JOB), None)

        sha = last_deployed(runs, conclusion, before=int(run))
    except (subprocess.CalledProcessError, OSError, ValueError, KeyError, TypeError) as err:
        detail = err.stderr.strip() if isinstance(err, subprocess.CalledProcessError) else str(err)
        print(f"last_deploy.py: GitHub could not be asked ({detail}): no base, the push deploys", file=sys.stderr)
        return 0
    if sha:
        print(sha)
    else:
        print("last_deploy.py: no earlier deploy found: no base, the push deploys", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
