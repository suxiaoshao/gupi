"""Create a draft from the packages built in this workflow run."""

import hashlib
import os
import subprocess
from pathlib import Path

tag = os.environ["GUPI_TAG"]
mode = os.environ["GUPI_MACOS_SIGNING"]
dist = Path("dist")
packages = sorted(path for path in dist.iterdir() if path.suffix in (".zip", ".msi", ".deb"))
if not packages:
    raise SystemExit("No release packages were downloaded")
checksums = dist / "SHA256SUMS"
with checksums.open("w") as output:
    for package in packages:
        with package.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        output.write(f"{digest}  {package.name}\n")
notes = Path(os.environ["RUNNER_TEMP"]) / "gupi-release-notes.md"
macos_status = (
    "Developer ID signed, notarized, and stapled"
    if mode == "developer-id"
    else "development ad-hoc signature; not notarized and subject to Gatekeeper restrictions"
)
notes.write_text(
    f"Gupi {tag} packages for review.\n\n"
    f"- macOS: {macos_status}.\n"
    "- Windows MSI and Linux deb packages are unsigned.\n"
    "- Pi is an external runtime and is not bundled.\n"
    "- Check platform installation and the current brand-asset permission before publishing this draft.\n"
)
existing = subprocess.run(
    ["gh", "release", "view", tag], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
)
if existing.returncode == 0:
    raise SystemExit("A release already exists for this tag; review it before replacing any assets")
subprocess.run(
    ["gh", "release", "create", tag, "--draft", "--verify-tag", "--latest=false",
     "--title", f"Gupi {tag}", "--notes-file", str(notes),
     *map(str, packages), str(checksums)],
    check=True,
)
