# Native Desktop Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the TUI-first desktop shell with a native desktop crate that owns MIDI device selection and starts the shared egui UI directly.

**Architecture:** Add a new `crates/desktop` integration crate above `gui` and `bpm_detection_midi`. Keep `gui` MIDI-free and keep `bpm_detection_midi` UI-free. Move behavior in small increments: first a tested non-visual device selection model, then a desktop MIDI controller, then egui extension hooks, then a direct native desktop binary.

**Tech Stack:** Rust 2024, Cargo workspace crates, egui/eframe through `gui`, `bpm_detection_midi`, Tokio only where the current native controller needs async/blocking isolation.

## Global Constraints

- Do not move MIDI/native dependencies into `gui`.
- Do not move egui/UI dependencies into `bpm_detection_midi`.
- Do not port the broad `tui::Action` / `tui::Event` bus into the new desktop crate.
- Keep plugin and WASM behavior unchanged while desktop mode is migrated.
- Preserve `SelectDevice`'s hotplug-stable selection behavior before deleting the TUI component.
- Preserve the `MidiService::execute(closure)` service-thread boundary unless a concrete problem justifies changing it.
- Add tests for non-visual behavior before wiring UI.

---

## File Structure

- Create `crates/desktop/Cargo.toml`: native desktop crate manifest.
- Create `crates/desktop/src/lib.rs`: module exports for desktop config, device selection, and controller.
- Create `crates/desktop/src/bin/desktop.rs`: eventual direct native GUI entry point.
- Create `crates/desktop/src/device_selection.rs`: pure model for current devices and selected input.
- Create `crates/desktop/src/config.rs`: desktop config shape containing only native desktop settings.
- Create `crates/desktop/src/live_parameters.rs`: desktop `BPMDetectionConfig` adapter for the shared GUI.
- Create `crates/desktop/src/controller.rs`: native MIDI controller boundary over `bpm_detection_midi::MidiService`.
- Modify `Cargo.toml`: add `crates/desktop` to workspace members and eventually default members.
- Modify `crates/gui/src/application_parameters.rs`: add a default no-op egui extension hook.
- Modify `crates/gui/src/config_ui.rs`: call the extension hook after shared controls.
- Modify `crates/tui/src/components/select_device.rs`: temporarily reuse `desktop::DeviceSelection`.
- Modify `crates/tui/Cargo.toml`: add temporary dependency on `desktop`.
- Modify `docs/architecture.md` and `docs/native-midi-flow.md`: update diagrams and transition notes as the crate appears.

---

### Task 1: Scaffold `crates/desktop`

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/desktop/Cargo.toml`
- Create: `crates/desktop/src/lib.rs`
- Create: `crates/desktop/src/bin/desktop.rs`

**Interfaces:**
- Produces: workspace package `desktop`
- Produces: binary `desktop`
- Consumes: existing workspace crates only

- [ ] **Step 1: Add the crate to the workspace**

Edit root `Cargo.toml`:

```toml
members = [
    "crates/tui",
    "crates/desktop",
    "crates/errors",
    "crates/build",
    "crates/bpm_detection_core",
    "crates/bpm_detection_midi",
    "crates/gui",
    "crates/sync",
    "crates/parameter",
    "crates/wasm",
    "crates/midi-bpm-detector-plugin",
    "crates/midi-bpm-detector-plugin/xtask",
    "crates/midi-reset",
]
```

- [ ] **Step 2: Create the desktop manifest**

Create `crates/desktop/Cargo.toml`:

```toml
[package]
name = "desktop"
version = "0.1.0"
edition = "2024"
description = "Native desktop runtime for MIDI BPM detection"
default-run = "desktop"

[dependencies]
bpm_detection_core = { path = "../bpm_detection_core" }
bpm_detection_midi = { path = "../bpm_detection_midi" }
build = { path = "../build" }
errors = { path = "../errors" }
gui = { path = "../gui" }
parameter = { path = "../parameter" }
sync = { path = "../sync" }

config = "0.15.13"
log = "0.4.27"
mimalloc = "0.1.47"
serde = { version = "1.0.219", features = ["derive"] }
tokio = { version = "1.47.1", features = ["rt-multi-thread", "sync"] }
toml = "0.9.5"

[dev-dependencies]
pretty_assertions = "1.4.1"

