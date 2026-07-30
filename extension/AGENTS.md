# AGENTS.md

Kotlin/Bitwig extension instructions for AI coding agents working under `extension/`.

## Read These First

- The repository-level `../AGENTS.md` still applies.
- Use `../docs/README.md` to select task-specific public documentation.
- Use `../docs/development.md` for Gradle, lint, package, and install commands.
- Use `../docs/bitwig-tempo-bridge.md` for the plugin-to-extension rendezvous and socket bridge.
- Use `../docs/lint-exceptions.md` when reviewing an existing lint exception or an explicitly approved new one.

## Kotlin And Extension Tooling

- Build through `./gradlew`, not global Gradle.
- Keep Bitwig extension API compatibility at version `2` until the user explicitly changes the baseline.
- Keep JVM bytecode target `17`.
- Prefer `private` or `internal`; make declarations `public` only when Bitwig's loader or another module needs them.
- Put reusable Bitwig ceremony in `libs/bitwig-bootstrap`.
- Put loadable extension outputs in `extensions/*`.
- Keep external API mechanics and callback normalization in focused Bitwig adapters. Product targeting, tempo-frame
  delivery, and status presentation belong to their owning extension/runtime boundary.

## Bitwig Settings And Undo

- Treat Bitwig document/controller settings as user-editable settings first, not as read-only status labels. In API 25,
  `Settings.getStringSetting(...)` creates an editable text field, and enum settings remain user-selectable.
- Before adding dynamic status or diagnostics through `DocumentState`, `Preferences`, or any other Bitwig `Settings`
  surface, check whether extension-owned writes appear in Bitwig's Undo history. If undo suppression is not available,
  prefer stable/coarse status, duplicate-write suppression, or a different feedback surface.
- Do not use Settings > Controllers / Controller Preferences for user-facing runtime status unless Bitwig exposes a
  clearly non-editable informational widget. Keep that area for installation/configuration.
