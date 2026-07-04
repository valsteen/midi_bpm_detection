# Foundation Parameter Crates

The foundation crates provide reusable parameter metadata and host-parameter
bridges for Rust plugin products. They let product crates describe their
configuration once, use that metadata in UI or runtime code, and optionally map
the same fields into NIH-plug host parameters.

These crates are ordinary Cargo packages. The filesystem grouping lives under
`rust/crates/foundation/`; package names and Rust imports keep their normal
hyphen/underscore forms.

## Crates

- `parameter` defines generic parameter metadata, value conversion helpers, and
  the `#[parameter_group]` macro for Rust config structs.
- `parameter-on-off` provides the reusable `OnOff<T>` value type when a setting
  needs an enabled/disabled state plus a value.
- `parameter-nih-plug` maps generic parameter metadata to NIH-plug host
  parameters and provides `#[nih_plugin_parameter_group]`,
  `NihPlugFieldAdapter`, and `MirrorHostParam`.
- `parameter-on-off-nih-plug` connects `OnOff<f32>` to NIH-plug through
  `OnOffParam` and `OnOffF32Adapter`.

Use only the crates your product needs. Plain config metadata does not require
NIH-plug crates, and a product that does not use `OnOff<T>` does not need the
OnOff packages.

## Cargo Usage

A product crate in this workspace can depend on the foundation crates directly:

```toml
[dependencies]
parameter = { path = "../../foundation/parameter" }

# Optional when config fields use OnOff<T>.
parameter-on-off = { path = "../../foundation/parameter-on-off" }

# Optional when the product exposes host parameters through NIH-plug.
parameter-nih-plug = { path = "../../foundation/parameter-nih-plug" }

# Optional when NIH-plug host parameters need OnOff<f32> support.
parameter-on-off-nih-plug = { path = "../../foundation/parameter-on-off-nih-plug" }
```

Adjust the relative path if the consuming crate lives in another group.

## Plain Config Metadata

Use `parameter::parameter_group` on the product config type. The generated
metadata can be consumed by runtime code or UI without any plugin-host
dependency.

```rust
use parameter::parameter_group;
use parameter_on_off::OnOff;

#[parameter_group]
pub struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = OnOff::On(0.5))]
    pub gain: OnOff<f32>,
}
```

## NIH-plug Host Parameters

Add NIH-plug integration in the plugin product crate when the host needs to see
the parameters. The host parameter holder points back to the plain config type
and imports any adapters required by custom value types.

```rust
use parameter_nih_plug::nih_plugin_parameter_group;
use parameter_on_off_nih_plug::{OnOffF32Adapter, OnOffParam};

#[nih_plugin_parameter_group(config = ExampleConfig, group = "Example")]
pub struct ExamplePluginParams {
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub gain: OnOffParam,
}
```

For the broader workspace boundaries around these crates, see
`../../../docs/architecture.md`.