[build-dependencies]
build = { path = "../build" }

[lints]
workspace = true
```

- [ ] **Step 3: Create crate exports**

Create `crates/desktop/src/lib.rs`:

```rust
pub mod config;
pub mod controller;
pub mod device_selection;
pub mod live_parameters;
```

- [ ] **Step 4: Create a minimal binary**

Create `crates/desktop/src/bin/desktop.rs`:

```rust
use errors::{Result, initialize_logging, initialize_panic_handler};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    initialize_logging()?;
    initialize_panic_handler(|| {})?;
    Ok(())
}
```

- [ ] **Step 5: Verify the scaffold builds**

Run:

```bash
cargo check -p desktop
```

Expected: command exits 0.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/desktop
git commit -m "Add native desktop crate scaffold"
```

---

### Task 2: Extract The Device Selection Model

**Files:**
- Create: `crates/desktop/src/device_selection.rs`
- Modify: `crates/desktop/src/lib.rs`

**Interfaces:**
- Consumes: `bpm_detection_midi::MidiInputPort`
- Produces: `DeviceSelection::new() -> Self`
- Produces: `DeviceSelection::refresh_devices(&mut self, devices: Vec<MidiInputPort>)`
- Produces: `DeviceSelection::select_index(&mut self, index: usize) -> Option<MidiInputPort>`
- Produces: `DeviceSelection::devices(&self) -> &[MidiInputPort]`
- Produces: `DeviceSelection::selected_index(&self) -> Option<usize>`
- Produces: `DeviceSelection::selected(&self) -> &MidiInputPort`

- [ ] **Step 1: Write tests for hotplug-stable selection**

Create `crates/desktop/src/device_selection.rs` with tests first:

```rust
use bpm_detection_midi::MidiInputPort;

#[derive(Debug, Clone)]
pub struct DeviceSelection {
    devices: Vec<MidiInputPort>,
    selected: MidiInputPort,
    selected_index: Option<usize>,
}

impl Default for DeviceSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceSelection {
    #[must_use]
    pub fn new() -> Self {
        Self { devices: Vec::new(), selected: MidiInputPort::None, selected_index: None }
    }

    #[must_use]
    pub fn devices(&self) -> &[MidiInputPort] {
        &self.devices
    }

    #[must_use]
    pub fn selected(&self) -> &MidiInputPort {
        &self.selected
    }

    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn virtual_port(name: &str) -> MidiInputPort {
        MidiInputPort::Virtual(name.to_string())
    }

    #[test]
    fn refresh_selects_none_when_current_device_disappears() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);
        selection.select_index(1);

        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("b")]);

        assert_eq!(selection.selected(), &MidiInputPort::None);
        assert_eq!(selection.selected_index(), Some(0));
    }

    #[test]
    fn refresh_keeps_selected_device_when_it_moves() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("b")]);
        selection.select_index(1);

        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a"), virtual_port("b")]);

        assert_eq!(selection.selected(), &virtual_port("b"));
        assert_eq!(selection.selected_index(), Some(2));
    }

    #[test]
    fn select_index_returns_selected_device() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);

        let selected = selection.select_index(1);

        assert_eq!(selected, Some(virtual_port("a")));
        assert_eq!(selection.selected_index(), Some(1));
    }
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test -p desktop device_selection
```

Expected: FAIL because `refresh_devices` and `select_index` are not implemented.

- [ ] **Step 3: Implement selection behavior**

Replace the `impl DeviceSelection` block with:

```rust
impl DeviceSelection {
    #[must_use]
    pub fn new() -> Self {
        Self { devices: Vec::new(), selected: MidiInputPort::None, selected_index: None }
    }

    pub fn refresh_devices(&mut self, mut devices: Vec<MidiInputPort>) {
        devices.sort_unstable_by(|left, right| left.sort_key().cmp(&right.sort_key()));

        let selected_index = devices.iter().position(|device| device == &self.selected);
        match selected_index {
            Some(index) => {
                self.selected_index = Some(index);
            }
            None => {
                self.selected = MidiInputPort::None;
                self.selected_index = devices.iter().position(|device| device == &MidiInputPort::None);
            }
        }

        self.devices = devices;
    }

    pub fn select_index(&mut self, index: usize) -> Option<MidiInputPort> {
        let device = self.devices.get(index)?.clone();
        self.selected = device.clone();
        self.selected_index = Some(index);
        Some(device)
    }

    #[must_use]
    pub fn devices(&self) -> &[MidiInputPort] {
        &self.devices
    }

    #[must_use]
    pub fn selected(&self) -> &MidiInputPort {
        &self.selected
    }

    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }
}
```

