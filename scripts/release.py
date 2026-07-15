#!/usr/bin/env python3
"""Release preflight, artifact packaging, and release-contract verification."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile
from pathlib import Path
from typing import Iterable


PLUGIN_PACKAGE = Path("rust/crates/entrypoints/midi-bpm-detector-plugin/Cargo.toml")
DESKTOP_PACKAGE = Path("rust/crates/entrypoints/desktop/Cargo.toml")
WORKSPACE_MANIFEST = Path("rust/Cargo.toml")
GRADLE_BUILD = Path("extension/build.gradle.kts")
EXTENSION_DEFINITION = Path(
    "extension/extensions/beat-detection-controller/src/main/kotlin/beatdetection/BeatDetectionExtensionDefinition.kt"
)
PLUGIN_PACKAGE_NAME = "midi-bpm-detector-plugin"
DESKTOP_PACKAGE_NAME = "desktop"
CARGO_ABOUT_VERSION = "0.9.1"
CARGO_ABOUT_ACCEPTED = (
    "Apache-2.0",
    "MIT",
    "ISC",
    "BSD-1-Clause",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "Zlib",
    "0BSD",
    "BlueOak-1.0.0",
    "CC0-1.0",
    "MPL-2.0",
    "OFL-1.1",
    "Ubuntu-font-1.0",
    "Unicode-3.0",
    "Unlicense",
    "GPL-3.0-or-later",
)
VST3_LICENSE_CHECKSUM = "1be76dd654024ee690864bea328622e912847461671cee0533ddf9a2cab4a31d"
PLUGIN_PLATFORMS = (
    "macos-arm64",
    "macos-x86_64",
    "windows-x86_64",
    "linux-x86_64",
)
DESKTOP_PLATFORMS = ("macos-arm64", "macos-x86_64", "linux-x86_64")
TAG_PATTERN = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
VERSION_PATTERN = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
ARCHIVE_TIMESTAMP = (1980, 1, 1, 0, 0, 0)


class ReleaseError(RuntimeError):
    """Raised when the release contract is not satisfied."""


def release_version(tag: str) -> str:
    """Return the version from a stable vX.Y.Z tag."""
    match = TAG_PATTERN.fullmatch(tag)
    if match is None:
        raise ReleaseError(f"release tag must use stable vX.Y.Z form, got {tag!r}")
    return ".".join(match.groups())


def stable_version(version: str) -> str:
    """Validate and return a stable X.Y.Z version without a leading v."""
    if VERSION_PATTERN.fullmatch(version) is None:
        raise ReleaseError(f"release version must use stable X.Y.Z form without v, got {version!r}")
    return version


def _single_quoted_version(path: Path, pattern: re.Pattern[str], label: str) -> str:
    matches = pattern.findall(path.read_text(encoding="utf-8"))
    if len(matches) != 1:
        raise ReleaseError(f"expected exactly one {label} declaration in {path}, found {len(matches)}")
    return matches[0]


def _replaced_single_quoted_version(
    contents: str, path: Path, pattern: re.Pattern[str], replacement: str, label: str
) -> str:
    updated, count = pattern.subn(replacement, contents)
    if count != 1:
        raise ReleaseError(f"expected exactly one {label} declaration in {path}, found {count}")
    return updated


def _cargo_metadata(repository_root: Path, *, locked: bool) -> dict[str, object]:
    command = ["cargo", "metadata", "--format-version", "1", "--no-deps"]
    if locked:
        command.append("--locked")
    result = subprocess.run(
        command,
        cwd=repository_root / "rust",
        capture_output=True,
        check=True,
        text=True,
    )
    return json.loads(result.stdout)


def _workspace_member_versions(repository_root: Path) -> dict[str, str]:
    metadata = _cargo_metadata(repository_root, locked=True)
    packages = {
        str(package["id"]): package for package in metadata.get("packages", [])  # type: ignore[index]
    }
    versions: dict[str, str] = {}
    for member_id in metadata.get("workspace_members", []):  # type: ignore[union-attr]
        package = packages.get(str(member_id))
        if package is None:
            raise ReleaseError(f"Cargo metadata omitted workspace member {member_id}")
        versions[f"Cargo workspace member {package['name']}"] = str(package["version"])
    if not versions:
        raise ReleaseError("Cargo metadata returned no workspace members")
    return dict(sorted(versions.items()))


def _refresh_cargo_lock(repository_root: Path) -> None:
    subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--offline"],
        cwd=repository_root / "rust",
        check=True,
        stdout=subprocess.DEVNULL,
    )


def set_version(repository_root: Path, version: str) -> str:
    """Set every product version authority and refresh Cargo.lock through Cargo."""
    version = stable_version(version)
    declarations = (
        (
            repository_root / WORKSPACE_MANIFEST,
            re.compile(r'^version\s*=\s*"[^"]+"\s*$', re.MULTILINE),
            f'version = "{version}"',
            "Rust workspace package version",
        ),
        (
            repository_root / GRADLE_BUILD,
            re.compile(r'^\s*version\s*=\s*"[^"]+"\s*$', re.MULTILINE),
            f'    version = "{version}"',
            "extension Gradle version",
        ),
        (
            repository_root / EXTENSION_DEFINITION,
            re.compile(r'^\s*version\s*=\s*"[^"]+",\s*$', re.MULTILINE),
            f'            version = "{version}",',
            "Bitwig host-visible version",
        ),
    )
    original_declarations = {
        path: path.read_text(encoding="utf-8") for path, _, _, _ in declarations
    }
    updated_declarations = {
        path: _replaced_single_quoted_version(
            original_declarations[path], path, pattern, replacement, label
        )
        for path, pattern, replacement, label in declarations
    }
    cargo_lock = repository_root / "rust/Cargo.lock"
    original_cargo_lock = cargo_lock.read_bytes() if cargo_lock.exists() else None

    try:
        for path, updated in updated_declarations.items():
            path.write_text(updated, encoding="utf-8")
        _refresh_cargo_lock(repository_root)
    except Exception:
        for path, original in original_declarations.items():
            path.write_text(original, encoding="utf-8")
        if original_cargo_lock is None:
            cargo_lock.unlink(missing_ok=True)
        else:
            cargo_lock.write_bytes(original_cargo_lock)
        raise
    return version


def product_versions(repository_root: Path) -> dict[str, str]:
    """Read every Cargo workspace member and both extension version declarations."""
    gradle_version = _single_quoted_version(
        repository_root / GRADLE_BUILD,
        re.compile(r'^\s*version\s*=\s*"([^"]+)"\s*$', re.MULTILINE),
        "extension Gradle version",
    )
    host_version = _single_quoted_version(
        repository_root / EXTENSION_DEFINITION,
        re.compile(r'^\s*version\s*=\s*"([^"]+)",\s*$', re.MULTILINE),
        "Bitwig host-visible version",
    )
    return {
        **_workspace_member_versions(repository_root),
        "extension Gradle version": gradle_version,
        "Bitwig host-visible version": host_version,
    }


def validate_release_version(repository_root: Path, tag: str) -> str:
    """Validate a release tag against all coordinated product versions."""
    version = release_version(tag)
    mismatches = [
        f"{label} is {actual}, expected {version} from {tag}"
        for label, actual in product_versions(repository_root).items()
        if actual != version
    ]
    if mismatches:
        raise ReleaseError("release version drift:\n- " + "\n- ".join(mismatches))
    return version


def expected_asset_names(tag: str) -> set[str]:
    """Return the exact public asset contract for a release tag."""
    release_version(tag)
    plugin_assets = {
        f"midi-bpm-detector-{tag}-{plugin_format}-{platform}.zip"
        for plugin_format in ("clap", "vst3")
        for platform in PLUGIN_PLATFORMS
    }
    desktop_assets = {
        f"midi-bpm-detector-{tag}-desktop-{platform}.zip" for platform in DESKTOP_PLATFORMS
    }
    return plugin_assets | desktop_assets | {
        f"midi-bpm-detector-{tag}-bitwig-extension.zip",
        f"midi-bpm-detector-{tag}-vst3-source.tar.gz",
        "SHA256SUMS",
    }


def _archive_info(name: str, mode: int) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(name, ARCHIVE_TIMESTAMP)
    info.compress_type = zipfile.ZIP_DEFLATED
    info.create_system = 3
    info.external_attr = (mode & 0xFFFF) << 16
    return info


def _archive_sources(source: Path, archive_name: str) -> Iterable[tuple[Path, str]]:
    if source.is_file():
        yield source, f"{archive_name}/{source.name}"
        return
    if not source.is_dir():
        raise ReleaseError(f"release input does not exist: {source}")
    for child in sorted(path for path in source.rglob("*") if path.is_file()):
        if child.is_symlink():
            raise ReleaseError(f"release input contains unsupported symbolic link: {child}")
        yield child, f"{archive_name}/{source.name}/{child.relative_to(source).as_posix()}"


def _write_archive(output: Path, sources: Iterable[tuple[Path, str]]) -> Path:
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for source, archive_path in sorted(sources, key=lambda item: item[1]):
            source_mode = source.stat().st_mode
            mode = stat.S_IFREG | (0o755 if source_mode & 0o111 else 0o644)
            archive.writestr(_archive_info(archive_path, mode), source.read_bytes(), compresslevel=9)
    return output


def _artifact_notice_sources(
    repository_root: Path, notice_directory: Path, archive_name: str
) -> list[tuple[Path, str]]:
    project_license = repository_root / "LICENSE"
    notice = notice_directory / "THIRD_PARTY_NOTICES.md"
    for required in (project_license, notice):
        if not required.is_file():
            raise ReleaseError(f"required release notice is missing: {required}")
    return [
        (project_license, f"{archive_name}/LICENSE"),
        (notice, f"{archive_name}/THIRD_PARTY_NOTICES.md"),
    ]


def package_clap(
    repository_root: Path,
    tag: str,
    platform: str,
    bundle: Path,
    notice_directory: Path,
    output_directory: Path,
) -> Path:
    """Create one versioned CLAP release ZIP with license notices."""
    release_version(tag)
    if platform not in PLUGIN_PLATFORMS:
        raise ReleaseError(f"unknown CLAP release platform: {platform}")
    output = output_directory / f"midi-bpm-detector-{tag}-clap-{platform}.zip"
    archive_name = output.stem
    sources = list(_archive_sources(bundle, archive_name))
    sources.extend(_artifact_notice_sources(repository_root, notice_directory, archive_name))
    return _write_archive(output, sources)


def package_vst3(
    repository_root: Path,
    tag: str,
    platform: str,
    bundle: Path,
    notice_directory: Path,
    output_directory: Path,
) -> Path:
    """Create one versioned VST3 release ZIP with GPL and source notices."""
    release_version(tag)
    if platform not in PLUGIN_PLATFORMS:
        raise ReleaseError(f"unknown VST3 release platform: {platform}")
    output = output_directory / f"midi-bpm-detector-{tag}-vst3-{platform}.zip"
    archive_name = output.stem
    sources = list(_archive_sources(bundle, archive_name))
    sources.extend(_artifact_notice_sources(repository_root, notice_directory, archive_name))
    for notice_name in ("GPL-3.0-or-later.txt", "VST3-BUILD.md"):
        notice = repository_root / "LICENSES" / notice_name
        if not notice.is_file():
            raise ReleaseError(f"required VST3 notice is missing: {notice}")
        sources.append((notice, f"{archive_name}/{notice_name}"))
    return _write_archive(output, sources)


def package_desktop(
    repository_root: Path,
    tag: str,
    platform: str,
    binary: Path,
    notice_directory: Path,
    output_directory: Path,
) -> Path:
    """Create one versioned desktop application ZIP."""
    release_version(tag)
    if platform not in DESKTOP_PLATFORMS:
        raise ReleaseError(f"unknown desktop release platform: {platform}")
    if not binary.is_file():
        raise ReleaseError(f"desktop release binary does not exist: {binary}")
    output = output_directory / f"midi-bpm-detector-{tag}-desktop-{platform}.zip"
    archive_name = output.stem
    sources = [(binary, f"{archive_name}/{binary.name}")]
    sources.extend(_artifact_notice_sources(repository_root, notice_directory, archive_name))
    return _write_archive(output, sources)


def package_extension(
    repository_root: Path,
    tag: str,
    extension_archive: Path,
    output_directory: Path,
) -> Path:
    """Create the versioned optional Bitwig extension release ZIP."""
    release_version(tag)
    if extension_archive.name != "BeatDetectionExtension.bwextension" or not extension_archive.is_file():
        raise ReleaseError(f"expected packaged BeatDetectionExtension.bwextension, got {extension_archive}")
    output = output_directory / f"midi-bpm-detector-{tag}-bitwig-extension.zip"
    archive_name = output.stem
    sources = [(extension_archive, f"{archive_name}/{extension_archive.name}")]
    project_license = repository_root / "LICENSE"
    extension_notice = repository_root / "LICENSES/EXTENSION-THIRD-PARTY.md"
    apache_license = repository_root / "LICENSES/Apache-2.0.txt"
    for required in (project_license, extension_notice, apache_license):
        if not required.is_file():
            raise ReleaseError(f"required extension notice is missing: {required}")
    sources.extend(
        [
            (project_license, f"{archive_name}/LICENSE"),
            (extension_notice, f"{archive_name}/THIRD_PARTY_NOTICES.md"),
            (apache_license, f"{archive_name}/third-party-licenses/Apache-2.0.txt"),
        ]
    )
    return _write_archive(output, sources)


def _write_tar_gz(output: Path, sources: Iterable[tuple[Path, str]]) -> Path:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as raw_output:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw_output, compresslevel=9, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                for source, archive_path in sorted(sources, key=lambda item: item[1]):
                    if not source.is_file() or source.is_symlink():
                        raise ReleaseError(f"unsupported source archive input: {source}")
                    mode = 0o755 if source.stat().st_mode & 0o111 else 0o644
                    info = tarfile.TarInfo(archive_path)
                    info.size = source.stat().st_size
                    info.mode = mode
                    info.mtime = 0
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    with source.open("rb") as source_file:
                        archive.addfile(info, source_file)
    return output


def package_vst3_source(
    repository_root: Path,
    tag: str,
    vendor_directory: Path,
    vendor_config: Path,
    output_directory: Path,
) -> Path:
    """Package exact tracked source plus vendored Cargo dependencies for VST3 correspondence."""
    release_version(tag)
    if not vendor_directory.is_dir() or not vendor_config.is_file():
        raise ReleaseError("VST3 source packaging requires a vendor directory and Cargo source config")
    result = subprocess.run(
        ["git", "ls-files", "-z"], cwd=repository_root, capture_output=True, check=True
    )
    tracked_paths = [Path(path.decode()) for path in result.stdout.split(b"\0") if path]
    output = output_directory / f"midi-bpm-detector-{tag}-vst3-source.tar.gz"
    archive_name = output.name.removesuffix(".tar.gz")
    sources_by_archive_path = {
        f"{archive_name}/{path.as_posix()}": repository_root / path for path in tracked_paths
    }
    for source, archive_path in _archive_sources(vendor_directory, archive_name):
        relative = Path(archive_path).relative_to(Path(archive_name) / vendor_directory.name)
        sources_by_archive_path[f"{archive_name}/rust/vendor/{relative.as_posix()}"] = source
    sources_by_archive_path[f"{archive_name}/rust/.cargo/config.toml"] = vendor_config
    return _write_tar_gz(output, ((source, path) for path, source in sources_by_archive_path.items()))


def _asset_names_without_checksums(tag: str) -> set[str]:
    return expected_asset_names(tag) - {"SHA256SUMS"}


def write_checksums(asset_directory: Path, tag: str) -> Path:
    """Write deterministic SHA-256 checksums for the complete non-checksum asset set."""
    expected = _asset_names_without_checksums(tag)
    actual = {path.name for path in asset_directory.iterdir() if path.is_file()}
    if actual != expected:
        raise ReleaseError("cannot write checksums before the complete binary/source asset set exists")
    lines = []
    for name in sorted(expected):
        digest = hashlib.sha256((asset_directory / name).read_bytes()).hexdigest()
        lines.append(f"{digest}  {name}\n")
    output = asset_directory / "SHA256SUMS"
    output.write_text("".join(lines), encoding="utf-8")
    return output


def verify_checksums(asset_directory: Path, tag: str) -> None:
    """Verify SHA256SUMS exactly covers and matches every non-checksum asset."""
    checksum_file = asset_directory / "SHA256SUMS"
    if not checksum_file.is_file():
        raise ReleaseError("SHA256SUMS is missing")
    expected_names = _asset_names_without_checksums(tag)
    checksums: dict[str, str] = {}
    for line in checksum_file.read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        if match is None or match.group(2) in checksums:
            raise ReleaseError(f"invalid SHA256SUMS entry: {line!r}")
        checksums[match.group(2)] = match.group(1)
    if set(checksums) != expected_names:
        raise ReleaseError("SHA256SUMS does not cover the exact release asset set")
    for name, expected_digest in checksums.items():
        actual_digest = hashlib.sha256((asset_directory / name).read_bytes()).hexdigest()
        if actual_digest != expected_digest:
            raise ReleaseError(f"checksum mismatch for {name}")


def verify_asset_directory(asset_directory: Path, tag: str) -> None:
    """Require the exact expanded non-empty asset set for a release tag."""
    if not asset_directory.is_dir():
        raise ReleaseError(f"asset directory does not exist: {asset_directory}")
    actual = {path.name for path in asset_directory.iterdir() if path.is_file()}
    expected = expected_asset_names(tag)
    missing = sorted(expected - actual)
    extra = sorted(actual - expected)
    empty = sorted(name for name in actual if (asset_directory / name).stat().st_size == 0)
    if missing or extra or empty:
        details = []
        if missing:
            details.append("missing: " + ", ".join(missing))
        if extra:
            details.append("unexpected: " + ", ".join(extra))
        if empty:
            details.append("empty: " + ", ".join(empty))
        raise ReleaseError("release asset set is invalid (" + "; ".join(details) + ")")
    verify_checksums(asset_directory, tag)


def verify_clap_symbols(symbols: str) -> None:
    """Require the CLAP export and reject the VST3 factory export."""
    if re.search(r"(^|\s)_?clap_entry($|\s)", symbols, re.MULTILINE) is None:
        raise ReleaseError("CLAP entry point clap_entry is missing")
    if re.search(r"(^|\s)_?GetPluginFactory($|\s)", symbols, re.MULTILINE) is not None:
        raise ReleaseError("VST3 factory entry point GetPluginFactory must be absent")


def verify_vst3_symbols(symbols: str) -> None:
    """Require the VST3 factory export and reject the CLAP entry export."""
    if re.search(r"(^|\s)_?GetPluginFactory($|\s)", symbols, re.MULTILINE) is None:
        raise ReleaseError("VST3 factory entry point GetPluginFactory is missing")
    if re.search(r"(^|\s)_?clap_entry($|\s)", symbols, re.MULTILINE) is not None:
        raise ReleaseError("CLAP entry point clap_entry must be absent")


def _bundle_binary(bundle: Path) -> Path:
    if bundle.is_file():
        return bundle
    contents = bundle / "Contents"
    if contents.is_dir():
        binaries = sorted(
            path
            for path in contents.rglob("*")
            if path.is_file()
            and ("MacOS" in path.parts or path.suffix.lower() in {".dll", ".so", ".vst3"})
        )
        if len(binaries) == 1:
            return binaries[0]
    raise ReleaseError(f"could not identify the plugin binary in {bundle}")


def _llvm_nm_from_rustup() -> Path | None:
    result = subprocess.run(
        ["rustc", "--print", "sysroot"], capture_output=True, check=True, text=True
    )
    sysroot = Path(result.stdout.strip())
    candidates = sorted((sysroot / "lib/rustlib").glob("*/bin/llvm-nm*"))
    return candidates[0] if candidates else None


def plugin_symbols(bundle: Path) -> str:
    """Inspect exported symbols using the platform tool or Rust's LLVM tools."""
    binary = _bundle_binary(bundle)
    if sys.platform == "darwin":
        command = ["nm", "-gU", str(binary)]
    elif sys.platform.startswith("linux"):
        command = ["nm", "-D", "--defined-only", str(binary)]
    else:
        llvm_nm = _llvm_nm_from_rustup()
        if llvm_nm is None:
            raise ReleaseError("llvm-nm is unavailable; install the llvm-tools-preview Rust component")
        command = [str(llvm_nm), "--defined-only", "--extern-only", str(binary)]
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        raise ReleaseError(f"symbol inspection failed: {result.stderr.strip()}")
    return result.stdout


