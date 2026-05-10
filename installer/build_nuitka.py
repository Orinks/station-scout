#!/usr/bin/env python3
"""Build Station Scout desktop artifacts with Nuitka."""

from __future__ import annotations

import argparse
import importlib.util
import platform
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIST_DIR = ROOT / "dist"
BUILD_DIR = ROOT / "build" / "nuitka"
APP_NAME = "StationScout"
DISPLAY_NAME = "Station Scout"
LINUX_SYSTEM_DLL_EXCLUDES = (
    "libcrypto.so*",
    "libgio-2.0.so*",
    "libglib-2.0.so*",
    "libgmodule-2.0.so*",
    "libgobject-2.0.so*",
    "libgthread-2.0.so*",
    "libssl.so*",
)
SOUND_LIB_NATIVE_EXTS = {".dll", ".dylib", ".so"}
SOUND_LIB_ARCH_DIR = "x64"

if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


def get_version() -> str:
    with (ROOT / "pyproject.toml").open("rb") as handle:
        return tomllib.load(handle)["project"]["version"]


def _nuitka_version(version: str) -> str:
    base = version.split("+", 1)[0].split(".dev", 1)[0].split("a", 1)[0].split("b", 1)[0]
    parts = [part for part in base.split(".") if part.isdigit()]
    return ".".join((parts + ["0", "0", "0", "0"])[:4])


def _repo_path(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def _sound_lib_lib_dir() -> Path:
    spec = importlib.util.find_spec("sound_lib")
    if not spec or not spec.submodule_search_locations:
        raise RuntimeError("sound_lib is not installed; cannot stage packaged audio support")
    lib_dir = Path(next(iter(spec.submodule_search_locations))) / "lib"
    if not lib_dir.exists():
        raise RuntimeError(f"sound_lib native library directory was not found: {lib_dir}")
    return lib_dir


def _sound_lib_target_dir(staged_dir: Path) -> Path:
    if staged_dir.suffix == ".app":
        return staged_dir / "Contents" / "Resources" / "sound_lib" / "lib"
    return staged_dir / "sound_lib" / "lib"


def _mirror_sound_lib_flat_files_to_arch_dir(target_dir: Path) -> None:
    flat_files = [path for path in target_dir.iterdir() if path.is_file()]
    if not flat_files:
        return
    arch_dir = target_dir / SOUND_LIB_ARCH_DIR
    arch_dir.mkdir(exist_ok=True)
    for path in flat_files:
        shutil.copy2(path, arch_dir / path.name)


def _stage_sound_lib_runtime_files(staged_dir: Path) -> None:
    source_dir = _sound_lib_lib_dir()
    target_dir = _sound_lib_target_dir(staged_dir)
    if target_dir.exists():
        shutil.rmtree(target_dir)
    target_dir.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source_dir, target_dir)
    _mirror_sound_lib_flat_files_to_arch_dir(target_dir)
    native_files = [
        path
        for path in target_dir.rglob("*")
        if path.is_file() and path.suffix.lower() in SOUND_LIB_NATIVE_EXTS
    ]
    if not native_files:
        raise RuntimeError(f"No sound_lib native libraries were staged under {target_dir}")


def write_inno_version_file() -> Path:
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    version_file = DIST_DIR / "version.txt"
    version_file.write_text(f"[version]\nvalue={get_version()}\n", encoding="utf-8")
    return version_file


def _find_nuitka_output_dir() -> tuple[Path, str]:
    for candidate in sorted(BUILD_DIR.glob("*.dist"), key=lambda path: path.stat().st_mtime, reverse=True):
        if (candidate / f"{APP_NAME}.exe").exists() or (candidate / APP_NAME).exists():
            return candidate, "dist"
    for candidate in sorted(BUILD_DIR.glob("*.app"), key=lambda path: path.stat().st_mtime, reverse=True):
        if (candidate / "Contents" / "MacOS" / APP_NAME).exists():
            return candidate, "app"
    raise FileNotFoundError(f"Nuitka output was not found under {BUILD_DIR}")


def stage_nuitka_distribution() -> Path:
    source_dir, output_kind = _find_nuitka_output_dir()
    if output_kind == "app":
        target_name = f"{APP_NAME}.app"
    elif platform.system() == "Linux":
        target_name = APP_NAME
    else:
        target_name = f"{APP_NAME}_dir"
    target_dir = DIST_DIR / target_name
    if target_dir.exists():
        shutil.rmtree(target_dir)
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source_dir, target_dir)
    _stage_sound_lib_runtime_files(target_dir)
    return target_dir


def build_nuitka_command(*, output_dir: Path, build_tag: str | None, assume_platform: str | None = None) -> list[str]:
    system = assume_platform or platform.system()
    version = get_version()
    numeric_version = _nuitka_version(version)
    mode = "--mode=app" if system == "Darwin" else "--mode=standalone"
    command = [
        sys.executable,
        "-m",
        "nuitka",
        mode,
        "--assume-yes-for-downloads",
        "--noinclude-pytest-mode=nofollow",
        "--include-package=station_scout",
        "--include-package-data=sound_lib",
        f"--output-dir={output_dir.as_posix()}",
        f"--output-filename={APP_NAME}",
        f"--report={(output_dir / 'compilation-report.xml').as_posix()}",
        f"--product-name={DISPLAY_NAME}",
        f"--file-description={DISPLAY_NAME}",
        f"--product-version={numeric_version}",
        f"--file-version={numeric_version}",
        f"--company-name=Orinks{f' ({build_tag})' if build_tag else ''}",
    ]
    for optional_package in ("desktop_notifier", "toasted"):
        if importlib.util.find_spec(optional_package):
            command.append(f"--include-package-data={optional_package}")
    if system == "Windows":
        command.append("--windows-console-mode=disable")
    elif system == "Darwin":
        command.append(f"--macos-app-name={DISPLAY_NAME}")
    elif system == "Linux":
        command.extend(f"--noinclude-dlls={pattern}" for pattern in LINUX_SYSTEM_DLL_EXCLUDES)
    command.append(_repo_path(ROOT / "installer" / "nuitka_entry.py"))
    return command


def run_command(command: list[str]) -> None:
    print("Running:", " ".join(command))
    subprocess.run(command, cwd=ROOT, check=True)


def ensure_nuitka_available() -> None:
    try:
        subprocess.run(
            [sys.executable, "-m", "nuitka", "--version"],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise RuntimeError("Nuitka is not installed. Run: pip install nuitka") from exc


def create_portable_archive() -> bool:
    from installer import build

    return build.create_portable_archive()


def create_windows_installer() -> bool:
    from installer import build

    return build.create_windows_installer()


def main() -> int:
    parser = argparse.ArgumentParser(description="Build Station Scout with Nuitka.")
    parser.add_argument("--tag", default=None, help="Build tag, e.g. nightly-20260510.")
    parser.add_argument("--skip-compile", action="store_true", help="Only write version metadata.")
    parser.add_argument("--skip-installer", action="store_true", help="Skip Windows installer creation.")
    args = parser.parse_args()

    print(f"Station Scout Nuitka build, version {get_version()}")
    write_inno_version_file()
    if args.skip_compile:
        return 0

    ensure_nuitka_available()
    run_command(build_nuitka_command(output_dir=BUILD_DIR, build_tag=args.tag))
    stage_nuitka_distribution()
    if platform.system() == "Windows" and not args.skip_installer and not create_windows_installer():
        return 1
    if not create_portable_archive():
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