- [ ] **Step 4: Verify tests pass**

Run:

```bash
cargo test -p desktop device_selection
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/desktop/src/device_selection.rs crates/desktop/src/lib.rs
git commit -m "Extract desktop device selection model"
```

---

### Task 3: Move Desktop Config Shape Into `desktop`

**Files:**
- Create: `crates/desktop/config/base_config.toml`
- Create: `crates/desktop/src/config.rs`
- Create: `crates/desktop/src/live_parameters.rs`
- Modify: `crates/desktop/src/lib.rs`

**Interfaces:**
- Produces: `DesktopConfig::new() -> TypedResult<Self, config::ConfigError>`
- Produces: `DesktopConfig::save(&self) -> errors::Result<()>`
- Produces: `DesktopBaseConfig`
- Consumes: `gui::BPMDetectionConfig`

- [ ] **Step 1: Create non-TUI config defaults**

Create `crates/desktop/config/base_config.toml`:

```toml
[GUI]
interpolation_curve = 0.800000011920929

[GUI.interpolation_duration]
secs = 0
nanos = 730000000

[MIDI]
device_name = "Desktop"
enable_midi_clock = false
send_tempo = false

[static_bpm_detection_config]
bpm_range = 40
bpm_center = 90.0
sample_rate = 500

[static_bpm_detection_config.normal_distribution]
std_dev = 24.0
factor = 47.0
cutoff = 100.0
resolution = 0.699999988079071

[dynamic_bpm_detection_config]
beats_lookback = 8

[dynamic_bpm_detection_config.velocity_current_note_weight]
enabled = false
value = 0.699999988079071

[dynamic_bpm_detection_config.velocity_note_from_weight]
enabled = false
value = 0.6499999761581421

[dynamic_bpm_detection_config.time_distance_weight]
enabled = false
value = 0.7300000190734863

[dynamic_bpm_detection_config.octave_distance_weight]
enabled = false
value = 0.6499999761581421

[dynamic_bpm_detection_config.pitch_distance_weight]
enabled = false
value = 0.8500000238418579

[dynamic_bpm_detection_config.multiplier_weight]
enabled = true
value = 0.6600000262260437

[dynamic_bpm_detection_config.subdivision_weight]
enabled = false
value = 0.699999988079071

[dynamic_bpm_detection_config.in_beat_range_weight]
enabled = false
value = 0.7599999904632568

[dynamic_bpm_detection_config.normal_distribution_weight]
enabled = true
value = 1.0

[dynamic_bpm_detection_config.high_tempo_bias]
enabled = true
value = 0.0
```

- [ ] **Step 2: Create `DesktopConfig`**

Create `crates/desktop/src/config.rs`:

```rust
use std::{fs::write, path::PathBuf};

use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use bpm_detection_midi::MidiServiceConfig;
use build::{get_config_dir, get_data_dir};
use config::ConfigError;
use errors::{Report, Result, TypedResult};
use gui::GUIConfig;
use log::{error, info};
use serde::{Deserialize, Serialize};

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesktopConfig {
    #[serde(default, flatten)]
    #[serde(skip_serializing)]
    pub app: AppConfig,
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    #[serde(rename = "MIDI")]
    pub midi: MidiServiceConfig,
    #[serde(default)]
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
    #[serde(default)]
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
}

impl DesktopConfig {
    /// Load the desktop configuration from the built-in defaults and the optional user config file.
    pub fn new() -> TypedResult<Self, ConfigError> {
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();
        let data_dir_value = data_dir.to_string_lossy().to_string();
        let config_dir_value = config_dir.to_string_lossy().to_string();
        let builder = config::Config::builder()
            .set_default("_data_dir", data_dir_value)?
            .set_default("_config_dir", config_dir_value)?
            .add_source(config::File::from_str(CONFIG, config::FileFormat::Toml))
            .add_source(
                config::File::from(config_dir.join("config.toml")).format(config::FileFormat::Toml).required(false),
            );

        Ok(builder.build()?.try_deserialize()?)
    }

    /// Persist the user-editable desktop configuration to the configured config directory.
    pub fn save(&self) -> Result<()> {
        let serialized = match toml::to_string_pretty(self) {
            Ok(serialized) => serialized,
            Err(e) => {
                error!("Serialization error: {e:?}");
                return Err(Report::new(e));
            }
        };

        let config_path = get_config_dir().join("config.toml");
        info!("configuration saved at {}", config_path.display());
        Ok(write(config_path, serialized)?)
    }
}
```