def verify_clap_dependencies(repository_root: Path, target: str | None = None) -> None:
    """Prove the selected CLAP graph excludes vst3-sys."""
    command = [
        "cargo",
        "tree",
        "--locked",
        "--package",
        PLUGIN_PACKAGE_NAME,
        "--no-default-features",
        "--features",
        "clap",
        "--edges",
        "normal",
        "--prefix",
        "none",
    ]
    if target:
        command.extend(["--target", target])
    result = subprocess.run(command, cwd=repository_root / "rust", capture_output=True, text=True)
    if result.returncode != 0:
        raise ReleaseError(f"cargo tree failed: {result.stderr.strip()}")
    if re.search(r"^vst3-sys\s", result.stdout, re.MULTILINE):
        raise ReleaseError("CLAP-only dependency graph unexpectedly contains vst3-sys")


def verify_vst3_dependencies(repository_root: Path, target: str | None = None) -> None:
    """Prove the selected VST3 graph includes vst3-sys."""
    command = [
        "cargo",
        "tree",
        "--locked",
        "--package",
        PLUGIN_PACKAGE_NAME,
        "--no-default-features",
        "--features",
        "vst3",
        "--edges",
        "normal",
        "--prefix",
        "none",
    ]
    if target:
        command.extend(["--target", target])
    result = subprocess.run(command, cwd=repository_root / "rust", capture_output=True, text=True)
    if result.returncode != 0:
        raise ReleaseError(f"cargo tree failed: {result.stderr.strip()}")
    if re.search(r"^vst3-sys\s", result.stdout, re.MULTILINE) is None:
        raise ReleaseError("VST3 dependency graph does not contain vst3-sys")


