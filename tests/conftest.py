"""Shared pytest fixtures for the CityLoom harness.

Task 046 fills in the real fixtures:
  - ``pg_database``  isolated Postgres database per test (TEST-002)
  - ``server``       server on an ephemeral port, healthchecked (TEST-003/004)
  - ``scene_state``  reads ``window.__cityloom.scene_state()`` (TEST-006)

Until then this module only exposes repository paths, so the skeleton suite
collects and passes without Postgres or a browser present.
"""

from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture(scope="session")
def repo_root() -> Path:
    """Absolute path to the repository root."""
    return REPO_ROOT