- [ ] **Step 3: Create `DesktopBaseConfig` with propagation hooks**

Create `crates/desktop/src/live_parameters.rs`:

```rust
use std::{sync::{Arc, atomic::Ordering}, time::Duration};

use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionConfigAccessor, NormalDistributionConfigAccessor,
    StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor,
};
use errors::LogErrorWithExt;
use gui::{BPMDetectionConfig, GUIConfigAccessor};
use parameter::OnOff;

use crate::config::DesktopConfig;

pub type StaticConfigCallback = Arc<dyn Fn(StaticBPMDetectionConfig) + Send + Sync>;
pub type DynamicConfigCallback = Arc<dyn Fn(DynamicBPMDetectionConfig) + Send + Sync>;

pub struct DesktopBaseConfig {
    pub config: DesktopConfig,
    pub on_static_config_changed: StaticConfigCallback,
    pub on_dynamic_config_changed: DynamicConfigCallback,
}

impl DesktopBaseConfig {
    pub fn propagate_static_changes(&self) {
        (self.on_static_config_changed)(self.config.static_bpm_detection_config.clone());
    }

    pub fn propagate_dynamic_changes(&self) {
        (self.on_dynamic_config_changed)(self.config.dynamic_bpm_detection_config.clone());
    }
}

impl NormalDistributionConfigAccessor for DesktopBaseConfig {
    fn std_dev(&self) -> f64 {
        self.config.static_bpm_detection_config.normal_distribution.std_dev
    }

    fn factor(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.factor
    }

    fn cutoff(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.cutoff
    }

    fn resolution(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.resolution
    }

    fn set_std_dev(&mut self, val: f64) {
        self.config.static_bpm_detection_config.normal_distribution.std_dev = val;
        self.propagate_static_changes();
    }

    fn set_factor(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.factor = val;
        self.propagate_static_changes();
    }

    fn set_cutoff(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.cutoff = val;
        self.propagate_static_changes();
    }

    fn set_resolution(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.resolution = val;
        self.propagate_static_changes();
    }
}

impl DynamicBPMDetectionConfigAccessor for DesktopBaseConfig {
    fn beats_lookback(&self) -> u8 {
        self.config.dynamic_bpm_detection_config.beats_lookback
    }

    fn velocity_current_note_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.velocity_current_note_weight
    }

    fn velocity_note_from_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.velocity_note_from_weight
    }

    fn time_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.time_distance_weight
    }

    fn octave_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.octave_distance_weight
    }

    fn pitch_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.pitch_distance_weight
    }

    fn multiplier_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.multiplier_weight
    }

    fn subdivision_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.subdivision_weight
    }

    fn in_beat_range_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.in_beat_range_weight
    }

    fn normal_distribution_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.normal_distribution_weight
    }

    fn high_tempo_bias(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.high_tempo_bias
    }

    fn set_beats_lookback(&mut self, val: u8) {
        self.config.dynamic_bpm_detection_config.beats_lookback = val;
        self.propagate_dynamic_changes();
    }

    fn set_velocity_current_note_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.velocity_current_note_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_velocity_note_from_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.velocity_note_from_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_time_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.time_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_octave_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.octave_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_pitch_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.pitch_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_multiplier_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.multiplier_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_subdivision_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.subdivision_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_in_beat_range_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.in_beat_range_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_normal_distribution_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.normal_distribution_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_high_tempo_bias(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.high_tempo_bias = val;
        self.propagate_dynamic_changes();
    }
}

impl StaticBPMDetectionConfigAccessor for DesktopBaseConfig {
    fn bpm_center(&self) -> f32 {
        self.config.static_bpm_detection_config.bpm_center
    }

    fn bpm_range(&self) -> u16 {
        self.config.static_bpm_detection_config.bpm_range
    }

    fn sample_rate(&self) -> u16 {
        self.config.static_bpm_detection_config.sample_rate
    }

    fn index_to_bpm(&self, index: usize) -> f32 {
        self.config.static_bpm_detection_config.index_to_bpm(index)
    }

    fn highest_bpm(&self) -> f32 {
        self.config.static_bpm_detection_config.highest_bpm()
    }

    fn lowest_bpm(&self) -> f32 {
        self.config.static_bpm_detection_config.lowest_bpm()
    }

    fn set_bpm_center(&mut self, val: f32) {
        self.config.static_bpm_detection_config.bpm_center = val;
        self.propagate_static_changes();
    }

    fn set_bpm_range(&mut self, val: u16) {
        self.config.static_bpm_detection_config.bpm_range = val;
        self.propagate_static_changes();
    }

    fn set_sample_rate(&mut self, val: u16) {
        self.config.static_bpm_detection_config.sample_rate = val;
        self.propagate_static_changes();
    }
}

impl GUIConfigAccessor for DesktopBaseConfig {
    fn interpolation_duration(&self) -> Duration {
        self.config.gui_config.interpolation_duration
    }

    fn interpolation_curve(&self) -> f32 {
        self.config.gui_config.interpolation_curve
    }

    fn set_interpolation_duration(&mut self, val: Duration) {
        self.config.gui_config.interpolation_duration = val;
        self.propagate_dynamic_changes();
    }

    fn set_interpolation_curve(&mut self, val: f32) {
        self.config.gui_config.interpolation_curve = val;
        self.propagate_dynamic_changes();
    }
}

impl BPMDetectionConfig for DesktopBaseConfig {
    fn get_send_tempo(&self) -> bool {
        self.config.midi.send_tempo.load(Ordering::Relaxed)
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.config.midi.send_tempo.store(enabled, Ordering::Relaxed);
    }

    fn save(&mut self) {
        self.config.save().log_error_msg("Could not save configuration").ok();
    }
}
```

