"""Sums up what the run's other tasks left in the run folder, into /gofer/run/summary.md."""

import json
import os
import pathlib

RUN = pathlib.Path("/gofer/run")


def read_version():
    path = RUN / "version"
    return path.read_text().strip() if path.exists() else "unknown"


def summarize_tests():
    report = RUN / "reports" / "tests.json"
    if not report.exists():
        return "No test results; the test task didn't get far enough to write them."

    counts = {"pass": 0, "fail": 0, "skip": 0}
    failed = []
    for line in report.read_text().splitlines():
        event = json.loads(line)
        action = event.get("Action")
        if action not in counts:
            continue

        # Events without a test name are for the whole package, like one that didn't compile.
        name = event.get("Test")
        if name is None:
            if action == "fail":
                failed.append(f"{event['Package']} (the whole package)")
            continue

        counts[action] += 1
        if action == "fail":
            failed.append(f"{event['Package']}.{name}")

    summary = f"{counts['pass']} passed, {counts['fail']} failed, {counts['skip']} skipped."
    for name in failed:
        summary += f"\n- {name}"
    return summary


def summarize_release():
    release = RUN / "release"
    if not release.exists():
        return "Nothing was packaged."

    lines = []
    for path in sorted(release.iterdir()):
        size = path.stat().st_size
        lines.append(f"- {path.name} ({size} bytes)" if size < 1024 else f"- {path.name} ({size / 1024:.0f} KiB)")
    return "\n".join(lines)


summary = f"""# Run {os.environ["GOFER_RUN_ID"]} at {read_version()}

## Tests

{summarize_tests()}

## Release

{summarize_release()}
"""

(RUN / "summary.md").write_text(summary)
print(summary)
