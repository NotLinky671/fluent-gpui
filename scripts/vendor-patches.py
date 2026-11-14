#!/usr/bin/env python3
"""Export/check reproducible GPUI patches against checksum-verified .crate archives.

No downloads. Use Cargo's registry cache, or --cache to point at an archive
folder. --check regenerates in memory, then reconstructs in a temporary folder
and compares every retained file. It never applies a patch to the working tree.
"""
import argparse
import difflib
import hashlib
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = {
    "gpui-base": ("0.8.1", "d862bc8f2e27710039013d075ac4995892e9687ff207fa530f880cbcd513936f"),
    "gpui-pre": ("0.3.10", "df4b5f8d0c278308df2f11e0bfe0212dd77d7848ce1bdc4b9d513334414396f6"),
}
EXCLUDED = {"tests", "benches", "examples", "Cargo.lock", ".cargo-ok"}


def normalized(data):
    # Cargo text sources are UTF-8; normalize checkout CRLF before diffing.
    try:
        return data.decode("utf-8").replace("\r\n", "\n").encode("utf-8")
    except UnicodeDecodeError:
        return data


def upstream(name, version, checksum, cache):
    filename = f"{name}-{version}.crate"
    matches = list(cache.rglob(filename))
    archives = [p for p in matches if hashlib.sha256(p.read_bytes()).hexdigest() == checksum]
    if not archives:
        raise RuntimeError(f"Missing checksum-matching {filename} in {cache}; run cargo fetch --locked first")
    prefix = f"{name}-{version}/"
    files = {}
    with tarfile.open(archives[0], "r:gz") as archive:
        for member in archive.getmembers():
            if not member.isfile() or not member.name.startswith(prefix):
                continue
            relative = member.name[len(prefix):]
            parts = Path(relative).parts
            if not parts or parts[0] in EXCLUDED or ".." in parts or Path(relative).is_absolute():
                continue
            with archive.extractfile(member) as stream:
                files[relative] = normalized(stream.read())
    return files


def local(name):
    base = ROOT / "vendor" / name
    return {p.relative_to(base).as_posix(): normalized(p.read_bytes())
            for p in base.rglob("*") if p.is_file()}


def make_patch(name, original, modified):
    patch = []
    for relative in sorted(original.keys() | modified.keys()):
        before, after = original.get(relative, b""), modified.get(relative, b"")
        if before == after:
            continue
        if (before and not before.endswith(b"\n")) or (after and not after.endswith(b"\n")):
            raise RuntimeError(f"Cannot represent missing final newline: {name}/{relative}")
        source = f"a/{name}/{relative}" if relative in original else "/dev/null"
        target = f"b/{name}/{relative}" if relative in modified else "/dev/null"
        patch.extend(difflib.unified_diff(before.decode("utf-8").splitlines(True),
                                         after.decode("utf-8").splitlines(True), source, target))
    return "".join(patch)


def verify(name, original, modified, patch_path):
    scratch = ROOT / "target" / "vendor-patch-check"
    scratch.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="fluent-vendor-", dir=scratch) as directory:
        destination = Path(directory)
        for relative, content in original.items():
            path = destination / name / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        subprocess.run(["git", "apply", "--check", str(patch_path)], cwd=destination, check=True)
        subprocess.run(["git", "apply", str(patch_path)], cwd=destination, check=True)
        rebuilt = {p.relative_to(destination / name).as_posix(): normalized(p.read_bytes())
                   for p in (destination / name).rglob("*") if p.is_file()}
        if rebuilt != modified:
            raise RuntimeError(f"Reconstructed {name} differs from vendor tree")
        print(f"{name}: patch reconstructs all {len(rebuilt)} retained files")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--cache", type=Path, default=Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "registry" / "cache")
    args = parser.parse_args()
    for name, (version, checksum) in PACKAGES.items():
        original = upstream(name, version, checksum, args.cache)
        modified = local(name)
        patch = make_patch(name, original, modified)
        path = ROOT / "vendor" / "patches" / f"{name}.patch"
        if args.check:
            if not path.exists() or normalized(path.read_bytes()) != patch.encode("utf-8"):
                raise RuntimeError(f"{path.relative_to(ROOT)} is stale; run python scripts/vendor-patches.py")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(patch.encode("utf-8"))
        verify(name, original, modified, path)


if __name__ == "__main__":
    main()