- [ ] **Step 4: Verify config crate compiles**

Run:

```bash
cargo check -p desktop
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/desktop/config crates/desktop/src/config.rs crates/desktop/src/live_parameters.rs crates/desktop/src/lib.rs
git commit -m "Move desktop config shape into desktop crate"
```

---

### Task 4: Add Desktop Controller Boundary

**Files:**
- Create: `crates/desktop/src/controller.rs`
- Modify: `crates/desktop/src/lib.rs`

**Interfaces:**
- Produces: `DesktopController<B>`
- Produces: `DesktopController::new(midi_service_config, static_config, dynamic_config, on_device_change, on_midi_message, bpm_detection_receiver) -> Result<Self>`
- Produces: `DesktopController::refresh_devices(&mut self) -> Result<()>`
- Produces: `DesktopController::select_device_index(&mut self, index: usize) -> Result<()>`
- Produces: `DesktopController::apply_static_config(&self, config: StaticBPMDetectionConfig) -> Result<()>`
- Produces: `DesktopController::apply_dynamic_config(&self, config: DynamicBPMDetectionConfig) -> Result<()>`

- [ ] **Step 1: Implement controller over `bpm_detection_midi::MidiService`**

Create `crates/desktop/src/controller.rs`:

```rust
use std::sync::Arc;

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use bpm_detection_midi::{MidiIn, MidiInputConnection, MidiServiceConfig, TimedMidiMessage, to_owned_event};
use errors::Result;
use log::error;
use sync::{ArcRwLock, ArcRwLockExt, RwLock};

use crate::device_selection::DeviceSelection;

pub type MidiMessageCallback = Arc<dyn Fn(TimedMidiMessage) + Send + Sync>;
pub type DeviceChangeCallback = Arc<dyn Fn() + Send + Sync>;

pub struct DesktopController<B>
where
    B: BPMDetectionReceiver,
{
    selection: DeviceSelection,
    midi_service: ArcRwLock<bpm_detection_midi::MidiService<B>>,
    on_midi_message: MidiMessageCallback,
}

impl<B> DesktopController<B>
where
    B: BPMDetectionReceiver,
{
    /// Start the native MIDI service thread and wrap it behind the desktop controller boundary.
    pub fn new(
        midi_service_config: MidiServiceConfig,
        static_config: StaticBPMDetectionConfig,
        dynamic_config: DynamicBPMDetectionConfig,
        _on_device_change: DeviceChangeCallback,
        on_midi_message: MidiMessageCallback,
        bpm_detection_receiver: B,
    ) -> Result<Self> {
        let midi_service = bpm_detection_midi::MidiService::new(
            midi_service_config,
            static_config,
            dynamic_config,
            #[cfg(target_os = "macos")]
            move || (_on_device_change)(),
            bpm_detection_receiver,
        )?;

        Ok(Self { selection: DeviceSelection::new(), midi_service: Arc::new(RwLock::new(midi_service)), on_midi_message })
    }

    #[must_use]
    pub fn device_selection(&self) -> &DeviceSelection {
        &self.selection
    }

    fn execute<R, F>(&self, command: F) -> Result<R>
    where
        F: FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) -> Result<R> + Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        self.midi_service.get(|midi_service| midi_service.execute(command))
    }

    /// Refresh the known MIDI input list while preserving the selected device when it is still present.
    pub fn refresh_devices(&mut self) -> Result<()> {
        let devices = self.execute(|midi_in, _| midi_in.get_ports())?;
        self.selection.refresh_devices(devices);
        Ok(())
    }

    /// Select a MIDI input by the current displayed device index and reconnect the MIDI listener.
    pub fn select_device_index(&mut self, index: usize) -> Result<()> {
        let Some(port) = self.selection.select_index(index) else {
            return Ok(());
        };

        let on_midi_message = self.on_midi_message.clone();
        self.execute(move |midi_in, midi_input_connection| {
            match midi_in.listen(&port, move |event| {
                on_midi_message(to_owned_event(event));
            }) {
                Ok(input_connection) => *midi_input_connection = input_connection,
                Err(err) => error!("error while selecting device: {err:?}"),
            }
            Ok(())
        })?;

        Ok(())
    }

    /// Apply static BPM detection settings that require rebuilding the detection buffers.
    pub fn apply_static_config(&self, config: StaticBPMDetectionConfig) -> Result<()> {
        self.execute(move |midi_in, _| midi_in.change_bpm_detection_config(config))
    }

    /// Apply dynamic BPM detection settings that can be changed on the running service.
    pub fn apply_dynamic_config(&self, config: DynamicBPMDetectionConfig) -> Result<()> {
        self.execute(move |midi_in, _| midi_in.change_bpm_detection_config_live(config))
    }
}
```

