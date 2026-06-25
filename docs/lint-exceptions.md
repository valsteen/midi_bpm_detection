# Lint Exceptions

This file records approved lint exceptions for both build roots: Rust `allow` attributes and Kotlin/Detekt suppressions.

Policy:

- Fix Clippy warnings by default.
- Fix Kotlin compiler warnings, Detekt findings, and Spotless failures by default.
- Do not add a new Rust `allow`, Kotlin suppression, or Detekt ignore without human confirmation.
- If an exception is confirmed, keep it narrow and explain why it exists near the code.
- Prefer removing broad legacy exceptions as touched code becomes clearer.
- When a small helper exists only to wrap synchronization or another focused concern, prefer moving it to the focused
  crate that owns that concern instead of leaving it in a domain crate or creating a generic `utils` crate.

## Audit Notes

Rust reviewed on 2026-06-24 against the tracked crates while keeping the 2026-06-21 cleanup notes below.

Kotlin currently has no approved suppressions or Detekt ignores.

Removed during this audit:

- `forbidden_lint_groups`
  - These suppressions were unexplained and did not affect `clippy-all`; they were removed.
- `non_canonical_partial_ord_impl` on `MidiInputPort`
  - The type no longer pretends to have global ordering. Device lists now sort explicitly by `MidiInputPort::sort_key()`.
- `unused_variables`, `dead_code`, `unused`, and `no_effect_underscore_binding`
  - Removed where the code could express intent directly through underscore names or by deleting stale helpers.

### Broad Legacy Exceptions

These are the main cleanup risk. They are not immediate behavior bugs, but they hide categories of warnings across whole
crates:

- `missing_panics_doc`, `missing_errors_doc`, `module_name_repetitions`
  - Present across core, GUI, WASM, sync, parameter, errors, build, native MIDI, and plugin crates.
  - Mostly documentation/API-style noise from `clippy::pedantic`.
  - Acceptable temporarily, but should not be copied to new crates without confirmation.
- Cast lint groups such as `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, and
  `cast_precision_loss`
  - Present mainly in core numeric code, GUI rendering, parameter conversion, plugin code, and WASM input adapters.
  - Higher-risk than doc/style exceptions because they can hide real overflow or precision bugs.
  - Keep reviewing these opportunistically when touching numeric conversion code.
- `similar_names`
  - Present in the plugin crate.
  - Treat this as readability debt in a dense integration layer, not as permission to introduce confusing local names.

### Local Exceptions That Look Intentional

- `fake_midi_output.rs`: `unnecessary_wraps`
  - The fake output constructor mirrors the real output constructor, which can fail. Keeping the same shape simplifies
    target-specific construction.
- `midi_output_trait.rs`: `dead_code`
  - Some output capabilities are not used by every build mode, but the trait represents the native MIDI output surface.
- `serializable_atomic.rs`: `must_use_candidate`
  - Atomic wrappers mirror standard atomic APIs. Some callers intentionally use the side effect and ignore the returned
    previous value.
- `gui.rs`: `match_same_arms`
  - The equal arms carry different lifecycle comments: "editor is open but GUI not created yet" versus "editor is
    closed." This is a readability exception, not a behavior workaround.

### Refactor Markers

These exceptions are signs of code that may deserve splitting or clearer names:

- `too_many_lines`
  - Present in BPM scoring, native worker loop, plugin task executor, and plugin parameter construction.
  - These are complexity markers. Refactor when working in those areas, but avoid mechanical extraction that hides the
    flow.
- `too_many_arguments`
  - Present in generic parameter construction and GUI methods.
  - May be better represented by a small builder/config object if the call sites become harder to read.
- `struct_field_names`
  - Present around runtime/service structs with repeated domain terms.
  - Often harmless, but worth revisiting when renaming concepts.
- `needless_pass_by_value`
  - Present around boundaries where moved values line up with thread/worker ownership.
  - Re-check when changing ownership or cloning behavior.

### Current Priority

The broad cast lint suppressions are now the highest-risk remaining category. They are probably legitimate in many GUI,
parameter, and timestamp conversion paths, but each one should be narrowed or replaced with checked conversion when that
code is touched.

Example from this audit: optional atomic wrappers belong to `sync`, not `bpm_detection_midi` or the plugin crate, because
they are synchronization primitives with no MIDI dependency. They should still preserve domain invariants at their public
surface: sample/timestamp wrappers must preserve `Some(0)`, while disabled TCP ports are represented as
`Option<NonZeroU16>` instead of accepting `Some(0)` and interpreting it as missing.
