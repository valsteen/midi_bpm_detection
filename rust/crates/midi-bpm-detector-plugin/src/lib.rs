#![allow(clippy::struct_field_names)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::module_name_repetitions)]

mod bpm_detector_configuration;
mod gui;
mod parameter_sync;
mod plugin_parameters;
mod task_executor;

use std::{
    num::NonZeroU16,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bpm_detection_core::{
    BPMDetection, TimedNoteOn,
    note_events::NoteOn,
    parameters::{duration_to_sample, sample_to_duration},
};
use crossbeam::atomic::AtomicCell;
#[cfg(not(debug_assertions))]
use mimalloc::MiMalloc;
use nih_plug::{log::error, midi::MidiResult, prelude::*};
use nih_plug_egui::create_egui_editor;
use ringbuf::{SharedRb, StaticRb, producer::Producer, storage::Array, traits::Split, wrap::frozen::Frozen};
use sync::{ArcAtomicBool, ArcAtomicOptionNonZeroU16, ArcAtomicOptionUsize, RwLock};

use crate::{
    bpm_detector_configuration::PluginConfig,
    gui::GuiEditor,
    parameter_sync::{HOST_PARAMETER_SYNC_COALESCING_WINDOW, ParameterSyncOrigin},
    plugin_parameters::MidiBpmDetectorParams,
    task_executor::{Event, Task},
};

fn midi_note_on_from_message(event: &wmidi::MidiMessage<'_>) -> Option<NoteOn> {
    if let wmidi::MidiMessage::NoteOn(channel, note, velocity) = event {
        return Some(NoteOn { channel: channel.index(), pitch: *note as u8, velocity: u8::from(*velocity) });
    }
    None
}

#[cfg(not(debug_assertions))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub struct MidiBpmDetector {
    params: Arc<MidiBpmDetectorParams>,
    current_sample: Arc<AtomicUsize>,
    timing: PluginTiming,
    // should recompute bpm evaluation, even if there is no new notes. Happens after config change
    // or GUI just reopened
    force_evaluate_bpm_detection: ArcAtomicBool,
    events_sender: Frozen<Arc<SharedRb<Array<Event, 1000>>>, true, false>,
    task_executor_handoff: Option<task_executor::TaskExecutor>,
    gui_editor_handoff: Option<GuiEditor>,
    static_bpm_detection_config_changed_at: DeferredConfigUpdate,
    dynamic_bpm_detection_config_changed_at: DeferredConfigUpdate,
}

const INITIAL_CONFIG_SYNC_SAMPLE: usize = 1;

#[derive(Clone)]
pub(crate) struct DeferredConfigUpdate {
    changed_at_sample: ArcAtomicOptionUsize,
}

impl DeferredConfigUpdate {
    fn pending_initial_sync() -> Self {
        Self { changed_at_sample: ArcAtomicOptionUsize::new(Some(INITIAL_CONFIG_SYNC_SAMPLE)) }
    }

    #[cfg(test)]
    fn idle() -> Self {
        Self { changed_at_sample: ArcAtomicOptionUsize::none() }
    }

    pub(crate) fn mark_changed_at_if_idle(&self, current_sample: usize) {
        self.changed_at_sample.store_if_none(Some(current_sample), Ordering::Relaxed);
    }

    fn changed_at_sample(&self) -> Option<usize> {
        self.changed_at_sample.load(Ordering::Relaxed)
    }

    fn take(&self) -> Option<usize> {
        self.changed_at_sample.take(Ordering::Relaxed)
    }
}

#[derive(Default)]
enum PluginTiming {
    #[default]
    AwaitingHostInitialization,
    Ready {
        sample_rate: NonZeroU16,
    },
}

impl PluginTiming {
    fn initialize(&mut self, sample_rate: f32) -> bool {
        let Some(sample_rate) = NonZeroU16::new(sample_rate as u16) else {
            *self = Self::AwaitingHostInitialization;
            return false;
        };

        *self = Self::Ready { sample_rate };
        true
    }

    fn sample_rate(&self) -> Option<u16> {
        match self {
            Self::AwaitingHostInitialization => None,
            Self::Ready { sample_rate } => Some(sample_rate.get()),
        }
    }
}

