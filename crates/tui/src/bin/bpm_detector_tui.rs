use std::sync::mpsc::{SyncSender, sync_channel};

use errors::{Result, initialize_logging, initialize_panic_handler};
use gui::{GuiRemote, create_gui, start_gui};
use log::info;
use tokio::sync::{
    mpsc,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tui::{
    action::Action, app::run_tui, cli::update_config, config::Config, live_parameters::LiveParameters,
    services::crossterm::reset_crossterm,
};

async fn tokio_main(
    start_gui: SyncSender<()>,
    action_tx: UnboundedSender<Action>,
    action_rx: UnboundedReceiver<Action>,
    config: Config,
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
    let config = Config::new()?;
    let config = match update_config(config) {
        Ok(args) => args,
        Err(e) => {
            e.print()?;
            return Ok(());
        }
    };

    let (action_tx, action_rx) = mpsc::unbounded_channel();

    let (gui_remote, app_builder) = create_gui(LiveParameters { action_tx: action_tx.clone(), config: config.clone() });

    // "runtime" must not be dropped, so it cannot be inlined with `spawn` here. Otherwise, the executor will
    // immediately exit
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let (should_start_gui_sender, should_start_gui_receiver) = sync_channel(0);
    runtime.spawn(tokio_main(should_start_gui_sender, action_tx.clone(), action_rx, config.clone(), gui_remote));

    if should_start_gui_receiver.recv().is_ok() {
        start_gui(app_builder)?;
    }
    Ok(())

    // Don't add anything here. Due to macOS application lifecycle, when the main window exits, the process exits,
    // the rest of `main` is not executed.
}
