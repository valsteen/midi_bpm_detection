import importlib.util
import subprocess
import tarfile
import tempfile
import tomllib
import unittest
import zipfile
from pathlib import Path
from unittest import mock


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
RELEASE_TOOL_PATH = REPOSITORY_ROOT / "scripts" / "release.py"
SPEC = importlib.util.spec_from_file_location("release_tool", RELEASE_TOOL_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Could not load {RELEASE_TOOL_PATH}")
release_tool = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_tool)


class ReleaseToolTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        (self.root / "rust/crates/member-a").mkdir(parents=True)
        (self.root / "rust/crates/member-b").mkdir(parents=True)
        (self.root / "extension/extensions/beat-detection-controller/src/main/kotlin/beatdetection").mkdir(
            parents=True
        )
        (self.root / "rust/Cargo.toml").write_text(
            '[workspace]\nmembers = ["crates/member-a", "crates/member-b"]\n\n'
            '[workspace.package]\nversion = "0.1.0"\n',
            encoding="utf-8",
        )
        for member in ("member-a", "member-b"):
            (self.root / f"rust/crates/{member}/src").mkdir()
            (self.root / f"rust/crates/{member}/Cargo.toml").write_text(
                f'[package]\nname = "{member}"\nversion.workspace = true\n', encoding="utf-8"
            )
            (self.root / f"rust/crates/{member}/src/lib.rs").write_text(
                "pub fn fixture() {}\n", encoding="utf-8"
            )
        (self.root / "extension/build.gradle.kts").write_text(
            'allprojects {\n    version = "0.1.0"\n}\n', encoding="utf-8"
        )
        (
            self.root
            / "extension/extensions/beat-detection-controller/src/main/kotlin/beatdetection/BeatDetectionExtensionDefinition.kt"
        ).write_text('version = "0.1.0",\n', encoding="utf-8")
        (self.root / "LICENSE").write_text("project license\n", encoding="utf-8")
        (self.root / "LICENSES").mkdir()
        (self.root / "LICENSES/GPL-3.0-or-later.txt").write_text("GPL text\n", encoding="utf-8")
        (self.root / "LICENSES/VST3-BUILD.md").write_text(
            "Corresponding source: midi-bpm-detector-<release-tag>-vst3-source.tar.gz "
            "on the same GitHub Release.\n",
            encoding="utf-8",
        )
        (self.root / "LICENSES/EXTENSION-THIRD-PARTY.md").write_text(
            "extension notices\n", encoding="utf-8"
        )
        (self.root / "LICENSES/Apache-2.0.txt").write_text("Apache text\n", encoding="utf-8")
        self.notices = self.root / "notices"
        self.notices.mkdir()
        (self.notices / "THIRD_PARTY_NOTICES.md").write_text(
            "artifact notices with full license text\n", encoding="utf-8"
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    @staticmethod
    def cargo_metadata(**versions: str) -> dict[str, object]:
        packages = [
            {"id": f"path+file:///fixture/{name}#{version}", "name": name, "version": version}
            for name, version in versions.items()
        ]
        return {
            "packages": packages,
            "workspace_members": [package["id"] for package in packages],
        }

    def test_preflight_accepts_matching_stable_version(self) -> None:
        with mock.patch.object(
            release_tool,
            "_cargo_metadata",
            return_value=self.cargo_metadata(**{"member-a": "0.1.0", "member-b": "0.1.0"}),
        ):
            self.assertEqual("0.1.0", release_tool.validate_release_version(self.root, "v0.1.0"))

    def test_preflight_rejects_malformed_tag(self) -> None:
        with self.assertRaisesRegex(release_tool.ReleaseError, "vX.Y.Z"):
            release_tool.validate_release_version(self.root, "0.1.0")

    def test_preflight_rejects_version_drift(self) -> None:
        (self.root / "extension/build.gradle.kts").write_text(
            'allprojects {\n    version = "0.1.1"\n}\n', encoding="utf-8"
        )

        with (
            mock.patch.object(
                release_tool,
                "_cargo_metadata",
                return_value=self.cargo_metadata(**{"member-a": "0.1.0", "member-b": "0.1.0"}),
            ),
            self.assertRaisesRegex(release_tool.ReleaseError, "extension Gradle version is 0.1.1"),
        ):
            release_tool.validate_release_version(self.root, "v0.1.0")

    def test_preflight_rejects_any_workspace_member_version_drift(self) -> None:
        with (
            mock.patch.object(
                release_tool,
                "_cargo_metadata",
                return_value=self.cargo_metadata(**{"member-a": "0.1.0", "member-b": "0.1.1"}),
            ),
            self.assertRaisesRegex(release_tool.ReleaseError, "Cargo workspace member member-b is 0.1.1"),
        ):
            release_tool.validate_release_version(self.root, "v0.1.0")

    def test_set_version_updates_every_product_authority_and_refreshes_cargo_lock(self) -> None:
        with mock.patch.object(release_tool, "_refresh_cargo_lock") as refresh_cargo_lock:
            release_tool.set_version(self.root, "0.2.0")

        self.assertIn(
            'version = "0.2.0"',
            (self.root / "rust/Cargo.toml").read_text(encoding="utf-8"),
        )
        self.assertIn(
            'version = "0.2.0"',
            (self.root / "extension/build.gradle.kts").read_text(encoding="utf-8"),
        )
        self.assertIn(
            'version = "0.2.0",',
            (self.root / release_tool.EXTENSION_DEFINITION).read_text(encoding="utf-8"),
        )
        refresh_cargo_lock.assert_called_once_with(self.root)

    def test_set_version_rejects_malformed_input_without_writing(self) -> None:
        cargo_manifest = self.root / "rust/Cargo.toml"
        original_manifest = cargo_manifest.read_text(encoding="utf-8")

        with (
            mock.patch.object(release_tool, "_refresh_cargo_lock") as refresh_cargo_lock,
            self.assertRaisesRegex(release_tool.ReleaseError, "stable X.Y.Z"),
        ):
            release_tool.set_version(self.root, "v0.2.0")

        self.assertEqual(original_manifest, cargo_manifest.read_text(encoding="utf-8"))
        refresh_cargo_lock.assert_not_called()

    def test_set_version_validates_every_authority_before_writing(self) -> None:
        cargo_manifest = self.root / "rust/Cargo.toml"
        gradle_build = self.root / "extension/build.gradle.kts"
        extension_definition = self.root / release_tool.EXTENSION_DEFINITION
        extension_definition.write_text("const val unrelated = true\n", encoding="utf-8")
        originals = {
            path: path.read_bytes() for path in (cargo_manifest, gradle_build, extension_definition)
        }

        with (
            mock.patch.object(release_tool, "_refresh_cargo_lock") as refresh_cargo_lock,
            self.assertRaisesRegex(release_tool.ReleaseError, "Bitwig host-visible version"),
        ):
            release_tool.set_version(self.root, "0.2.0")

        self.assertEqual(originals, {path: path.read_bytes() for path in originals})
        refresh_cargo_lock.assert_not_called()

    def test_set_version_restores_authorities_and_lock_when_cargo_refresh_fails(self) -> None:
        cargo_lock = self.root / "rust/Cargo.lock"
        cargo_lock.write_bytes(b"original lock contents\n")
        paths = (
            self.root / "rust/Cargo.toml",
            self.root / "extension/build.gradle.kts",
            self.root / release_tool.EXTENSION_DEFINITION,
            cargo_lock,
        )
        originals = {path: path.read_bytes() for path in paths}

        with (
            mock.patch.object(
                release_tool,
                "_refresh_cargo_lock",
                side_effect=subprocess.CalledProcessError(101, ["cargo", "metadata"]),
            ),
            self.assertRaises(subprocess.CalledProcessError),
        ):
            release_tool.set_version(self.root, "0.2.0")

        self.assertEqual(originals, {path: path.read_bytes() for path in paths})

    def test_set_version_refreshes_lock_for_locked_workspace_build(self) -> None:
        manifest = self.root / "rust/Cargo.toml"
        subprocess.run(
            ["cargo", "generate-lockfile", "--manifest-path", str(manifest), "--offline"],
            check=True,
            capture_output=True,
            text=True,
        )

        release_tool.set_version(self.root, "0.2.0")

        lock_data = tomllib.loads((self.root / "rust/Cargo.lock").read_text(encoding="utf-8"))
        workspace_versions = {
            package["name"]: package["version"]
            for package in lock_data["package"]
            if package["name"] in {"member-a", "member-b"}
        }
        self.assertEqual({"member-a": "0.2.0", "member-b": "0.2.0"}, workspace_versions)
        subprocess.run(
            [
                "cargo",
                "check",
                "--manifest-path",
                str(manifest),
                "--locked",
                "--workspace",
                "--offline",
            ],
            check=True,
            capture_output=True,
            text=True,
        )

    def test_expected_assets_are_exact_and_versioned(self) -> None:
        self.assertEqual(
            {
                "midi-bpm-detector-v0.1.0-clap-macos-arm64.zip",
                "midi-bpm-detector-v0.1.0-clap-macos-x86_64.zip",
                "midi-bpm-detector-v0.1.0-clap-windows-x86_64.zip",
                "midi-bpm-detector-v0.1.0-clap-linux-x86_64.zip",
                "midi-bpm-detector-v0.1.0-vst3-macos-arm64.zip",
                "midi-bpm-detector-v0.1.0-vst3-macos-x86_64.zip",
                "midi-bpm-detector-v0.1.0-vst3-windows-x86_64.zip",
                "midi-bpm-detector-v0.1.0-vst3-linux-x86_64.zip",
                "midi-bpm-detector-v0.1.0-desktop-macos-arm64.zip",
                "midi-bpm-detector-v0.1.0-desktop-macos-x86_64.zip",
                "midi-bpm-detector-v0.1.0-desktop-linux-x86_64.zip",
                "midi-bpm-detector-v0.1.0-bitwig-extension.zip",
                "midi-bpm-detector-v0.1.0-vst3-source.tar.gz",
                "SHA256SUMS",
            },
            release_tool.expected_asset_names("v0.1.0"),
        )

    def test_clap_archive_has_stable_layout_and_notices(self) -> None:
        bundle = self.root / "bundle/midi-bpm-detector-plugin.clap"
        binary = bundle / "Contents/MacOS/midi-bpm-detector-plugin"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"plugin")
        output_directory = self.root / "dist"

        first = release_tool.package_clap(
            self.root, "v0.1.0", "macos-arm64", bundle, self.notices, output_directory
        )
        first_bytes = first.read_bytes()
        second = release_tool.package_clap(
            self.root, "v0.1.0", "macos-arm64", bundle, self.notices, output_directory
        )

        self.assertEqual(first_bytes, second.read_bytes())
        with zipfile.ZipFile(first) as archive:
            self.assertEqual(
                {
                    "midi-bpm-detector-v0.1.0-clap-macos-arm64/LICENSE",
                    "midi-bpm-detector-v0.1.0-clap-macos-arm64/THIRD_PARTY_NOTICES.md",
                    "midi-bpm-detector-v0.1.0-clap-macos-arm64/midi-bpm-detector-plugin.clap/Contents/MacOS/midi-bpm-detector-plugin",
                },
                {name for name in archive.namelist() if not name.endswith("/")},
            )

    def test_asset_verification_rejects_extra_file(self) -> None:
        asset_directory = self.root / "assets"
        asset_directory.mkdir()
        for name in release_tool.expected_asset_names("v0.1.0"):
            (asset_directory / name).write_bytes(b"asset")
        (asset_directory / "unexpected.zip").write_bytes(b"asset")

        with self.assertRaisesRegex(release_tool.ReleaseError, "unexpected.zip"):
            release_tool.verify_asset_directory(asset_directory, "v0.1.0")

    def test_vst3_archive_has_gpl_notice_and_source_pointer(self) -> None:
        bundle = self.root / "bundle/midi-bpm-detector-plugin.vst3"
        binary = bundle / "Contents/MacOS/midi-bpm-detector-plugin"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"plugin")

        output = release_tool.package_vst3(
            self.root, "v0.1.0", "macos-arm64", bundle, self.notices, self.root / "dist"
        )

        with zipfile.ZipFile(output) as archive:
            files = {name for name in archive.namelist() if not name.endswith("/")}
            prefix = "midi-bpm-detector-v0.1.0-vst3-macos-arm64"
            self.assertIn(f"{prefix}/GPL-3.0-or-later.txt", files)
            self.assertIn(f"{prefix}/VST3-BUILD.md", files)
            self.assertIn(f"{prefix}/THIRD_PARTY_NOTICES.md", files)
            source_pointer = archive.read(f"{prefix}/VST3-BUILD.md").decode("utf-8")
            self.assertIn(
                "midi-bpm-detector-<release-tag>-vst3-source.tar.gz", source_pointer
            )
            self.assertIn("same GitHub Release", source_pointer)

    def test_vst3_source_notice_is_release_tag_independent(self) -> None:
        source_pointer = (REPOSITORY_ROOT / "LICENSES/VST3-BUILD.md").read_text(
            encoding="utf-8"
        )

        self.assertIn("midi-bpm-detector-<release-tag>-vst3-source.tar.gz", source_pointer)
        self.assertIn("same GitHub Release", source_pointer)
        self.assertNotIn("v0.1.0", source_pointer)

    def test_bundle_binary_finds_non_macos_vst3_layout(self) -> None:
        bundle = self.root / "bundle/plugin.vst3"
        binary = bundle / "Contents/x86_64-linux/plugin.so"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"plugin")

        self.assertEqual(binary, release_tool._bundle_binary(bundle))

    def test_desktop_archive_contains_binary_and_artifact_notices(self) -> None:
        binary = self.root / "target/desktop"
        binary.parent.mkdir()
        binary.write_bytes(b"desktop")

        output = release_tool.package_desktop(
            self.root, "v0.1.0", "macos-arm64", binary, self.notices, self.root / "dist"
        )

        with zipfile.ZipFile(output) as archive:
            files = {name for name in archive.namelist() if not name.endswith("/")}
            prefix = "midi-bpm-detector-v0.1.0-desktop-macos-arm64"
            self.assertIn(f"{prefix}/desktop", files)
            self.assertIn(f"{prefix}/THIRD_PARTY_NOTICES.md", files)

    def test_vst3_source_archive_contains_tracked_source_vendor_and_build_config(self) -> None:
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        (self.root / "source.txt").write_text("tracked source\n", encoding="utf-8")
        tracked_config = self.root / "rust/.cargo/config.toml"
        tracked_config.parent.mkdir(parents=True)
        tracked_config.write_text("[alias]\nxtask = 'run --package xtask --release --'\n", encoding="utf-8")
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        vendor = self.root / "vendor"
        vendor.mkdir()
        (vendor / "dependency.rs").write_text("vendored source\n", encoding="utf-8")
        vendor_config = self.root / "vendor-config.toml"
        vendor_config.write_text("[source.crates-io]\nreplace-with = 'vendored-sources'\n", encoding="utf-8")

        output = release_tool.package_vst3_source(
            self.root, "v0.1.0", vendor, vendor_config, self.root / "dist"
        )

        with tarfile.open(output, "r:gz") as archive:
            names = archive.getnames()
            prefix = "midi-bpm-detector-v0.1.0-vst3-source"
            self.assertIn(f"{prefix}/source.txt", names)
            self.assertIn(f"{prefix}/rust/vendor/dependency.rs", names)
            self.assertIn(f"{prefix}/rust/.cargo/config.toml", names)
            self.assertEqual(1, names.count(f"{prefix}/rust/.cargo/config.toml"))
            config = archive.extractfile(f"{prefix}/rust/.cargo/config.toml")
            self.assertIsNotNone(config)
            self.assertIn(b"vendored-sources", config.read())

    def test_checksums_cover_every_asset_except_the_checksum_file(self) -> None:
        asset_directory = self.root / "assets"
        asset_directory.mkdir()
        for name in release_tool.expected_asset_names("v0.1.0") - {"SHA256SUMS"}:
            (asset_directory / name).write_bytes(name.encode())

        checksum_file = release_tool.write_checksums(asset_directory, "v0.1.0")
        release_tool.verify_asset_directory(asset_directory, "v0.1.0")
        release_tool.verify_checksums(asset_directory, "v0.1.0")

        self.assertEqual("SHA256SUMS", checksum_file.name)
        self.assertNotIn("SHA256SUMS  SHA256SUMS", checksum_file.read_text(encoding="utf-8"))

    def test_cargo_about_report_renders_dependency_and_full_license_text(self) -> None:
        report = {
            "licenses": [
                {
                    "id": "MIT",
                    "name": "MIT License",
                    "text": "Permission is hereby granted.\n",
                    "used_by": [
                        {
                            "crate": {
                                "name": "dependency",
                                "version": "1.2.3",
                                "source": "registry+example",
                                "repository": "https://example.invalid/dependency",
                            }
                        },
                        {
                            "crate": {
                                "name": "local-package",
                                "version": "0.1.0",
                                "source": None,
                                "repository": None,
                            }
                        },
                    ],
                }
            ]
        }
        output = self.root / "generated-notices"

        release_tool.write_cargo_about_notice(report, output)

        notice = (output / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
        self.assertIn("dependency 1.2.3", notice)
        self.assertIn("Permission is hereby granted.", notice)
        self.assertNotIn("local-package", notice)

    def test_license_policy_accepts_windows_boost_license(self) -> None:
        self.assertIn('"BSL-1.0"', release_tool._cargo_about_config("x86_64-pc-windows-msvc", False))

    def test_license_policy_accepts_gpl_only_for_vst3(self) -> None:
        clap_config = release_tool._cargo_about_config("aarch64-apple-darwin", False)
        vst3_config = release_tool._cargo_about_config("aarch64-apple-darwin", True)

        self.assertNotIn('"GPL-3.0-or-later"', clap_config)
        self.assertIn('"GPL-3.0-or-later"', vst3_config)

    def test_symbol_verification_is_format_specific(self) -> None:
        release_tool.verify_clap_symbols("0000 T _clap_entry\n")
        release_tool.verify_vst3_symbols("0000 T _GetPluginFactory\n")

        with self.assertRaisesRegex(release_tool.ReleaseError, "GetPluginFactory"):
            release_tool.verify_clap_symbols("0000 T clap_entry\n0001 T GetPluginFactory\n")
        with self.assertRaisesRegex(release_tool.ReleaseError, "clap_entry"):
            release_tool.verify_vst3_symbols("0000 T clap_entry\n0001 T GetPluginFactory\n")

    def test_windows_symbol_inspection_reads_the_pe_export_table(self) -> None:
        bundle = self.root / "plugin.clap"
        binary = self.root / "plugin.dll"
        readobj = Path("llvm-readobj.exe")
        completed = subprocess.CompletedProcess(
            args=[],
            returncode=0,
            stdout="Export {\n  Name: clap_entry\n}\n",
            stderr="",
        )

        with (
            mock.patch.object(release_tool.sys, "platform", "win32"),
            mock.patch.object(release_tool, "_bundle_binary", return_value=binary),
            mock.patch.object(
                release_tool,
                "_rustup_llvm_tool",
                return_value=readobj,
                create=True,
            ) as rustup_tool,
            mock.patch.object(release_tool.subprocess, "run", return_value=completed) as run,
        ):
            symbols = release_tool.plugin_symbols(bundle)

        self.assertEqual(completed.stdout, symbols)
        rustup_tool.assert_called_once_with("llvm-readobj")
        run.assert_called_once_with(
            [str(readobj), "--coff-exports", str(binary)],
            capture_output=True,
            text=True,
        )

    def test_release_workflow_keeps_write_permission_in_draft_only_tag_job(self) -> None:
        workflow = (REPOSITORY_ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")

        self.assertIn("workflow_dispatch:", workflow)
        self.assertNotIn("default: v0.1.0", workflow)
        self.assertIn('tags: ["v*.*.*"]', workflow)
        self.assertIn("permissions:\n  contents: read", workflow)
        self.assertIn("needs: [preflight, clap, vst3, desktop, extension, source]", workflow)
        self.assertIn("needs: assemble", workflow)
        self.assertIn("contents: write", workflow)
        self.assertIn("github.event_name == 'push'", workflow)
        self.assertIn("gh release create", workflow)
        self.assertIn("--draft", workflow)
        self.assertIn('--notes-file ".github/release-notes/$RELEASE_TAG.md"', workflow)
        self.assertNotIn("--generate-notes", workflow)
        self.assertTrue((REPOSITORY_ROOT / ".github/release-notes/v0.1.0.md").is_file())
        self.assertNotIn("gh release edit", workflow)
        self.assertNotIn("--draft=false", workflow)

    def test_windows_plugin_packaging_uses_shell_neutral_release_tag(self) -> None:
        workflow = (REPOSITORY_ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")

        self.assertIn(
            'python scripts/release.py package-clap\n          "${{ env.RELEASE_TAG }}"',
            workflow,
        )
        self.assertIn(
            'python scripts/release.py package-vst3\n          "${{ env.RELEASE_TAG }}"',
            workflow,
        )
        self.assertIn("package-vst3", workflow)
        self.assertIn("package-desktop", workflow)
        self.assertIn("package-vst3-source", workflow)
        self.assertIn("write-checksums", workflow)
        self.assertIn("cargo-about --version 0.9.1", workflow)
        self.assertEqual(
            4,
            workflow.count(
                "bundle: rust/target/bundled/midi-bpm-detector-plugin.clap"
            ),
        )
        self.assertEqual(
            4,
            workflow.count(
                "bundle: rust/target/bundled/midi-bpm-detector-plugin.vst3"
            ),
        )


if __name__ == "__main__":
    unittest.main()
