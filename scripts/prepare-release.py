#!/usr/bin/env python3
"""Prepare a patch release, or resume the release commit after a failed upload."""

import datetime
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
import urllib.error
import urllib.request


def next_version(current, subject, published):
    if subject == f"chore(release): v{current}":
        return current
    major, minor, patch = map(int, current.split("."))
    candidate = f"{major}.{minor}.{patch + 1}"
    while candidate in published:
        patch += 1
        candidate = f"{major}.{minor}.{patch + 1}"
    return candidate


def release_changelog(contents, version, date):
    marker = "## Unreleased\n"
    if contents.count(marker) != 1:
        raise ValueError("Expected exactly one Unreleased section in CHANGELOG.md")
    return contents.replace(marker, f"{marker}\n## {version} - {date}\n", 1)


def main():
    manifest_path = Path("Cargo.toml")
    manifest_text = manifest_path.read_text()
    package = tomllib.loads(manifest_text)["package"]
    current = package["version"]
    request = urllib.request.Request(
        f"https://crates.io/api/v1/crates/{package['name']}",
        headers={"User-Agent": "aai-cli-release (github.com/aai-labs/aai-cli)"},
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            published = {item["num"] for item in json.load(response)["versions"]}
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
        published = set()
    subject = subprocess.check_output(["git", "log", "-1", "--format=%s"], text=True).strip()
    version = next_version(current, subject, published)
    if version != current:
        updated, count = re.subn(
            rf'^version = "{re.escape(current)}"$',
            f'version = "{version}"', manifest_text, count=1, flags=re.MULTILINE,
        )
        if count != 1:
            raise ValueError("Could not update package version")
        changelog = Path("CHANGELOG.md")
        contents = release_changelog(
            changelog.read_text(), version, datetime.datetime.now(datetime.timezone.utc).date()
        )
        manifest_path.write_text(updated)
        changelog.write_text(contents)
    outputs = f"version={version}\npublished={str(version in published).lower()}\n"
    print(outputs, end="")
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(outputs)


if __name__ == "__main__":
    main()