def write_cargo_about_notice(report: dict[str, object], output_directory: Path) -> Path:
    """Render cargo-about JSON as a self-contained third-party notice with full texts."""
    if output_directory.exists():
        shutil.rmtree(output_directory)
    output_directory.mkdir(parents=True)
    lines = [
        "# Third-Party Notices\n\n",
        "This file was generated from the locked, target-specific runtime dependency graph. Each section includes "
        "the complete license text selected by the pinned license generator.\n\n",
    ]
    rendered = 0
    for license_entry in report.get("licenses", []):  # type: ignore[union-attr]
        third_party = [
            used_by["crate"]
            for used_by in license_entry["used_by"]
            if used_by["crate"].get("source")
        ]
        if not third_party:
            continue
        text = str(license_entry.get("text") or "").strip()
        if not text:
            raise ReleaseError(f"cargo-about returned an empty text for {license_entry.get('id')}")
        rendered += 1
        lines.append(f"## {license_entry['name']} (`{license_entry['id']}`)\n\n")
        lines.append("Applies to:\n\n")
        for crate in sorted(third_party, key=lambda item: (item["name"], item["version"])):
            source = crate.get("repository") or crate["source"]
            lines.append(f"- `{crate['name']} {crate['version']}` — {source}\n")
        lines.append("\n")
        lines.extend(f"    {line}\n" for line in text.splitlines())
        lines.append("\n")
    if rendered == 0:
        raise ReleaseError("cargo-about report contained no third-party license texts")
    notice = output_directory / "THIRD_PARTY_NOTICES.md"
    notice.write_text("".join(lines), encoding="utf-8")
    return notice


