use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use log::{debug, info};
use std::sync::mpsc::SyncSender;

use errors::{Result, error_backtrace};
use gui::GuiRemote;
use ratatui::prelude::Rect;

use errors::LogErrorWithExt;
use tokio::sync::{
    mpsc,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{
    components::{ComponentNewBox, midi_display::MidiDisplay, select_device::SelectDevice},
    services::{midi::MidiService, screens::Screens},
    tui::Event,
};

use crate::{
    action::Action,
    config::Config,
    lifecycle::signals::spawn_signal_task,
    mode::Mode,
    tui,
    utils::dispatch::{ActionHandler, EventHandler, try_dispatch_concurrently},
};

#[allow(forbidden_lint_groups)]
#[allow(clippy::too_many_lines)]
pub async fn run_tui(
    start_gui: SyncSender<()>,
    action_tx: UnboundedSender<Action>,
    mut action_rx: UnboundedReceiver<Action>,
    config: Config,
    mut gui_exit_receiver: UnboundedReceiver<()>,
    gui_remote: GuiRemote,
) -> Result<()> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    let signal_task = spawn_signal_task({
        let action_tx = action_tx.clone();
        move || Ok(action_tx.send(Action::Quit)?)
    })?;

    let gui_close_task = tokio::spawn({
        let action_tx = action_tx.clone();
        async move {
            gui_exit_receiver.recv().await;
            action_tx.send(Action::Quit).ok();
        }
    });

    gui_remote.receive_keystrokes({
        let event_tx = event_tx.clone();
        Box::new(move |key| {
            info!("{key}");
            let key = if let Ok(keycode) = serde_json::from_value(serde_json::Value::String(key.to_string())) {
                Event::Key(KeyEvent {
                    code: keycode,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    state: KeyEventState::NONE,
                })
            } else if key.len() == 1 {
                let Some(char) = key.to_lowercase().chars().next() else {
                    return;
                };

                Event::Key(KeyEvent {
                    code: KeyCode::Char(char),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    state: KeyEventState::NONE,
                })
            } else {
                match key {
                    "Escape" => Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    }),
                    "Space" => Event::Key(KeyEvent {
                        code: KeyCode::Char(' '),
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    }),
                    _ => return,
                }
            };
            event_tx.send(key).ok();
        })
    });

    let mut components = [SelectDevice::box_new(), MidiDisplay::box_new()];
    for component in &mut components {
        component.register_config_handler(config.clone())?;
    }

    let mut services = [
        MidiService::box_new(
            &config.midi,
            config.static_bpm_detection_parameters.clone(),
            config.dynamic_bpm_detection_parameters.clone(),
            event_tx.clone(),
            gui_remote.clone(),
        )
        .await?,
        Box::<Screens>::default(),
    ];
    let mut should_quit = false;
    let mut should_suspend = false;
    let mut mode = Mode::DeviceView;
    action_tx.send(Action::Switch(mode))?;

    let mut last_tick_key_events = Vec::new();

    let mut tui = tui::Tui::new(event_tx.clone())?.tick_rate(config.tick_rate).frame_rate(config.frame_rate);
    tui.enter()?;

    loop {
        if let Some(e) = event_rx.recv().await {
            match e {
                Event::Tick => action_tx.send(Action::Tick)?,
                Event::Render => action_tx.send(Action::Render)?,
                Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                Event::FocusGained => gui_remote.always_on_top(),
                Event::FocusLost => gui_remote.always_on_top_cancel(),
                Event::Key(key) => {
                    for mapping in [config.keybindings.get(&None), config.keybindings.get(&Some(mode))].iter().flatten()
                    {
                        if let Some(action) = mapping.get(&vec![key]) {
                            info!("Got action: {action:?}");
                            action_tx.send(action.clone())?;
                        } else {
                            // If the key was not handled as a single key action,
                            // then consider it for multi-key combinations.
                            last_tick_key_events.push(key);

                            // Check for multi-key combinations
                            if let Some(action) = mapping.get(&last_tick_key_events) {
                                info!("Got action: {action:?}");
                                action_tx.send(action.clone())?;
                            }
                        }
                    }
                }
                Event::Init
                | Event::Error
                | Event::Paste(_)
                | Event::Mouse(_)
                | Event::DeviceChangeDetected
                | Event::DeviceList(_)
                | Event::Midi(_) => (),
            }

            // duplicate because despite having both Service and Component implementing the same EventHandler trait,
            // this is hitting an issue of dyn upcasting coercion https://github.com/rust-lang/rust/issues/65991
            futures_util::future::try_join(
                try_dispatch_concurrently(
                    components.iter_mut().map(Box::as_mut),
                    &e,
                    &action_tx,
                    EventHandler::handle_event,
                ),
                try_dispatch_concurrently(
                    services.iter_mut().map(Box::as_mut),
                    &e,
                    &action_tx,
                    EventHandler::handle_event,
                ),
            )
            .await?;
        }

        while let Ok(action) = action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action {
                Action::Tick => {
                    last_tick_key_events.drain(..);
                }
                Action::Quit => should_quit = true,
                Action::Suspend => should_suspend = true,

                Action::Resize(w, h) => {
                    tui.resize(Rect::new(0, 0, w, h))?;
                    tui.draw(|f| {
                        for component in &mut components {
                            let r = component.draw(f, f.size());
                            if let Err(e) = r {
                                if let Err(e) = action_tx.send(Action::Error(format!("Failed to draw: {e:?}"))) {
                                    error_backtrace!("Error while sending to action_tx {:?}", e);
                                }
                            }
                        }
                    })?;
                }
                Action::Render => {
                    tui.draw(|f| {
                        for component in &mut components {
                            let r = component.draw(f, f.size());
                            if let Err(e) = r {
                                if let Err(e) = action_tx.send(Action::Error(format!("Failed to draw: {e:?}"))) {
                                    error_backtrace!("Error while sending to action_tx {:?}", e);
                                }
                            }
                        }
                    })?;
                }
                Action::Switch(new_mode) => mode = new_mode,
                Action::ShowGUI => start_gui.send(()).log_error_msg("unable to start GUI")?,
                Action::Save => gui_remote.save_config(),
                _ => {}
            }

            // duplicate because despite having both Service and Component implementing the same ActionHandler trait,
            // this is hitting an issue of dyn upcasting coercion https://github.com/rust-lang/rust/issues/65991
            futures_util::future::try_join(
                try_dispatch_concurrently(
                    components.iter_mut().map(Box::as_mut),
                    &action,
                    &action_tx,
                    ActionHandler::handle_action,
                ),
                try_dispatch_concurrently(
                    services.iter_mut().map(Box::as_mut),
                    &action,
                    &action_tx,
                    ActionHandler::handle_action,
                ),
            )
            .await?;
        }

        if should_suspend {
            tui.suspend().await?;
            tui = tui::Tui::new(event_tx.clone())?.tick_rate(config.tick_rate).frame_rate(config.frame_rate);
            tui.enter()?;
            should_suspend = false;
        } else if should_quit {
            tui.stop().await;
            break;
        }
    }
    tui.exit().await?;

    signal_task.abort();
    gui_close_task.abort();
    Ok(())
}