- [ ] **Step 2: Verify controller compiles**

Run:

```bash
cargo check -p desktop
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/desktop/src/controller.rs crates/desktop/src/lib.rs
git commit -m "Add desktop MIDI controller boundary"
```

---

### Task 5: Add A Shared GUI Extension Hook

**Files:**
- Modify: `crates/gui/src/application_parameters.rs`
- Modify: `crates/gui/src/config_ui.rs`

**Interfaces:**
- Produces: `BPMDetectionConfig::desktop_controls(&mut self, ui: &mut eframe::egui::Ui)` default no-op
- Consumes: existing `BPMDetectionConfig` implementors in plugin, WASM, desktop

- [ ] **Step 1: Add a default no-op hook**

Modify `crates/gui/src/application_parameters.rs`:

```rust
use eframe::egui::Ui;

pub trait BPMDetectionConfig:
    NormalDistributionConfigAccessor
    + DynamicBPMDetectionConfigAccessor
    + StaticBPMDetectionConfigAccessor
    + GUIConfigAccessor
{
    fn get_send_tempo(&self) -> bool;
    fn set_send_tempo(&mut self, enabled: bool);
    fn save(&mut self) {}
    fn desktop_controls(&mut self, _ui: &mut Ui) {}
}
```

- [ ] **Step 2: Call the hook from the settings panel**

Modify `crates/gui/src/config_ui.rs` after the `Send tempo` toggle:

```rust
            config.desktop_controls(ui);
```

- [ ] **Step 3: Verify all GUI consumers still compile**

Run:

```bash
cargo check -p gui
cargo check -p wasm --target wasm32-unknown-unknown
cargo check -p midi-bpm-detector-plugin
```