def _cargo_about_config(target: str, include_vst3: bool) -> str:
    accepted_ids = (
        CARGO_ABOUT_ACCEPTED
        if include_vst3
        else tuple(
            license_id
            for license_id in CARGO_ABOUT_ACCEPTED
            if license_id != "GPL-3.0-or-later"
        )
    )
    accepted = ",\n".join(f"  {json.dumps(license_id)}" for license_id in accepted_ids)
    config = (
        f"accepted = [\n{accepted},\n]\n"
        f"targets = [{json.dumps(target)}]\n"
        "ignore-build-dependencies = true\n"
        "ignore-dev-dependencies = true\n"
        'workarounds = ["ring"]\n'
    )
    if include_vst3:
        config += (
            "\n[vst3-sys.clarify]\n"
            'license = "GPL-3.0-or-later"\n'
            "\n[[vst3-sys.clarify.git]]\n"
            'path = "license.md"\n'
            f'checksum = "{VST3_LICENSE_CHECKSUM}"\n'
        )
    return config


def generate_rust_notices(
    repository_root: Path,
    package_name: str,
    target: str,
    features: tuple[str, ...],
    output_directory: Path,
) -> Path:
    """Generate fail-closed, target-specific notices with pinned cargo-about."""
    manifests = {
        PLUGIN_PACKAGE_NAME: PLUGIN_PACKAGE,
        DESKTOP_PACKAGE_NAME: DESKTOP_PACKAGE,
    }
    if package_name not in manifests:
        raise ReleaseError(f"unsupported Rust release package: {package_name}")
    version_result = subprocess.run(
        ["cargo", "about", "--version"], capture_output=True, check=True, text=True
    )
    if version_result.stdout.strip() != f"cargo-about {CARGO_ABOUT_VERSION}":
        raise ReleaseError(
            f"cargo-about {CARGO_ABOUT_VERSION} is required, got {version_result.stdout.strip()}"
        )
    with tempfile.TemporaryDirectory() as temporary_directory:
        temporary_root = Path(temporary_directory)
        config = temporary_root / "about.toml"
        report = temporary_root / "about.json"
        config.write_text(_cargo_about_config(target, "vst3" in features), encoding="utf-8")
        command = [
            "cargo",
            "about",
            "generate",
            "--manifest-path",
            str(repository_root / manifests[package_name]),
            "--no-default-features",
            "--locked",
            "--offline",
            "--fail",
            "--format",
            "json",
            "--config",
            str(config),
            "--output-file",
            str(report),
        ]
        if features:
            command.extend(["--features", " ".join(features)])
        subprocess.run(command, cwd=repository_root / "rust", check=True)
        return write_cargo_about_notice(json.loads(report.read_text(encoding="utf-8")), output_directory)


