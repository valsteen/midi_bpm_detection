# VST3 Binary License And Corresponding Source

The MIDI BPM Detection repository source remains available under the root MIT `LICENSE`. The separately built VST3
binary links the pinned `vst3-sys` revision `b3ff4d775940f5b476b9d1cca02a90e07e1922a2`, which declares GPLv3-or-later.
The combined VST3 binary distributed in a release is therefore conveyed under GPL-3.0-or-later. The full GPL text is
in `GPL-3.0-or-later.txt` beside this file.

The corresponding source for every VST3 binary is the shared source asset attached to the same GitHub Release as the
enclosing VST3 archive:

```text
midi-bpm-detector-<release-tag>-vst3-source.tar.gz
```

Here, `<release-tag>` is the stable `vX.Y.Z` tag named by the enclosing VST3 archive. The source archive contains the
exact repository files from that tag, `rust/Cargo.lock`, the release build scripts, all Cargo dependency sources selected
by the lockfile, and `rust/.cargo/config.toml` configured to use the included `rust/vendor/` directory.

To rebuild a platform VST3 bundle from the extracted corresponding-source archive:

```shell
cd rust
cargo run --offline --locked --package xtask --release -- bundle midi-bpm-detector-plugin \
  --release --locked --lib --no-default-features --features vst3
```

Add `--target <target-triple>` to the bundle arguments when reproducing a specific cross-target artifact. The bundler
forwards that target to its Cargo build. Platform system libraries and a Rust toolchain are build prerequisites and are
not part of the source archive.

This file records the project's conservative engineering distribution policy; it is not legal advice.