Expected: all commands exit 0.

- [ ] **Step 4: Commit**

```bash
git add crates/gui/src/application_parameters.rs crates/gui/src/config_ui.rs
git commit -m "Add mode-specific GUI controls hook"
```

---

### Task 6: Render Device Selection From Desktop Config

**Files:**
- Modify: `crates/desktop/src/live_parameters.rs`
- Modify: `crates/desktop/src/controller.rs` if controller needs a UI-safe handle

**Interfaces:**
- Consumes: `BPMDetectionConfig::desktop_controls`
- Consumes: `DesktopController`
- Produces: egui combo/selector for native MIDI devices

- [ ] **Step 1: Add controller slot to `DesktopBaseConfig`**

Modify `DesktopBaseConfig`:

```rust
use bpm_detection_core::bpm_detection_receiver::BPMDetectionReceiver;

use crate::controller::DesktopController;

pub type DesktopControllerSlot<B> = Arc<sync::Mutex<Option<DesktopController<B>>>>;

pub struct DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    pub config: DesktopConfig,
    pub controller: DesktopControllerSlot<B>,
    pub on_static_config_changed: StaticConfigCallback,
    pub on_dynamic_config_changed: DynamicConfigCallback,
}
```

- [ ] **Step 2: Make the config accessor impls generic**

Apply these exact signature changes in `crates/desktop/src/live_parameters.rs`:

```rust
impl DesktopBaseConfig {
```

becomes:

```rust
impl<B> DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
```

```rust
impl NormalDistributionConfigAccessor for DesktopBaseConfig {
```

becomes:

```rust
impl<B> NormalDistributionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
```

```rust
impl DynamicBPMDetectionConfigAccessor for DesktopBaseConfig {
```

becomes:

```rust
impl<B> DynamicBPMDetectionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
```

```rust
impl StaticBPMDetectionConfigAccessor for DesktopBaseConfig {
```

becomes:

```rust
impl<B> StaticBPMDetectionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
```

```rust
impl GUIConfigAccessor for DesktopBaseConfig {
```

becomes:

```rust
impl<B> GUIConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
```

- [ ] **Step 3: Implement desktop controls**

Replace the existing `BPMDetectionConfig` impl with:

```rust
impl<B> BPMDetectionConfig for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    fn get_send_tempo(&self) -> bool {
        self.config.midi.send_tempo.load(Ordering::Relaxed)
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.config.midi.send_tempo.store(enabled, Ordering::Relaxed);
    }

    fn save(&mut self) {
        self.config.save().log_error_msg("Could not save configuration").ok();
    }

    fn desktop_controls(&mut self, ui: &mut gui::eframe::egui::Ui) {
        let mut controller_slot = self.controller.lock();
        let Some(controller) = controller_slot.as_mut() else {
            ui.label("MIDI service is starting");
            return;
        };

        let devices = controller.device_selection().devices().to_vec();
        let selected = controller.device_selection().selected().clone();
        let mut selected_index = controller.device_selection().selected_index().unwrap_or_default();

        gui::eframe::egui::ComboBox::from_label("MIDI input")
            .selected_text(selected.as_str())
            .show_ui(ui, |ui| {
                for (index, device) in devices.iter().enumerate() {
                    ui.selectable_value(&mut selected_index, index, device.as_str());
                }
            });

        if Some(selected_index) != controller.device_selection().selected_index() {
            controller.select_device_index(selected_index).log_error_msg("Could not select MIDI input").ok();
        }

        if ui.button("Refresh MIDI inputs").clicked() {
            controller.refresh_devices().log_error_msg("Could not refresh MIDI input list").ok();
        }
    }
}
```

- [ ] **Step 4: Verify desktop compiles**

Run:

```bash
cargo check -p desktop
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/desktop/src/live_parameters.rs crates/desktop/src/controller.rs
git commit -m "Render native MIDI input selection in desktop GUI"
```

---

### Task 7: Start egui Directly From The Desktop Binary

**Files:**
- Modify: `crates/desktop/src/bin/desktop.rs`
- Modify: `Cargo.toml` default members only after this works

**Interfaces:**
- Consumes: `DesktopConfig`
- Consumes: `DesktopController`
- Consumes: `gui::create_gui`
- Consumes: `gui::start_gui`

