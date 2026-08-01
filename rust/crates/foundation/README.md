# Foundation Parameter Crates

The foundation crates provide reusable parameter metadata and host-parameter
bridges for Rust plugin products. They let product crates describe their
configuration once, use that metadata in UI or runtime code, and optionally map
the same fields into nice-plug host parameters.

These crates are ordinary Cargo packages. The filesystem grouping lives under
`rust/crates/foundation/`; package names and Rust imports keep their normal
hyphen/underscore forms.

## Crates

- `parameter` defines generic parameter metadata, value conversion helpers, and
  the `#[parameter_group]` macro for Rust config structs.
- `parameter-on-off` provides the reusable `OnOff<T>` value type when a setting
  needs an enabled/disabled state plus a value.
- `parameter-nice-plug` maps generic parameter metadata to nice-plug host
  parameters and provides `#[nice_plugin_parameter_group]`,
  `NicePlugFieldAdapter`, and `MirrorHostParams`.
- `parameter-on-off-nice-plug` connects `OnOff<f32>` to nice-plug through
  `OnOffParams` and `OnOffF32Adapter`.

Use only the crates your product needs. Plain config metadata does not require
nice-plug crates, and a product that does not use `OnOff<T>` does not need the
OnOff packages.

## Cargo Usage

A product crate in this workspace can depend on the foundation crates directly:

```toml
[dependencies]
parameter = { path = "../../foundation/parameter" }

# Optional when config fields use OnOff<T>.
parameter-on-off = { path = "../../foundation/parameter-on-off" }

# Optional when the product exposes host parameters through nice-plug.
parameter-nice-plug = { path = "../../foundation/parameter-nice-plug" }

# Optional when nice-plug host parameters need OnOff<f32> support.
parameter-on-off-nice-plug = { path = "../../foundation/parameter-on-off-nice-plug" }
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

## nice-plug Host Parameters

Add nice-plug integration in the plugin product crate when the host needs to see
the parameters. The host parameter holder points back to the plain config type
and imports any adapters required by custom value types.

```rust
use parameter_nice_plug::nice_plugin_parameter_group;
use parameter_on_off_nice_plug::{OnOffF32Adapter, OnOffParams};

#[nice_plugin_parameter_group(config = ExampleConfig, group = "Example")]
pub struct ExamplePluginParams {
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub gain: OnOffParams,
}
```

A `NicePlugFieldAdapter` owns the host-parameter set for one config field. That
set may contain one concrete host parameter or several. `OnOffF32Adapter` maps
one `OnOff<f32>` field to its enabled/value pair, so the config and host holder
do not need another field declaration for the enabled state.

## Automatic Changed-Config Reconciliation

The same config and host parameter declarations automatically provide typed
merge and mirror operations:

```rust
use parameter::MergeChangedFields;
use parameter_nice_plug::MirrorChangedConfig;

let merged = ExampleConfig::merge_changed_fields(&draft, &previous, &current);
let mirrored = params.mirror_changed_config(&before, &after, setter);
```

`PartialEq` comparisons preserve complete variant state, adapters stay behind
`MirrorHostParams`, and nested parameter groups recurse into their own generated
operations. No additional macro or duplicate field declaration is required.

For the broader workspace boundaries around these crates, see
`../../../docs/architecture.md`.
