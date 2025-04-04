#![allow(forbidden_lint_groups)]
#![allow(clippy::struct_field_names)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::module_name_repetitions)]

mod bpm_detector_configuration;
mod gui;
mod plugin_parameters;
mod task_executor;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use bpm_detection_core::{
    BPMDetection, TimedMidiNoteOn,
    bpm::sample_to_duration,
    midi_messages::{MidiNoteOn, wmidi},
};
use chrono::Duration;
use crossbeam::atomic::AtomicCell;
use nih_plug::{log::error, midi::MidiResult, prelude::*};
use nih_plug_egui::create_egui_editor;
use ringbuf::{SharedRb, StaticRb, producer::Producer, storage::Array, traits::Split, wrap::frozen::Frozen};
use sync::{ArcAtomicBool, ArcAtomicOptional, RwLock};

use crate::{
    bpm_detector_configuration::Config,
    gui::GuiEditor,
    plugin_parameters::MidiBpmDetectorParams,
    task_executor::{Event, Task, UpdateOrigin},
};

pub struct MidiBpmDetector {
    params: Arc<MidiBpmDetectorParams>,
    current_sample: Arc<AtomicUsize>,
    sample_rate: u16,
    // should recompute bpm evaluation, even if there is no new notes. Happens after config change
    // or GUI just reopened
    force_evaluate_bpm_detection: ArcAtomicBool,
    events_sender: Frozen<Arc<SharedRb<Array<Event, 1000>>>, true, false>,
    task_executor: Option<task_executor::TaskExecutor>,
    gui_editor: Option<GuiEditor>,
    static_bpm_detection_parameters_changed_at: ArcAtomicOptional<usize>,
    dynamic_bpm_detection_parameters_changed_at: ArcAtomicOptional<usize>,
}

impl Default for MidiBpmDetector {
    fn default() -> Self {
        let current_sample = Arc::new(AtomicUsize::new(0));
        let (events_sender, events_receiver) = StaticRb::<Event, 1000>::default().split();
        let events_sender: Frozen<Arc<SharedRb<Array<Event, 1000>>>, true, false> = events_sender.freeze();
        let events_receiver: Frozen<Arc<SharedRb<Array<Event, 1000>>>, false, true> = events_receiver.freeze();
        let gui_remote_receiver = Arc::new(AtomicCell::new(None));
        let gui_remote = None;
        let daw_port = ArcAtomicOptional::<u16>::new(None);

        let mut config = Config::default();
        let bpm_detection = BPMDetection::new(config.static_bpm_detection_parameters.clone());

        // set a dummy value so GUI params are updated from saved daw parameters at startup
        let static_bpm_detection_parameters_changed_at = ArcAtomicOptional::<usize>::new(Some(1));
        let dynamic_bpm_detection_parameters_changed_at = ArcAtomicOptional::<usize>::new(Some(1));

        let params = Arc::new(MidiBpmDetectorParams::new(
            &mut config,
            static_bpm_detection_parameters_changed_at.clone(),
            dynamic_bpm_detection_parameters_changed_at.clone(),
            current_sample.clone(),
            daw_port.clone(),
        ));

        let shared_config = Arc::new(RwLock::new(config.clone()));
        let gui_must_update_config = ArcAtomicBool::new(false);

        let task_executor = task_executor::TaskExecutor {
            bpm_detection,
            dynamic_bpm_detection_parameters: config.dynamic_bpm_detection_parameters,
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
            bpm_detection_gui: None,
            gui_remote_receiver: gui_remote_receiver.clone(),
            force_evaluate_bpm_detection: force_evaluate_bpm_detection.clone(),
            config: shared_config,
            params: params.clone(),
            gui_must_update_config,
        };

        Self {
            params,
            current_sample,
            sample_rate: 0,
            force_evaluate_bpm_detection,
            events_sender,
            task_executor: Some(task_executor),
            gui_editor: Some(gui_editor),
            static_bpm_detection_parameters_changed_at,
            dynamic_bpm_detection_parameters_changed_at,
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
        let mut task_executor = self.task_executor.take().unwrap();
        Box::new(move |task| task_executor.execute(task))
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let gui_editor = self.gui_editor.take().unwrap();
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
        self.sample_rate = buffer_config.sample_rate as u16;
        true
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
        if let Some(static_bpm_detection_parameters_changed_at) =
            self.static_bpm_detection_parameters_changed_at.load(Ordering::Relaxed)
        {
            let duration_since_change = sample_to_duration(
                self.sample_rate,
                self.current_sample.load(Ordering::Relaxed).saturating_sub(static_bpm_detection_parameters_changed_at),
            );
            if duration_since_change > Duration::milliseconds(50) {
                context.execute_background(Task::StaticBPMDetectionParameters(UpdateOrigin::Daw));
                self.static_bpm_detection_parameters_changed_at.store(None, Ordering::Relaxed);
            }
        }
        if let Some(dynamic_bpm_detection_parameters_changed_at) =
            self.dynamic_bpm_detection_parameters_changed_at.load(Ordering::Relaxed)
        {
            let duration_since_change = sample_to_duration(
                self.sample_rate,
                self.current_sample.load(Ordering::Relaxed).saturating_sub(dynamic_bpm_detection_parameters_changed_at),
            );
            if duration_since_change > Duration::milliseconds(50) {
                context.execute_background(Task::DynamicBPMDetectionParameters(UpdateOrigin::Daw));
                self.dynamic_bpm_detection_parameters_changed_at.store(None, Ordering::Relaxed);
            }
        }
        self.receive_notes(context);
        self.current_sample.fetch_add(buffer.samples(), Ordering::Relaxed);
        if self.params.editor_state.is_open() { ProcessStatus::KeepAlive } else { ProcessStatus::Normal }
    }
}

impl MidiBpmDetector {
    fn receive_notes<P>(&mut self, context: &mut P) -> bool
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
            let Ok(midi_note_on) = MidiNoteOn::try_from(midi_message.to_owned()) else {
                continue;
            };

            let note_sample = current_sample + event.timing() as usize;
            let timestamp = sample_to_duration(self.sample_rate, note_sample);

            if self
                .events_sender
                .try_push(Event::TimedMidiNoteOn(TimedMidiNoteOn { timestamp, midi_message: midi_note_on }))
                .is_err()
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

    #[allow(unused)]
    fn current_time(&self) -> Duration {
        sample_to_duration(self.sample_rate, self.current_sample.load(Ordering::Relaxed))
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
                page.add_param(&self.params.static_params.normal_distribution.imprecision);
                page.add_param(&self.params.static_params.normal_distribution.std_dev);
            });
        });
        context.add_section("Dynamic parameters", |section| {
            section.add_page("Dynamic parameters", |page| {
                page.add_param(&self.params.dynamic_params.beats_lookback);
                page.add_param(&self.params.dynamic_params.velocity_current_note_weight);
                page.add_param(&self.params.dynamic_params.velocity_note_from_weight);
                page.add_param(&self.params.dynamic_params.age_weight);
                page.add_param(&self.params.dynamic_params.octave_distance_weight);
                page.add_param(&self.params.dynamic_params.pitch_distance_weight);
                page.add_param(&self.params.dynamic_params.multiplier_weight);
                page.add_param(&self.params.dynamic_params.subdivision_weight);
                page.add_param(&self.params.dynamic_params.normal_distribution_weight);
                page.add_param(&self.params.dynamic_params.high_tempo_bias);
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
