"""Check the package contract for one release matrix target."""

import os
import tomllib
from pathlib import Path

target = os.environ["GUPI_TARGET"]
root = Path("dist") / target
files = list(root.iterdir())
if "apple-darwin" in target:
    archives = [path for path in files if path.suffix == ".zip"]
    if len(archives) != 1:
        raise SystemExit("Expected one macOS application ZIP")
    development = os.environ["GUPI_MACOS_SIGNING"] == "development"
    if archives[0].stem.endswith("_development") != development:
        raise SystemExit("macOS artifact filename must reflect its signature mode")
elif "windows-msvc" in target:
    locales = tomllib.loads(Path("Cargo.toml").read_text())["package"]["metadata"]["bundle"]["localizations"]
    cultures = {"ja": "ja-JP", "ko": "ko-KR", "de": "de-DE", "fr": "fr-FR", "es": "es-ES"}
    expected = {cultures.get(locale, locale) for locale in locales}
    installers = [path for path in files if path.suffix == ".msi"]
    actual = {path.stem.rsplit("_", 1)[-1] for path in installers}
    if actual != expected or len(installers) != len(expected):
        raise SystemExit(f"MSI cultures differ: expected {sorted(expected)}, got {sorted(actual)}")
elif "linux-gnu" in target:
    if len([path for path in files if path.suffix == ".deb"]) != 1:
        raise SystemExit("Expected one Linux Debian package")
else:
    raise SystemExit(f"Unexpected release target: {target}")
print(f"Verified {len(files)} package files for {target}")
