"""Skeleton smoke tests: prove the harness runs before any product code exists."""

from pathlib import Path

import pytest


@pytest.mark.covers("OPS-015")
def test_repo_root_has_a_justfile(repo_root: Path) -> None:
    """The single gate is discoverable from the repository root."""
    assert (repo_root / "justfile").is_file()


@pytest.mark.covers("OPS-015")
def test_feature_inventory_exists(repo_root: Path) -> None:
    """Future agents must always be able to find the requirement list."""
    features = repo_root / "docs" / "state" / "features.md"
    assert features.is_file()
    assert "[FAILING]" in features.read_text(encoding="utf-8")
