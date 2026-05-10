#!/usr/bin/env python3
"""Packaging helpers shared by Station Scout's Nuitka build path."""

from __future__ import annotations

import platform
import shutil
import subprocess
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INSTALLER_DIR = ROOT / "installer"
DIST_DIR = ROOT / "dist"
APP_NAME = "StationScout"
DISPLAY_NAME = "Station Scout"

IS_WINDOWS = platform.system() == "Windows"
IS_MACOS = platform.system() == "Darwin"
IS_LINUX = platform.system() == "Linux"


def run_command(cmd: list[str], *, cwd: Path | None = None) -> None:
    print(f"$ {' '.join(cmd)}")
    subprocess.run(cmd, cwd=cwd or ROOT, check=True)


def get_version() -> str:
    import tomllib

    with (ROOT / "pyproject.toml").open("rb") as handle:
        return tomllib.load(handle)["project"]["version"]


def create_windows_installer() -> bool:
    iss_file = INSTALLER_DIR / "station-scout.iss"
    iscc_exe = None
    for candidate in (
        r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        r"C:\Program Files\Inno Setup 6\ISCC.exe",
        "iscc",
    ):
        if Path(candidate).exists() or shutil.which(candidate):
            iscc_exe = candidate
            break
    if not iscc_exe:
        print("Inno Setup not found; skipping installer creation.")
        return False

    try:
        run_command([iscc_exe, str(iss_file)], cwd=INSTALLER_DIR)
        return True
    except subprocess.CalledProcessError as exc:
        print(f"Inno Setup failed: {exc}")
        return False


def create_portable_archive() -> bool:
    version = get_version()
    if IS_WINDOWS:
        source_dir = DIST_DIR / f"{APP_NAME}_dir"
        archive_base = DIST_DIR / f"{APP_NAME}_Portable_v{version}"
        if not source_dir.exists():
            print(f"Windows build output not found: {source_dir}")
            return False
        (source_dir / ".portable").write_text("1\n", encoding="utf-8")
        (source_dir / "config").mkdir(exist_ok=True)
        archive_file = Path(f"{archive_base}.zip")
        if archive_file.exists():
            archive_file.unlink()
        shutil.make_archive(str(archive_base), "zip", source_dir.parent, source_dir.name)
    elif IS_MACOS:
        source_dir = DIST_DIR / f"{APP_NAME}.app"
        archive_base = DIST_DIR / f"{APP_NAME}_macOS_v{version}"
        if not source_dir.exists():
            print(f"macOS app bundle not found: {source_dir}")
            return False
        archive_file = Path(f"{archive_base}.zip")
        if archive_file.exists():
            archive_file.unlink()
        shutil.make_archive(str(archive_base), "zip", source_dir.parent, source_dir.name)
    else:
        source_dir = DIST_DIR / APP_NAME
        archive_file = DIST_DIR / f"{APP_NAME}_Linux_v{version}.tar.gz"
        if not source_dir.exists():
            print(f"Linux build output not found: {source_dir}")
            return False
        if archive_file.exists():
            archive_file.unlink()
        with tarfile.open(archive_file, "w:gz") as archive:
            archive.add(source_dir, arcname=source_dir.name)

    print(f"Created portable artifact: {archive_file}")
    return True


def main() -> int:
    if not create_portable_archive():
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