impl Default for MidiBpmDetector {
    fn default() -> Self {
        let current_sample = Arc::new(AtomicUsize::new(0));
        let (events_sender, events_receiver) = StaticRb::<Event, 1000>::default().split();
        let events_sender: Frozen<Arc<SharedRb<Array<Event, 1000>>>, true, false> = events_sender.freeze();
        let events_receiver: Frozen<Arc<SharedRb<Array<Event, 1000>>>, false, true> = events_receiver.freeze();
        let gui_remote_receiver = Arc::new(AtomicCell::new(None));
        let gui_remote = None;
        let daw_port = ArcAtomicOptionNonZeroU16::none();

        let mut config = PluginConfig::default();
        let bpm_detection = BPMDetection::new(config.static_bpm_detection_config.clone());

        let static_bpm_detection_config_changed_at = DeferredConfigUpdate::pending_initial_sync();
        let dynamic_bpm_detection_config_changed_at = DeferredConfigUpdate::pending_initial_sync();

        let params = Arc::new(MidiBpmDetectorParams::new(
            &mut config,
            &static_bpm_detection_config_changed_at,
            &dynamic_bpm_detection_config_changed_at,
            &current_sample,
            &daw_port,
        ));

        let shared_config = Arc::new(RwLock::new(config.clone()));
        let gui_must_update_config = ArcAtomicBool::new(false);

        let task_executor = task_executor::TaskExecutor {
            bpm_detection,
            dynamic_bpm_detection_config: config.dynamic_bpm_detection_config,
            gui_remote,
            params: params.clone(),
            gui_remote_receiver: gui_remote_receiver.clone(),
            events_receiver,
            config: shared_config.clone(),
            gui_must_update_config: gui_must_update_config.clone(),
            daw_port,
            daw_connection: None,
            send_tempo: config.send_tempo.clone(),
        };

        let force_evaluate_bpm_detection = ArcAtomicBool::new(false);

        let gui_editor = GuiEditor {
            editor_state: params.editor_state.clone(),
            bpm_detection_app: None,
            gui_remote_receiver: gui_remote_receiver.clone(),
            force_evaluate_bpm_detection: force_evaluate_bpm_detection.clone(),
            config: shared_config,
            params: params.clone(),
            gui_must_update_config,
        };

        Self {
            params,
            current_sample,
            timing: PluginTiming::default(),
            force_evaluate_bpm_detection,
            events_sender,
            task_executor_handoff: Some(task_executor),
            gui_editor_handoff: Some(gui_editor),
            static_bpm_detection_config_changed_at,
            dynamic_bpm_detection_config_changed_at,
        }
    }
}

impl Plugin for MidiBpmDetector {
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the midi-bpm-detector-plugin does not have any background
    // tasks.
    type BackgroundTask = Task;
    // If the midi-bpm-detector-plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the midi-bpm-detector-plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];
    const EMAIL: &'static str = "vincent.alsteen@gmail.com";
    const HARD_REALTIME_ONLY: bool = true;
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const NAME: &'static str = "Midi BPM Detector";
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const VENDOR: &'static str = "Vincent Alsteen";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        // guaranteed to be called once by nih-plug
        let mut task_executor = self.task_executor_handoff.take().unwrap();
        Box::new(move |task| task_executor.execute(task))
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // guaranteed to be called once by nih-plug
        let gui_editor = self.gui_editor_handoff.take().unwrap();
        create_egui_editor(
            self.params.editor_state.clone(),
            (async_executor, gui_editor),
            |egui_ctx, (async_executor, gui_editor)| gui_editor.build(egui_ctx, async_executor.clone()),
            |egui_ctx, setter, (_async_executor, gui_editor)| gui_editor.update(setter, egui_ctx),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.timing.initialize(buffer_config.sample_rate)
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let Some(sample_rate) = self.timing.sample_rate() else {
            return if self.params.editor_state.is_open() { ProcessStatus::KeepAlive } else { ProcessStatus::Normal };
        };
        let current_sample = self.current_sample.load(Ordering::Relaxed);
        let delay_by = duration_to_sample(sample_rate, HOST_PARAMETER_SYNC_COALESCING_WINDOW);
        Self::execute_at_delay(current_sample, delay_by, &self.static_bpm_detection_config_changed_at, || {
            context.execute_background(Task::StaticBPMDetectionConfig(ParameterSyncOrigin::Host));
        });
        Self::execute_at_delay(current_sample, delay_by, &self.dynamic_bpm_detection_config_changed_at, || {
            context.execute_background(Task::DynamicBPMDetectionConfig(ParameterSyncOrigin::Host));
        });
        self.receive_notes_at_sample_rate(context, sample_rate);
        self.current_sample.fetch_add(buffer.samples(), Ordering::Relaxed);
        if self.params.editor_state.is_open() { ProcessStatus::KeepAlive } else { ProcessStatus::Normal }
    }
}

