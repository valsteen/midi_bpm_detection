use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

use errors::{Result, initialize_logging, initialize_panic_handler};
use gui::{AppBuilder, GuiRemote, create_gui, start_gui};
use log::info;
use mimalloc::MiMalloc;
use tokio::{
    runtime::Runtime,
    sync::{
        mpsc,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
};
use tui::{
    action::Action, app::run_tui, cli::update_config, config::TUIConfig, live_parameters::BaseConfig,
    services::crossterm::reset_crossterm,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

struct PreparedTui {
    action_tx: UnboundedSender<Action>,
    action_rx: UnboundedReceiver<Action>,
    config: TUIConfig,
    gui_remote: GuiRemote,
    app_builder: AppBuilder<BaseConfig>,
}

impl PreparedTui {
    fn new(config: TUIConfig) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (gui_remote, app_builder) = create_gui(BaseConfig { action_tx: action_tx.clone(), config: config.clone() });

        Self { action_tx, action_rx, config, gui_remote, app_builder }
    }

    fn spawn_tui(self) -> Result<RunningTui> {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let (should_start_gui_sender, should_start_gui_receiver) = sync_channel(0);

        runtime.spawn(tokio_main(
            should_start_gui_sender,
            self.action_tx.clone(),
            self.action_rx,
            self.config,
            self.gui_remote,
        ));

        Ok(RunningTui { _runtime: runtime, should_start_gui_receiver, app_builder: self.app_builder })
    }
}

struct RunningTui {
    // Keep the runtime alive while the GUI owns the main thread.
    _runtime: Runtime,
    should_start_gui_receiver: Receiver<()>,
    app_builder: AppBuilder<BaseConfig>,
}

impl RunningTui {
    fn start_gui_when_requested(self) -> Result<()> {
        if self.should_start_gui_receiver.recv().is_ok() {
            start_gui(self.app_builder)?;
        }
        Ok(())

        // Don't add anything here. Due to macOS application lifecycle, when the main window exits, the process exits,
        // the rest of `main` is not executed.
    }
}

async fn tokio_main(
    start_gui: SyncSender<()>,
    action_tx: UnboundedSender<Action>,
    action_rx: UnboundedReceiver<Action>,
    config: TUIConfig,
    gui_remote: GuiRemote,
) -> Result<()> {
    let (gui_exit_sender, gui_exit_receiver) = mpsc::unbounded_channel();
    let (tokio_has_exited_sender, tokio_has_exited_receiver) = sync_channel(0);
    gui_remote.set_on_gui_exit_callback(move || {
        gui_exit_sender.send(()).ok();
        info!("waiting for clean exit");
        tokio_has_exited_receiver.recv().ok(); // this blocks until tokio has exited
    });
    run_tui(start_gui, action_tx, action_rx, config, gui_exit_receiver, gui_remote.clone()).await?;
    tokio_has_exited_sender.try_send(()).ok();
    gui_remote.close();
    // Nothing should be added here : due to macOS application lifecycle, once the GUI exits, which happens when
    // calling gui_remote.close(), the process will exit without going through the rest of `main`.
    Ok(())
}

fn main() -> Result<()> {
    initialize_logging()?;
    initialize_panic_handler(reset_crossterm)?;
    let config = TUIConfig::new()?;
    let config = match update_config(config) {
        Ok(args) => args,
        Err(e) => {
            e.print()?;
            return Ok(());
        }
    };

    PreparedTui::new(config).spawn_tui()?.start_gui_when_requested()
}
