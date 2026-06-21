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
