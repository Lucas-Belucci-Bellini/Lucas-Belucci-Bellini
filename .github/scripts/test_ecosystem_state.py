#!/usr/bin/env python3
"""Pure tests for the monitor V2 accounting model."""
from __future__ import annotations

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("ecosystem_watch", ROOT / ".github/scripts/ecosystem_watch.py")
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def test_schema5_migration_does_not_double_count_legacy_baseline():
    state = {
        "schema": 5,
        "metrics": {
            "project_commits": 1542,
            "monitor_commits": 3,
        },
    }
    metrics, baseline = MODULE.migrate_metrics(state)
    assert baseline["legacy_tracked_commits"] == 1538
    assert metrics["project_commits_total"] == 4
    assert metrics["monitor_commits_total"] == 3


def test_no_project_change_still_adds_one_monitor_snapshot():
    result = MODULE.calculate_metrics(
        {"schema": 6, "baseline": {"legacy_tracked_commits": 1538}, "metrics": {"project_commits_total": 4, "monitor_commits_total": 3}},
        0,
    )
    assert result["project_commits_total"] == 4
    assert result["monitor_commits_total"] == 4
    assert result["tracked_commits_total"] == 1546
    assert result["monitor_commit_this_run"] == 1


def test_project_changes_and_monitor_snapshot_are_separate():
    result = MODULE.calculate_metrics(
        {"schema": 6, "baseline": {"legacy_tracked_commits": 1538}, "metrics": {"project_commits_total": 4, "monitor_commits_total": 3}},
        12,
    )
    assert result["project_commits_total"] == 16
    assert result["monitor_commits_total"] == 4
    assert result["tracked_commits_total"] == 1558


def test_legacy_baseline_is_not_project_commits():
    result = MODULE.calculate_metrics({}, 0)
    assert result["legacy_baseline"] == 1538
    assert result["project_commits_total"] == 0
    assert result["monitor_commits_total"] == 1
    assert result["tracked_commits_total"] == 1539


def test_invariant_tracked_equals_baseline_plus_categories():
    result = MODULE.calculate_metrics(
        {"schema": 6, "baseline": {"legacy_tracked_commits": 1538}, "metrics": {"project_commits_total": 100, "monitor_commits_total": 50}},
        7,
    )
    assert result["tracked_commits_total"] == 1538 + result["project_commits_total"] + result["monitor_commits_total"]


if __name__ == "__main__":
    tests = [
        test_schema5_migration_does_not_double_count_legacy_baseline,
        test_no_project_change_still_adds_one_monitor_snapshot,
        test_project_changes_and_monitor_snapshot_are_separate,
        test_legacy_baseline_is_not_project_commits,
        test_invariant_tracked_equals_baseline_plus_categories,
    ]
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"{len(tests)} tests passed")
