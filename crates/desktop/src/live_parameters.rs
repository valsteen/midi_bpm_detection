use std::{
    sync::{Arc, Weak, atomic::Ordering, mpsc},
    thread,
    time::Duration,
};

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{
        DynamicBPMDetectionConfig, DynamicBPMDetectionConfigAccessor, NormalDistributionConfigAccessor,
        StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor,
    },
};
use bpm_detection_midi::MidiInputPort;
use errors::{LogErrorWithExt, Result};
use gui::{BPMDetectionConfig, GUIConfigAccessor};
use parameter::OnOff;

use crate::{config::DesktopConfig, controller::DesktopController};

pub type StaticConfigCallback = Arc<dyn Fn(StaticBPMDetectionConfig) + Send + Sync>;
pub type DynamicConfigCallback = Arc<dyn Fn(DynamicBPMDetectionConfig) + Send + Sync>;
pub type DesktopControllerSlot<B> = Arc<sync::Mutex<Option<DesktopController<B>>>>;

type SlotCommand<T> = Box<dyn FnOnce(&mut T) -> Result<()> + Send + 'static>;

struct QueuedSlotCommand<T> {
    error_message: &'static str,
    command: SlotCommand<T>,
}

struct SlotCommandQueue<T>
where
    T: Send + 'static,
{
    inner: Arc<SlotCommandQueueInner<T>>,
}

struct WeakSlotCommandQueue<T>
where
    T: Send + 'static,
{
    inner: Weak<SlotCommandQueueInner<T>>,
}

struct SlotCommandQueueInner<T>
where
    T: Send + 'static,
{
    sender: mpsc::Sender<QueuedSlotCommand<T>>,
}

impl<T> Clone for SlotCommandQueue<T>
where
    T: Send + 'static,
{
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T> SlotCommandQueue<T>
where
    T: Send + 'static,
{
    fn new(slot: Arc<sync::Mutex<Option<T>>>, thread_name: &'static str) -> Result<Self> {
        let (sender, receiver) = mpsc::channel::<QueuedSlotCommand<T>>();

        thread::Builder::new().name(thread_name.to_string()).spawn(move || {
            while let Ok(command) = receiver.recv() {
                let mut slot = slot.lock();
                let Some(value) = slot.as_mut() else {
                    continue;
                };

                (command.command)(value).log_error_msg(command.error_message).ok();
            }
        })?;

        Ok(Self { inner: Arc::new(SlotCommandQueueInner { sender }) })
    }

    fn send(&self, error_message: &'static str, command: impl FnOnce(&mut T) -> Result<()> + Send + 'static) {
        self.inner
            .sender
            .send(QueuedSlotCommand { error_message, command: Box::new(command) })
            .log_error_msg("Could not queue desktop controller command")
            .ok();
    }

    fn downgrade(&self) -> WeakSlotCommandQueue<T> {
        WeakSlotCommandQueue { inner: Arc::downgrade(&self.inner) }
    }
}

impl<T> WeakSlotCommandQueue<T>
where
    T: Send + 'static,
{
    fn upgrade(&self) -> Option<SlotCommandQueue<T>> {
        self.inner.upgrade().map(|inner| SlotCommandQueue { inner })
    }
}

pub struct DesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    queue: SlotCommandQueue<DesktopController<B>>,
}

pub struct WeakDesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    queue: WeakSlotCommandQueue<DesktopController<B>>,
}

impl<B> Clone for DesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    fn clone(&self) -> Self {
        Self { queue: self.queue.clone() }
    }
}

impl<B> DesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    /// Start the single desktop controller command worker.
    ///
    /// Frame-driven UI code sends commands here instead of spawning per-command threads. MIDI operations may block
    /// while opening/listing devices, so the worker owns that cost away from egui's frame loop.
    ///
    /// # Errors
    ///
    /// Returns an error if the command worker thread cannot be started.
    pub fn new(controller: DesktopControllerSlot<B>) -> Result<Self> {
        Ok(Self { queue: SlotCommandQueue::new(controller, "desktop-controller-command")? })
    }

    pub fn send(
        &self,
        error_message: &'static str,
        command: impl FnOnce(&mut DesktopController<B>) -> Result<()> + Send + 'static,
    ) {
        self.queue.send(error_message, command);
    }

    #[must_use]
    pub fn downgrade(&self) -> WeakDesktopControllerCommandQueue<B> {
        WeakDesktopControllerCommandQueue { queue: self.queue.downgrade() }
    }
}

