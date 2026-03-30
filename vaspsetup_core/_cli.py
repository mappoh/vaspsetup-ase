"""CLI entry point — locates or downloads the Rust binary, then launches it."""

import os
import platform
import stat
import sys
import tarfile
import urllib.request
from pathlib import Path

REPO = "mappoh/vaspsetup-ase"
VERSION = "v0.2.14"


def _bin_dir() -> Path:
    """Directory where the binary is cached."""
    cache = Path.home() / ".cache" / "vaspsetup"
    cache.mkdir(parents=True, exist_ok=True)
    return cache


def _find_binary() -> Path:
    """Find or download the vaspsetup binary."""
    # 1. Check cached binary (version-aware)
    cached = _bin_dir() / "vaspsetup"
    version_file = _bin_dir() / "version"
    if cached.is_file() and os.access(cached, os.X_OK):
        if version_file.is_file() and version_file.read_text().strip() == VERSION:
            return cached
        # Stale binary — remove and re-download
        cached.unlink()
        version_file.unlink(missing_ok=True)

    # 2. Check development build (cargo build)
    project_root = Path(__file__).parent.parent
    for candidate in [
        project_root / "target" / "release" / "vaspsetup",
        project_root / "target" / "debug" / "vaspsetup",
    ]:
        if candidate.is_file():
            return candidate

    # 3. Download from GitHub Releases
    return _download_binary()


def _download_binary() -> Path:
    """Download the pre-built binary from GitHub Releases."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system != "linux" or machine not in ("x86_64", "amd64"):
        print(
            f"Error: No pre-built binary for {system}/{machine}.\n"
            "Build from source: cargo build --release",
            file=sys.stderr,
        )
        sys.exit(1)

    url = (
        f"https://github.com/{REPO}/releases/download/{VERSION}/"
        f"vaspsetup-{VERSION}-linux-x86_64.tar.gz"
    )

    dest = _bin_dir() / "vaspsetup"
    tarball = _bin_dir() / "vaspsetup.tar.gz"

    print(f"Downloading vaspsetup {VERSION}...", file=sys.stderr)
    try:
        urllib.request.urlretrieve(url, tarball)
    except Exception as e:
        print(f"Error: Failed to download binary: {e}", file=sys.stderr)
        print(f"URL: {url}", file=sys.stderr)
        print("Build from source: cargo build --release", file=sys.stderr)
        sys.exit(1)

    # Extract just the binary from the tarball
    try:
        with tarfile.open(tarball, "r:gz") as tar:
            member = tar.getmember("vaspsetup")
            f = tar.extractfile(member)
            if f is None:
                raise RuntimeError("vaspsetup not found in tarball")
            dest.write_bytes(f.read())
            dest.chmod(dest.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)
    except Exception as e:
        print(f"Error: Failed to extract binary: {e}", file=sys.stderr)
        sys.exit(1)
    finally:
        tarball.unlink(missing_ok=True)

    # Record the version so upgrades invalidate the cache
    (_bin_dir() / "version").write_text(VERSION)

    print(f"Installed to {dest}", file=sys.stderr)
    return dest


def main():
    """Launch the vaspsetup TUI binary."""
    binary = _find_binary()

    # Set PYTHONPATH so the binary can find vaspsetup_core
    pkg_parent = str(Path(__file__).parent.parent)
    existing = os.environ.get("PYTHONPATH", "")
    if existing:
        os.environ["PYTHONPATH"] = f"{pkg_parent}:{existing}"
    else:
        os.environ["PYTHONPATH"] = pkg_parent

    # Replace this process with the binary
    os.execvp(str(binary), [str(binary)] + sys.argv[1:])


if __name__ == "__main__":
    main()
