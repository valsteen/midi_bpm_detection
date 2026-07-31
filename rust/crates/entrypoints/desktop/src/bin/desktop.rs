use std::sync::Arc;

use desktop::{
    app::DesktopApp,
    config::DesktopConfig,
    controller::DesktopController,
    controller_runtime::{DesktopControllerCommandQueue, PendingDesktopControllerRuntime, SharedDesktopController},
};
use errors::{LogErrorWithExt, MakeReportExt, Result, initialize_logging, initialize_panic_handler};
use gui::{
    BpmDisplayPublisher, GuiContextHandle, GuiLifecycleOwner, create_gui,
    eframe::{self, egui},
};
use mimalloc::MiMalloc;
use sync::Mutex;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    initialize_logging()?;
    initialize_panic_handler(|| {})?;

    let config = DesktopConfig::new()?;
    let pending_controller_runtime = PendingDesktopControllerRuntime::new();
    let controller_commands = pending_controller_runtime.command_queue();
    let (publisher, context, gui) = create_gui();

    let controller = start_desktop_controller(
        &config,
        publisher,
        #[cfg(target_os = "macos")]
        &context,
        #[cfg(target_os = "macos")]
        &controller_commands,
    )?;
    pending_controller_runtime.start(controller.clone())?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
        run_and_return: true,
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "Estimated BPM",
        options,
        Box::new(move |cc| {
            let mut gui = gui;
            gui.attach_context(&cc.egui_ctx, GuiLifecycleOwner::ApplicationRuntime);
            Ok(Box::new(DesktopApp::new(config, gui, context, controller, controller_commands)))
        }),
    )
    .report_msg("Could not display eframe")?;
    Ok(())
}

fn start_desktop_controller(
    config: &DesktopConfig,
    publisher: BpmDisplayPublisher,
    #[cfg(target_os = "macos")] context: &GuiContextHandle,
    #[cfg(target_os = "macos")] controller_commands: &DesktopControllerCommandQueue<BpmDisplayPublisher>,
) -> Result<SharedDesktopController<BpmDisplayPublisher>> {
    let midi_service = bpm_detection_midi::MidiService::new(
        config.midi.clone(),
        config.bpm_detection.static_bpm_detection_config.clone(),
        config.bpm_detection.dynamic_bpm_detection_config.clone(),
        #[cfg(target_os = "macos")]
        notify_device_change(context.clone(), controller_commands),
        publisher,
    )?;
    let mut desktop_controller = DesktopController::new(midi_service);
    desktop_controller.refresh_devices().log_error_msg("Could not refresh MIDI input list on startup").ok();
    Ok(Arc::new(Mutex::new(desktop_controller)))
}

#[cfg(target_os = "macos")]
fn notify_device_change(
    context: GuiContextHandle,
    controller_commands: &DesktopControllerCommandQueue<BpmDisplayPublisher>,
) -> impl Fn() + Send + 'static {
    let commands = controller_commands.downgrade();
    move || {
        if let Some(commands) = commands.upgrade() {
            commands.refresh_devices(context.clone());
        }
    }
}