impl MidiBpmDetector {
    fn receive_notes_at_sample_rate<P>(&mut self, context: &mut P, sample_rate: u16) -> bool
    where
        P: ProcessContext<Self>,
    {
        let current_sample = self.current_sample.load(Ordering::Relaxed);
        let mut has_new_events = false;
        if let Some(bpm) = context.transport().tempo {
            if self.events_sender.try_push(Event::DawBPM(bpm as f32)).is_err() {
                error!("event ringbuffer is full");
            }
            has_new_events = true;
        }
        while let Some(event) = context.next_event() {
            context.send_event(event);
            let Some(midi_event) = event.as_midi() else {
                continue;
            };
            let MidiResult::Basic(bytes) = midi_event else {
                continue;
            };
            let Ok(midi_message) = wmidi::MidiMessage::from_bytes(&bytes) else {
                continue;
            };
            let Some(midi_note_on) = midi_note_on_from_message(&midi_message) else {
                continue;
            };

            let note_sample = current_sample + event.timing() as usize;
            let timestamp = sample_to_duration(sample_rate, note_sample);

            if self.events_sender.try_push(Event::TimedNoteOn(TimedNoteOn { timestamp, event: midi_note_on })).is_err()
            {
                error!("event ringbuffer is full");
            }

            has_new_events = true;
        }

        let force_evaluate_bpm_detection = self.force_evaluate_bpm_detection.take(Ordering::Relaxed);
        if has_new_events || force_evaluate_bpm_detection {
            context.execute_background(Task::ProcessNotes { force_evaluate_bpm_detection });
        }

        self.events_sender.sync();
        has_new_events
    }

    fn execute_at_delay(sample: usize, delay_by: usize, deferred_update: &DeferredConfigUpdate, cb: impl Fn()) {
        let Some(changed_at_sample) = deferred_update.changed_at_sample() else {
            return;
        };
        if Self::has_delay_elapsed(sample, changed_at_sample, delay_by) {
            cb();
            deferred_update.take();
        }
    }

    fn has_delay_elapsed(sample: usize, changed_at_sample: usize, delay_by: usize) -> bool {
        sample >= changed_at_sample.saturating_add(delay_by)
    }
}

#[cfg(test)]
mod tests {
    use super::{DeferredConfigUpdate, MidiBpmDetector, PluginTiming};

    #[test]
    fn delay_has_not_elapsed_before_target_sample() {
        assert!(!MidiBpmDetector::has_delay_elapsed(14, 10, 5));
    }

    #[test]
    fn delay_has_elapsed_at_target_sample() {
        assert!(MidiBpmDetector::has_delay_elapsed(15, 10, 5));
    }

    #[test]
    fn delay_uses_saturating_addition() {
        assert!(!MidiBpmDetector::has_delay_elapsed(usize::MAX - 1, usize::MAX - 1, 10));
        assert!(MidiBpmDetector::has_delay_elapsed(usize::MAX, usize::MAX - 1, 10));
    }

    #[test]
    fn plugin_timing_has_no_sample_rate_before_host_initialization() {
        let timing = PluginTiming::default();

        assert_eq!(timing.sample_rate(), None);
    }

    #[test]
    fn plugin_timing_exposes_sample_rate_after_host_initialization() {
        let mut timing = PluginTiming::default();

        assert!(timing.initialize(48_000.0));

        assert_eq!(timing.sample_rate(), Some(48_000));
    }

    #[test]
    fn plugin_timing_rejects_zero_sample_rate() {
        let mut timing = PluginTiming::default();

        assert!(!timing.initialize(0.0));

        assert_eq!(timing.sample_rate(), None);
    }

    #[test]
    fn deferred_config_update_names_initial_sync_sample() {
        let update = DeferredConfigUpdate::pending_initial_sync();

        assert_eq!(update.changed_at_sample(), Some(1));
    }

    #[test]
    fn deferred_config_update_preserves_first_change_sample_until_taken() {
        let update = DeferredConfigUpdate::idle();

        update.mark_changed_at_if_idle(8);
        update.mark_changed_at_if_idle(13);

        assert_eq!(update.changed_at_sample(), Some(8));
        assert_eq!(update.take(), Some(8));
        assert_eq!(update.changed_at_sample(), None);
    }
}

impl ClapPlugin for MidiBpmDetector {
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Midi midi-bpm-detector-plugin that will estimate the BPM of the midi input");
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Analyzer, ClapFeature::Utility];
    const CLAP_ID: &'static str = "mbd.local";
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    fn remote_controls(&self, context: &mut impl RemoteControlsContext) {
        context.add_section("Send tempo", |section| {
            section.add_page("Send tempo", |page| page.add_param(&self.params.send_tempo));
        });
        context.add_section("Static parameters", |section| {
            section.add_page("Range and resolution", |page| {
                page.add_param(&self.params.static_params.bpm_center);
                page.add_param(&self.params.static_params.bpm_range);
                page.add_param(&self.params.static_params.sample_rate);
            });
            section.add_page("Normal distribution", |page| {
                page.add_param(&self.params.static_params.normal_distribution.resolution);
                page.add_param(&self.params.static_params.normal_distribution.factor);
                page.add_param(&self.params.static_params.normal_distribution.cutoff);
                page.add_param(&self.params.static_params.normal_distribution.std_dev);
            });
        });
        context.add_section("Dynamic parameters", |section| {
            section.add_page("Dynamic parameters", |page| {
                self.params.dynamic_params.add_remote_controls(page);
            });
        });
    }
}

impl Vst3Plugin for MidiBpmDetector {
    const VST3_CLASS_ID: [u8; 16] = *b"MidiBPMDetector!";
    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(MidiBpmDetector);
nih_export_vst3!(MidiBpmDetector);