def _repository_root() -> Path:
    return Path(__file__).resolve().parent.parent


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    preflight = subparsers.add_parser("preflight", help="validate coordinated release versions")
    preflight.add_argument("tag")

    version_setter = subparsers.add_parser(
        "set-version", help="set coordinated product versions and refresh Cargo.lock"
    )
    version_setter.add_argument("version")

    dependencies = subparsers.add_parser(
        "verify-clap-dependencies", help="prove the CLAP graph excludes vst3-sys"
    )
    dependencies.add_argument("--target")

    vst3_dependencies = subparsers.add_parser(
        "verify-vst3-dependencies", help="prove the VST3 graph includes vst3-sys"
    )
    vst3_dependencies.add_argument("--target")

    rust_notices = subparsers.add_parser(
        "generate-rust-notices", help="generate notices for one Rust artifact graph"
    )
    rust_notices.add_argument("package")
    rust_notices.add_argument("target")
    rust_notices.add_argument("output_directory", type=Path)
    rust_notices.add_argument("--features", default="")

    symbols = subparsers.add_parser("verify-clap-bundle", help="verify CLAP binary exports")
    symbols.add_argument("bundle", type=Path)

    vst3_symbols = subparsers.add_parser("verify-vst3-bundle", help="verify VST3 binary exports")
    vst3_symbols.add_argument("bundle", type=Path)

    clap_package = subparsers.add_parser("package-clap", help="package a CLAP release asset")
    clap_package.add_argument("tag")
    clap_package.add_argument("platform", choices=PLUGIN_PLATFORMS)
    clap_package.add_argument("bundle", type=Path)
    clap_package.add_argument("notice_directory", type=Path)
    clap_package.add_argument("output_directory", type=Path)

    vst3_package = subparsers.add_parser("package-vst3", help="package a VST3 release asset")
    vst3_package.add_argument("tag")
    vst3_package.add_argument("platform", choices=PLUGIN_PLATFORMS)
    vst3_package.add_argument("bundle", type=Path)
    vst3_package.add_argument("notice_directory", type=Path)
    vst3_package.add_argument("output_directory", type=Path)

    desktop_package = subparsers.add_parser(
        "package-desktop", help="package a desktop release asset"
    )
    desktop_package.add_argument("tag")
    desktop_package.add_argument("platform", choices=DESKTOP_PLATFORMS)
    desktop_package.add_argument("binary", type=Path)
    desktop_package.add_argument("notice_directory", type=Path)
    desktop_package.add_argument("output_directory", type=Path)

    extension_package = subparsers.add_parser(
        "package-extension", help="package the Bitwig extension release asset"
    )
    extension_package.add_argument("tag")
    extension_package.add_argument("extension_archive", type=Path)
    extension_package.add_argument("output_directory", type=Path)

    source_package = subparsers.add_parser(
        "package-vst3-source", help="package tracked and vendored VST3 corresponding source"
    )
    source_package.add_argument("tag")
    source_package.add_argument("vendor_directory", type=Path)
    source_package.add_argument("vendor_config", type=Path)
    source_package.add_argument("output_directory", type=Path)

    checksums = subparsers.add_parser(
        "write-checksums", help="write SHA256SUMS for the complete candidate asset set"
    )
    checksums.add_argument("tag")
    checksums.add_argument("asset_directory", type=Path)

    assets = subparsers.add_parser("verify-assets", help="verify the exact release asset set")
    assets.add_argument("tag")
    assets.add_argument("asset_directory", type=Path)

    args = parser.parse_args()
    repository_root = _repository_root()
    try:
        if args.command == "preflight":
            version = validate_release_version(repository_root, args.tag)
            print(f"release version {version} matches every Cargo workspace member and extension metadata")
        elif args.command == "set-version":
            version = set_version(repository_root, args.version)
            print(f"set coordinated product version to {version} and refreshed Cargo.lock through Cargo")
        elif args.command == "verify-clap-dependencies":
            verify_clap_dependencies(repository_root, args.target)
            print("CLAP-only dependency graph excludes vst3-sys")
        elif args.command == "verify-vst3-dependencies":
            verify_vst3_dependencies(repository_root, args.target)
            print("VST3 dependency graph includes vst3-sys")
        elif args.command == "generate-rust-notices":
            features = tuple(feature for feature in args.features.split(",") if feature)
            print(
                generate_rust_notices(
                    repository_root,
                    args.package,
                    args.target,
                    features,
                    args.output_directory,
                )
            )
        elif args.command == "verify-clap-bundle":
            verify_clap_symbols(plugin_symbols(args.bundle))
            print(f"{args.bundle} exports clap_entry and does not export GetPluginFactory")
        elif args.command == "verify-vst3-bundle":
            verify_vst3_symbols(plugin_symbols(args.bundle))
            print(f"{args.bundle} exports GetPluginFactory and does not export clap_entry")
        elif args.command == "package-clap":
            print(
                package_clap(
                    repository_root,
                    args.tag,
                    args.platform,
                    args.bundle,
                    args.notice_directory,
                    args.output_directory,
                )
            )
        elif args.command == "package-vst3":
            print(
                package_vst3(
                    repository_root,
                    args.tag,
                    args.platform,
                    args.bundle,
                    args.notice_directory,
                    args.output_directory,
                )
            )
        elif args.command == "package-desktop":
            print(
                package_desktop(
                    repository_root,
                    args.tag,
                    args.platform,
                    args.binary,
                    args.notice_directory,
                    args.output_directory,
                )
            )
        elif args.command == "package-extension":
            print(
                package_extension(
                    repository_root, args.tag, args.extension_archive, args.output_directory
                )
            )
        elif args.command == "package-vst3-source":
            print(
                package_vst3_source(
                    repository_root,
                    args.tag,
                    args.vendor_directory,
                    args.vendor_config,
                    args.output_directory,
                )
            )
        elif args.command == "write-checksums":
            print(write_checksums(args.asset_directory, args.tag))
        elif args.command == "verify-assets":
            verify_asset_directory(args.asset_directory, args.tag)
            print(f"{args.asset_directory} contains the exact expected release asset set")
    except (OSError, ReleaseError, subprocess.SubprocessError, tomllib.TOMLDecodeError) as error:
        print(f"release error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
