# AGENTS.md

Repository instructions for AI coding agents working on this project.

## Working Style

- Make small, reviewable changes.
- Do not add Kotlin suppressions or Detekt ignore rules without explicit human confirmation.
- Treat compiler warnings, Detekt findings, and Spotless failures as issues to fix.
- Do not revert unrelated changes. Assume unrecognized local changes came from the user.

## Kotlin And Extension Tooling

- Build through `./gradlew`, not global Gradle.
- Keep Bitwig extension API compatibility at version `2` until the user explicitly changes the baseline.
- Keep JVM bytecode target `17`.
- Prefer `private` or `internal`; make declarations `public` only when Bitwig's loader or another module needs them.
- Put reusable Bitwig ceremony in `libs/bitwig-bootstrap`.
- Put loadable extension outputs in `extensions/*`.

## Documentation

- Use `../docs/development.md` for build, lint, package, and install commands.
- Use `../docs/bitwig-tempo-bridge.md` for the plugin-to-extension rendezvous and socket bridge.
- Use `../docs/lint-exceptions.md` when reviewing or changing lint exceptions.