- [ ] **Step 1: Wire the direct desktop main with an explicit controller slot**

Replace `crates/desktop/src/bin/desktop.rs` with:

```rust
use std::sync::Arc;

use desktop::{
    config::DesktopConfig,
    controller::DesktopController,
    live_parameters::{DesktopBaseConfig, DesktopControllerSlot},
};
use errors::{Result, initialize_logging, initialize_panic_handler};
use gui::{create_gui, start_gui};
use mimalloc::MiMalloc;
use sync::Mutex;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    initialize_logging()?;
    initialize_panic_handler(|| {})?;

    let config = DesktopConfig::new()?;
    let controller: DesktopControllerSlot<gui::GuiRemote> = Arc::new(Mutex::new(None));

    let static_controller = controller.clone();
    let dynamic_controller = controller.clone();
    let (gui_remote, app_builder) = create_gui(DesktopBaseConfig {
        config: config.clone(),
        controller: controller.clone(),
        on_static_config_changed: Arc::new(move |static_config| {
            if let Some(controller) = static_controller.lock().as_ref() {
                controller.apply_static_config(static_config).ok();
            }
        }),
        on_dynamic_config_changed: Arc::new(move |dynamic_config| {
            if let Some(controller) = dynamic_controller.lock().as_ref() {
                controller.apply_dynamic_config(dynamic_config).ok();
            }
        }),
    });

    controller.lock().replace(DesktopController::new(
        config.midi,
        config.static_bpm_detection_config,
        config.dynamic_bpm_detection_config,
        Arc::new({
            let gui_remote = gui_remote.clone();
            move || gui_remote.request_repaint()
        }),
        Arc::new(|_| {}),
        gui_remote,
    )?);

    start_gui(app_builder)
}
```

- [ ] **Step 2: Compile the direct desktop main**

Run:

```bash
cargo check -p desktop
```

Expected: PASS.

- [ ] **Step 3: Run the desktop binary**

Run:

```bash
cargo run -p desktop --bin desktop
```

Expected: egui window opens directly without Ratatui. MIDI input selector appears in settings.

- [ ] **Step 4: Commit**

```bash
git add crates/desktop/src/bin/desktop.rs Cargo.toml
git commit -m "Start native desktop GUI directly"
```

---

### Task 8: Retire TUI Usage Incrementally

**Files:**
- Modify: root `Cargo.toml`
- Modify: `scripts/dev.sh`
- Modify: `docs/development.md`
- Modify: `docs/architecture.md`
- Modify or remove: `crates/tui`

**Interfaces:**
- Consumes: working `desktop` binary from Task 7
- Produces: documented desktop command

- [ ] **Step 1: Update development commands**

Change desktop run/check commands in `scripts/dev.sh` and `docs/development.md` from TUI-first commands to:

```bash
cargo run -p desktop --bin desktop
cargo check -p desktop
```

- [ ] **Step 2: Update workspace defaults**

Change root `Cargo.toml`:

```toml
default-members = ["crates/desktop", "crates/midi-reset"]
```

- [ ] **Step 3: Keep `crates/tui` as a legacy comparison path for now**

Keep it out of default members and document it as legacy:

```toml
members = [
    "crates/tui",
    "crates/desktop",
    "crates/errors",
    "crates/build",
    "crates/bpm_detection_core",
    "crates/bpm_detection_midi",
    "crates/gui",
    "crates/sync",
    "crates/parameter",
    "crates/wasm",
    "crates/midi-bpm-detector-plugin",
    "crates/midi-bpm-detector-plugin/xtask",
    "crates/midi-reset",
]
```

Do not delete `crates/tui` in this task. Deletion is a separate review step after the direct desktop path has replaced
controller selection and startup behavior.

- [ ] **Step 4: Verify expected build set**

Run:

```bash
./scripts/dev.sh check-desktop
./scripts/dev.sh clippy-all
```

Expected: PASS. If `clippy-all` includes legacy TUI and fails only because of TUI-only code, stop this task without an
allow, report the failing lint, and make the next review decision explicit before changing lint scope or deleting TUI.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml scripts/dev.sh docs/development.md docs/architecture.md crates/tui crates/desktop
git commit -m "Make native desktop crate the default desktop path"
```