impl<B> WeakDesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    #[must_use]
    pub fn upgrade(&self) -> Option<DesktopControllerCommandQueue<B>> {
        self.queue.upgrade().map(|queue| DesktopControllerCommandQueue { queue })
    }
}

pub struct DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    pub config: DesktopConfig,
    pub controller: DesktopControllerSlot<B>,
    pub controller_commands: DesktopControllerCommandQueue<B>,
    pub on_static_config_changed: StaticConfigCallback,
    pub on_dynamic_config_changed: DynamicConfigCallback,
}

impl<B> DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    pub fn propagate_static_changes(&self) {
        (self.on_static_config_changed)(self.config.static_bpm_detection_config.clone());
    }

    pub fn propagate_dynamic_changes(&self) {
        (self.on_dynamic_config_changed)(self.config.dynamic_bpm_detection_config.clone());
    }
}

impl<B> NormalDistributionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
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

impl<B> DynamicBPMDetectionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
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

impl<B> StaticBPMDetectionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
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

impl<B> GUIConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
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
        let Some(controller_slot) = self.controller.try_lock() else {
            ui.label("MIDI service is updating");
            return;
        };
        let Some(controller) = controller_slot.as_ref() else {
            ui.label("MIDI service is starting");
            return;
        };

        let devices = controller.device_selection().devices().to_vec();
        let selected = controller.device_selection().displayed_selection().cloned();
        let mut selected_index = controller.device_selection().selected_index().unwrap_or_default();
        let current_selected_index = controller.device_selection().selected_index();
        let displayed_selection_is_fallback = controller.device_selection().displayed_selection_is_fallback();
        // Keep the frame path short: egui actions below can enqueue MIDI work, so do not keep the controller slot
        // borrowed across UI callbacks.
        drop(controller_slot);

        let mut selected_index_clicked = false;
        gui::eframe::egui::ComboBox::from_label("MIDI input")
            .selected_text(selected.as_ref().map_or("<none selected>", MidiInputPort::as_str))
            .show_ui(ui, |ui| {
                for (index, device) in devices.iter().enumerate() {
                    selected_index_clicked |=
                        ui.selectable_value(&mut selected_index, index, device.as_str()).clicked();
                }
            });

        let selected_index_changed = Some(selected_index) != current_selected_index;
        let confirmed_displayed_fallback =
            selected_index_clicked && displayed_selection_is_fallback && Some(selected_index) == current_selected_index;
        if !devices.is_empty() && (selected_index_changed || confirmed_displayed_fallback) {
            self.controller_commands
                .send("Could not select MIDI input", move |controller| controller.select_device_index(selected_index));
        }

        if ui.button("Refresh MIDI inputs").clicked() {
            let ctx = ui.ctx().clone();
            self.controller_commands.send("Could not refresh MIDI input list", move |controller| {
                let result = controller.refresh_devices();
                ctx.request_repaint();
                result
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex as StdMutex, mpsc},
        thread,
        time::Duration,
    };

    use bpm_detection_core::{
        bpm_detection_receiver::BPMDetectionReceiver,
        parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor},
    };
    use bpm_detection_midi::MidiServiceConfig;
    use gui::{GUIConfig, GUIConfigAccessor};
    use sync::ArcAtomicBool;

    use super::*;
    use crate::config::{AppConfig, DesktopConfig};

    #[derive(Clone)]
    struct TestReceiver;

    impl BPMDetectionReceiver for TestReceiver {
        fn receive_bpm_histogram_data(&mut self, _histogram_data_points: &[f32], _detected_bpm: f32) {}

        fn receive_daw_bpm(&self, _bpm: f32) {}
    }

    fn desktop_config() -> DesktopConfig {
        DesktopConfig {
            app: AppConfig::default(),
            gui_config: GUIConfig::default(),
            midi: MidiServiceConfig {
                device_name: "Desktop".to_string(),
                send_tempo: ArcAtomicBool::new(false),
                enable_midi_clock: ArcAtomicBool::new(false),
            },
            static_bpm_detection_config: StaticBPMDetectionConfig::default(),
            dynamic_bpm_detection_config: DynamicBPMDetectionConfig::default(),
        }
    }

    fn base_config(
        static_changes: Arc<StdMutex<Vec<StaticBPMDetectionConfig>>>,
        dynamic_changes: Arc<StdMutex<Vec<DynamicBPMDetectionConfig>>>,
    ) -> DesktopBaseConfig<TestReceiver> {
        let controller = Arc::new(sync::Mutex::new(None));
        let controller_commands =
            DesktopControllerCommandQueue::new(Arc::clone(&controller)).expect("controller command queue should start");

        DesktopBaseConfig {
            config: desktop_config(),
            controller,
            controller_commands,
            on_static_config_changed: Arc::new(move |config| {
                static_changes.lock().expect("static changes lock should not be poisoned").push(config);
            }),
            on_dynamic_config_changed: Arc::new(move |config| {
                dynamic_changes.lock().expect("dynamic changes lock should not be poisoned").push(config);
            }),
        }
    }

    #[test]
    fn static_parameter_setter_propagates_static_config() {
        let static_changes = Arc::new(StdMutex::new(Vec::new()));
        let dynamic_changes = Arc::new(StdMutex::new(Vec::new()));
        let mut config = base_config(Arc::clone(&static_changes), Arc::clone(&dynamic_changes));

        config.set_bpm_center(120.0);

        let static_changes = static_changes.lock().expect("static changes lock should not be poisoned");
        let dynamic_changes = dynamic_changes.lock().expect("dynamic changes lock should not be poisoned");
        assert_eq!(static_changes.len(), 1);
        assert!((static_changes[0].bpm_center - 120.0).abs() < f32::EPSILON);
        assert!(dynamic_changes.is_empty());
    }

    #[test]
    fn gui_parameter_setter_propagates_dynamic_config() {
        let static_changes = Arc::new(StdMutex::new(Vec::new()));
        let dynamic_changes = Arc::new(StdMutex::new(Vec::new()));
        let mut config = base_config(Arc::clone(&static_changes), Arc::clone(&dynamic_changes));

        config.set_interpolation_duration(Duration::from_millis(250));

        let static_changes = static_changes.lock().expect("static changes lock should not be poisoned");
        let dynamic_changes = dynamic_changes.lock().expect("dynamic changes lock should not be poisoned");
        assert!(static_changes.is_empty());
        assert_eq!(dynamic_changes.len(), 1);
        assert_eq!(config.interpolation_duration(), Duration::from_millis(250));
    }

    #[test]
    fn queued_slot_commands_reuse_one_worker_thread() {
        let slot = Arc::new(sync::Mutex::new(Some(0_u8)));
        let caller_thread = thread::current().id();
        let (sender, receiver) = mpsc::channel();
        let command_queue =
            SlotCommandQueue::new(Arc::clone(&slot), "test-slot-command").expect("slot command queue should start");

        command_queue.send("test command", {
            let sender = sender.clone();
            move |value| {
                *value += 1;
                sender.send(thread::current().id()).expect("receiver should still be waiting for the command result");
                Ok(())
            }
        });
        command_queue.send("test command", move |value| {
            *value = 1;
            sender.send(thread::current().id()).expect("receiver should still be waiting for the command result");
            Ok(())
        });

        let first_thread = receiver.recv_timeout(Duration::from_secs(2)).expect("first command should run");
        let second_thread = receiver.recv_timeout(Duration::from_secs(2)).expect("second command should run");

        assert_ne!(first_thread, caller_thread, "commands should run away from the caller thread");
        assert_eq!(first_thread, second_thread, "commands should reuse the queue worker");
        assert_eq!(*slot.lock().as_ref().expect("slot should still hold a value"), 1);
    }

    #[test]
    fn queued_slot_command_skips_missing_controller() {
        let slot = Arc::new(sync::Mutex::new(None::<u8>));
        let (sender, receiver) = mpsc::channel();
        let command_queue = SlotCommandQueue::new(slot, "test-slot-command").expect("slot command queue should start");

        command_queue.send("test command", move |value| {
            *value = 1;
            sender.send(()).expect("receiver should still be available");
            Ok(())
        });

        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
    }

    #[test]
    fn weak_slot_command_queue_does_not_keep_worker_alive() {
        let slot = Arc::new(sync::Mutex::new(Some(0_u8)));
        let command_queue =
            SlotCommandQueue::new(Arc::clone(&slot), "test-slot-command").expect("slot command queue should start");
        let weak_command_queue = command_queue.downgrade();

        drop(command_queue);

        assert!(weak_command_queue.upgrade().is_none());
    }
}
