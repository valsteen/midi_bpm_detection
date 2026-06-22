use std::sync::Arc;

use desktop::{
    config::DesktopConfig,
    controller::DesktopController,
    controller_runtime::{DesktopControllerCommandQueue, PendingDesktopControllerRuntime, SharedDesktopController},
    live_parameters::DesktopBaseConfig,
};
use errors::{LogErrorWithExt, Result, initialize_logging, initialize_panic_handler};
use gui::{create_gui_shell, start_gui};
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
    let (gui_remote, app_builder_shell) = create_gui_shell();

    let controller = start_desktop_controller(&config, gui_remote.clone(), &controller_commands)?;
    pending_controller_runtime.start(controller.clone())?;

    let app_builder = app_builder_shell.with_config(build_gui_config(config, controller, controller_commands));
    start_gui(app_builder)
}

fn start_desktop_controller(
    config: &DesktopConfig,
    gui_remote: gui::GuiRemote,
    controller_commands: &DesktopControllerCommandQueue<gui::GuiRemote>,
) -> Result<SharedDesktopController<gui::GuiRemote>> {
    #[cfg(not(target_os = "macos"))]
    let _ = controller_commands;

    let midi_service = bpm_detection_midi::MidiService::new(
        config.midi.clone(),
        config.static_bpm_detection_config.clone(),
        config.dynamic_bpm_detection_config.clone(),
        #[cfg(target_os = "macos")]
        notify_device_change(gui_remote.clone(), controller_commands),
        gui_remote,
    )?;
    let mut desktop_controller = DesktopController::new(midi_service);

    desktop_controller.refresh_devices().log_error_msg("Could not refresh MIDI input list on startup").ok();
    Ok(Arc::new(Mutex::new(desktop_controller)))
}

#[cfg(target_os = "macos")]
fn notify_device_change(
    gui_remote: gui::GuiRemote,
    controller_commands: &DesktopControllerCommandQueue<gui::GuiRemote>,
) -> impl Fn() + Send + 'static {
    let device_change_controller_commands = controller_commands.downgrade();
    move || {
        let Some(controller_commands) = device_change_controller_commands.upgrade() else {
            return;
        };
        let gui_remote = gui_remote.clone();

        controller_commands.send("Could not refresh MIDI input list after device change", move |controller| {
            let result = controller.refresh_devices();
            gui_remote.request_repaint();
            result
        });
    }
}

fn build_gui_config(
    config: DesktopConfig,
    controller: SharedDesktopController<gui::GuiRemote>,
    controller_commands: DesktopControllerCommandQueue<gui::GuiRemote>,
) -> DesktopBaseConfig<gui::GuiRemote> {
    let static_controller_commands = controller_commands.clone();
    let dynamic_controller_commands = controller_commands.clone();

    DesktopBaseConfig::new(
        config,
        controller,
        controller_commands,
        Arc::new(move |static_config| {
            static_controller_commands.send("Could not apply static BPM detection config", move |controller| {
                controller.apply_static_config(static_config)
            });
        }),
        Arc::new(move |dynamic_config| {
            dynamic_controller_commands.send("Could not apply dynamic BPM detection config", move |controller| {
                controller.apply_dynamic_config(dynamic_config)
            });
        }),
    )
}
