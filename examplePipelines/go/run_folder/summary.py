"""Sums up what the run's other tasks left in the run folder, then keeps the summary as a pipeline object.

The run folder is deleted when the run ends, so anything worth looking at later has to go somewhere that lasts.
"""

import json
import os
import pathlib
import urllib.request

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


def keep(summary):
    url = (
        f"{os.environ['GOFER_API_BASE_URL']}/api/namespaces/{os.environ['GOFER_NAMESPACE']}"
        f"/pipelines/{os.environ['GOFER_PIPELINE_ID']}/objects/summary?force=true"
    )
    request = urllib.request.Request(
        url,
        data=summary.encode(),
        method="POST",
        headers={
            "Authorization": f"Bearer {os.environ['GOFER_TOKEN']}",
            "gofer-api-version": "v0",
            "Content-Type": "application/octet-stream",
        },
    )
    urllib.request.urlopen(request).close()


summary = f"""# Run {os.environ["GOFER_RUN_ID"]} at {read_version()}

## Tests

{summarize_tests()}

## Release

{summarize_release()}
"""

print(summary)
keep(summary)
print(f"Kept as the pipeline object 'summary'. To read it: gofer pipeline object get summary "
      f"--pipeline {os.environ['GOFER_PIPELINE_ID']} --stringify")
