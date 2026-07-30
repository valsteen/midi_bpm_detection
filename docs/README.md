# Documentation

The project documentation covers architecture, runtime flows, the detection model, development workflows, and
implementation conventions.

| Topic | Main document | Coverage |
| --- | --- | --- |
| Project structure and operating modes | [Architecture](architecture.md) | Cross-build-root architecture and runtime modes. [Rust Workspace Architecture](../rust/architecture.md) contains the crate map and Rust dependency direction. |
| Runtime ownership and cross-mode data flow | [Runtime Lifecycle](runtime-lifecycle.md) | Bootstrap wiring and the main plugin, desktop, WASM, GUI, and BPM detection flows. |
| Plugin realtime processing | [Plugin Flow](plugin-flow.md) | Host buffer processing, realtime handoff, background work, parameter flow, and tempo feedback. |
| Native desktop and MIDI runtime | [Native MIDI Flow](native-midi-flow.md) | Desktop startup, MIDI service ownership, worker commands, and output ownership. |
| Plugin-to-Bitwig tempo control | [Bitwig Tempo Bridge](bitwig-tempo-bridge.md) | Rendezvous, socket frames, extension ownership, and failure behavior. |
| Detection model and interval terminology | [Algorithm Archaeology](algorithm-archaeology.md) | Histogram and weighting rationale, configuration vocabulary, and model development. |
| Setup, commands, packaging, and verification | [Development Commands](development.md) | Build-root and runtime-mode development workflows. |
| Implementation conventions | [Engineering Style](engineering-style.md) | Shared Rust and Kotlin engineering principles. |
| Behavior-preserving structural work | [Refactoring Guide](refactoring-guide.md) | Concrete smells, corrections, and stop conditions. |
| Existing lint exceptions | [Lint Exceptions](lint-exceptions.md) | Current Rust and Kotlin exception inventory and policy. |
