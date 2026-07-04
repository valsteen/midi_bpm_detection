# Foundation Parameter Stack

This directory groups the reusable parameter crates that another plugin product
can import from this Rust workspace. Cargo package names stay unchanged; only
the workspace paths live under `crates/foundation/`.

## Dependency Policy

Product, domain, and application crates may depend down into foundation crates.
Foundation crates must not depend back up into BPM-specific product crates such
as `midi-bpm-detector-plugin`, `bpm_detection_core`, `bpm_detection_midi`, or
`gui`.

The foundation layer is for reusable parameter metadata, reusable value types,
and optional plugin-host bridges. It is not a new application runtime, and it
does not own desktop, WASM, Bitwig controller, or BPM product behavior.

## Crates

- `parameter` defines generic parameter metadata, value conversion, and the
  `#[parameter_group]` macro for Rust config structs.
- `parameter-nih-plug` maps generic parameter metadata to NIH-plug host
  parameters and provides `#[nih_plugin_parameter_group]`,
  `NihPlugFieldAdapter`, and `MirrorHostParam`.
- `parameter-on-off` defines the reusable `OnOff<T>` value type and its
  serialization/value conversion behavior.
- `parameter-on-off-nih-plug` connects `OnOff<f32>` to NIH-plug through
  `OnOffParam` and `OnOffF32Adapter`.

Optional custom value crates can live beside the base stack when their behavior
is reusable by another plugin product. Optional bridge crates can live here when
they connect such a value type to NIH-plug. A new custom value type should not
require editing `parameter`, `parameter-nih-plug`, or their macro crates; add a
focused value crate and, if the production plugin needs host integration, a
focused NIH-plug bridge crate.

## Example Import Shape

If another plugin product crate lived at `rust/crates/example-plugin`, it would
depend on the foundation crates it needs directly:

```toml
[dependencies]
parameter = { path = "../foundation/parameter" }
parameter-nih-plug = { path = "../foundation/parameter-nih-plug" }

# Optional only when the product config uses OnOff<T>.
parameter-on-off = { path = "../foundation/parameter-on-off" }

# Optional only when the NIH-plug host params need OnOff<f32> support.
parameter-on-off-nih-plug = { path = "../foundation/parameter-on-off-nih-plug" }
```

The product config would keep its own domain fields and annotate them with
`parameter::parameter_group`. The plugin host layer would annotate its NIH-plug
parameter holder with `parameter_nih_plug::nih_plugin_parameter_group`. For an
`OnOff<f32>` field, the host holder imports
`parameter_on_off_nih_plug::{OnOffF32Adapter, OnOffParam}` and marks that field
with `#[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]`.

Plain config metadata does not need NIH-plug:

```rust
use parameter::parameter_group;
use parameter_on_off::OnOff;

#[parameter_group]
pub struct ExamplePluginConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = OnOff::On(0.5))]
    pub gain: OnOff<f32>,
}
```

NIH-plug integration is added by the plugin product crate when it needs host
parameters:

```rust
use parameter_nih_plug::nih_plugin_parameter_group;
use parameter_on_off_nih_plug::{OnOffF32Adapter, OnOffParam};

#[nih_plugin_parameter_group(config = ExamplePluginConfig, group = "Example")]
pub struct ExamplePluginParams {
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub gain: OnOffParam,
}
```
