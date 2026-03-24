"""CLI entry point — locates and launches the bundled Rust binary."""

import os
import platform
import stat
import sys
from pathlib import Path


def _find_binary() -> Path:
    """Find the vaspsetup binary bundled in this package."""
    # Check package's bin/ directory (installed via pip/uv)
    pkg_bin = Path(__file__).parent / "bin" / "vaspsetup"
    if pkg_bin.is_file():
        # Ensure executable permission
        if not os.access(pkg_bin, os.X_OK):
            pkg_bin.chmod(pkg_bin.stat().st_mode | stat.S_IEXEC)
        return pkg_bin

    # Check project root (development: cargo build)
    project_root = Path(__file__).parent.parent
    for candidate in [
        project_root / "target" / "release" / "vaspsetup",
        project_root / "target" / "debug" / "vaspsetup",
    ]:
        if candidate.is_file():
            return candidate

    print(
        "Error: vaspsetup binary not found.\n"
        "If developing, run: cargo build --release\n"
        "If installed via pip/uv, the package may be missing the binary for your platform.",
        file=sys.stderr,
    )
    sys.exit(1)


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
