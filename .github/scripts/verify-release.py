"""Verify that release packages will be built from an existing version tag."""

import os
import subprocess
import tomllib
from pathlib import Path

tag = os.environ["GUPI_TAG"]
mode = os.environ["GUPI_MACOS_SIGNING"]
manifest = tomllib.loads(Path("Cargo.toml").read_text())
version = manifest["package"]["version"]
if tag != f"v{version}":
    raise SystemExit(f"Tag {tag!r} must match Cargo.toml package version v{version}")
if mode not in ("development", "developer-id"):
    raise SystemExit(f"Unsupported macOS signing mode: {mode!r}")
subprocess.run(["git", "check-ref-format", f"refs/tags/{tag}"], check=True)
sha = subprocess.check_output(
    ["git", "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}"], text=True
).strip()
head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
if sha != head:
    raise SystemExit("Checkout HEAD must match the existing version tag")
with open(os.environ["GITHUB_OUTPUT"], "a") as output:
    output.write(f"tag={tag}\nsha={sha}\nmacos_signing={mode}\n")
print(f"Verified Gupi {version} at {sha}; macOS signing: {mode}")
