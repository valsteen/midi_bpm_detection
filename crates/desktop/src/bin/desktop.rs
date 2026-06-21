use std::sync::Arc;

use desktop::{
    config::DesktopConfig,
    controller::DesktopController,
    live_parameters::{DesktopBaseConfig, DesktopControllerSlot, spawn_controller_command},
};
use errors::{LogErrorWithExt, Result, initialize_logging, initialize_panic_handler};
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
            spawn_controller_command(
                &static_controller,
                "Could not apply static BPM detection config",
                move |controller| controller.apply_static_config(static_config),
            );
        }),
        on_dynamic_config_changed: Arc::new(move |dynamic_config| {
            spawn_controller_command(
                &dynamic_controller,
                "Could not apply dynamic BPM detection config",
                move |controller| controller.apply_dynamic_config(dynamic_config),
            );
        }),
    });

    let device_change_controller = Arc::downgrade(&controller);
    let mut desktop_controller = DesktopController::new(
        config.midi,
        config.static_bpm_detection_config,
        config.dynamic_bpm_detection_config,
        Arc::new({
            let gui_remote = gui_remote.clone();
            move || {
                let Some(controller) = device_change_controller.upgrade() else {
                    return;
                };
                let gui_remote = gui_remote.clone();

                spawn_controller_command(
                    &controller,
                    "Could not refresh MIDI input list after device change",
                    move |controller| {
                        let result = controller.refresh_devices();
                        gui_remote.request_repaint();
                        result
                    },
                );
            }
        }),
        Arc::new(|_| {}),
        gui_remote,
    )?;

    desktop_controller.refresh_devices().log_error_msg("Could not refresh MIDI input list on startup").ok();
    controller.lock().replace(desktop_controller);

    start_gui(app_builder)
}
