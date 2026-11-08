"""Copy the parts of GStreamer that Station Scout needs next to StationScout.exe.

Only the audio playback plugins in PLUGINS are bundled. The DLLs from
GStreamer's bin folder are copied only when the exe, a bundled plugin, or a
bundled GIO module imports them, directly or indirectly. That keeps the
Windows download small without hand-maintaining a DLL list.

Layout (matches rust/src/gst_runtime.rs):
  <dest>/*.dll                                     core and dependency DLLs
  <dest>/lib/gstreamer-1.0/*.dll                   plugins
  <dest>/lib/gio/modules/*.dll                     GIO TLS module (HTTPS)
  <dest>/libexec/gstreamer-1.0/gst-plugin-scanner.exe

Usage: python scripts/bundle_gstreamer_windows.py --gst-root DIR --dest DIR
"""

from __future__ import annotations

import argparse
import os
import shutil
import struct
import sys
from pathlib import Path

# Plugins for internet radio and podcast audio. Names are without the "gst"
# prefix and ".dll" suffix. Missing ones are skipped with a note, since
# plugin names shift between GStreamer releases.
PLUGINS = [
    # Pipeline basics
    "coreelements", "playback", "typefindfunctions", "app", "autodetect",
    "audioconvert", "audioresample", "volume", "audiofx", "audioparsers",
    # Audio output
    "wasapi", "wasapi2", "directsound",
    # Network sources and streaming formats
    "soup", "gio", "tcp", "udp", "adaptivedemux2", "hls", "dash",
    "rtsp", "rtp", "rtpmanager", "srtp",
    # Containers and tags
    "isomp4", "matroska", "ogg", "mpegtsdemux", "id3demux", "icydemux",
    "apetag", "wavparse", "aiff", "asf",
    # Decoders
    "mpg123", "libav", "fdkaac", "opus", "opusparse", "vorbis", "flac",
    "speex", "wavpack",
]

# DLLs that plugins open at runtime instead of importing. The soup plugin
# loads libsoup this way, so the import walk alone would miss it.
RUNTIME_LOADED_DLLS = ["libsoup-3.0-0.dll", "libsoup-2.4-1.dll"]

# Windows DLLs that are always present; never copied from GStreamer.
SYSTEM_DLL_PREFIXES = ("api-ms-win-", "ext-ms-")

MSVC_RUNTIME = ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"]


def pe_imports(path: Path) -> list[str]:
    """Names of the DLLs a PE file imports (normal and delay-load)."""
    data = path.read_bytes()
    if data[:2] != b"MZ":
        return []
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe : pe + 4] != b"PE\0\0":
        return []
    coff = pe + 4
    sections_count = struct.unpack_from("<H", data, coff + 2)[0]
    opt_size = struct.unpack_from("<H", data, coff + 16)[0]
    opt = coff + 20
    magic = struct.unpack_from("<H", data, opt)[0]
    dirs = opt + (112 if magic == 0x20B else 96)
    sections = []
    for i in range(sections_count):
        sec = opt + opt_size + i * 40
        vsize, vaddr, rawsize, rawptr = struct.unpack_from("<IIII", data, sec + 8)
        sections.append((vaddr, max(vsize, rawsize), rawptr))

    def offset(rva: int) -> int | None:
        for vaddr, size, rawptr in sections:
            if vaddr <= rva < vaddr + size:
                return rva - vaddr + rawptr
        return None

    def cstring(rva: int) -> str | None:
        off = offset(rva)
        if off is None:
            return None
        end = data.index(b"\0", off)
        return data[off:end].decode("ascii", "replace")

    names = []
    # Data directory 1: import table (20-byte entries, name RVA at +12).
    # Data directory 13: delay-load imports (32-byte entries, name RVA at +4).
    for index, entry_size, name_field in ((1, 20, 12), (13, 32, 4)):
        rva, _size = struct.unpack_from("<II", data, dirs + index * 8)
        off = offset(rva) if rva else None
        while off is not None and off + entry_size <= len(data):
            entry = data[off : off + entry_size]
            if not any(entry):
                break
            name_rva = struct.unpack_from("<I", entry, name_field)[0]
            name = cstring(name_rva) if name_rva else None
            if name:
                names.append(name)
            off += entry_size
    return names


def copy(src: Path, dest_dir: Path) -> None:
    dest_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest_dir / src.name)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gst-root", required=True, type=Path)
    parser.add_argument("--dest", required=True, type=Path)
    parser.add_argument("--exe", default="StationScout.exe")
    args = parser.parse_args()

    gst_bin = args.gst_root / "bin"
    plugin_src = args.gst_root / "lib" / "gstreamer-1.0"
    gio_src = args.gst_root / "lib" / "gio" / "modules"
    scanner = args.gst_root / "libexec" / "gstreamer-1.0" / "gst-plugin-scanner.exe"
    for folder in (gst_bin, plugin_src):
        if not folder.is_dir():
            sys.exit(f"GStreamer folder not found: {folder}")

    available = {p.name.lower(): p for p in gst_bin.glob("*.dll")}
    roots: list[Path] = [args.dest / args.exe]

    plugin_dest = args.dest / "lib" / "gstreamer-1.0"
    for name in PLUGINS:
        plugin = plugin_src / f"gst{name}.dll"
        if plugin.is_file():
            copy(plugin, plugin_dest)
            roots.append(plugin)
        else:
            print(f"Plugin not in this GStreamer build, skipping: {name}")

    if gio_src.is_dir():
        for module in gio_src.glob("*.dll"):
            copy(module, args.dest / "lib" / "gio" / "modules")
            roots.append(module)
    if scanner.is_file():
        copy(scanner, args.dest / "libexec" / "gstreamer-1.0")
        roots.append(scanner)

    for name in RUNTIME_LOADED_DLLS:
        if name.lower() in available:
            roots.append(available[name.lower()])

    # Walk imports and copy every GStreamer DLL that something needs.
    needed: set[str] = set()
    queue = list(roots)
    while queue:
        for name in pe_imports(queue.pop()):
            key = name.lower()
            if key.startswith(SYSTEM_DLL_PREFIXES) or key in needed:
                continue
            if key in available:
                needed.add(key)
                queue.append(available[key])
    for root in roots:
        if root.parent == gst_bin:
            needed.add(root.name.lower())
    for key in sorted(needed):
        copy(available[key], args.dest)

    # MSVC runtime, in case the PC lacks the VC++ redistributable.
    # Microsoft allows app-local copies of these files.
    system32 = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32"
    for dll in MSVC_RUNTIME:
        src = system32 / dll
        if src.is_file() and not (args.dest / dll).exists():
            copy(src, args.dest)

    plugins = len(list(plugin_dest.glob("*.dll")))
    size = sum(f.stat().st_size for f in args.dest.rglob("*") if f.is_file())
    print(f"Bundled {plugins} plugins and {len(needed)} GStreamer DLLs.")
    print(f"Bundle size: {size / 1024 / 1024:.1f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
