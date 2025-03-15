use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::services::crossterm::reset_crossterm;
use crossterm::{
    cursor,
    event::{
        EnableBracketedPaste, EnableFocusChange, EnableMouseCapture, Event as CrosstermEvent, KeyEvent, KeyEventKind,
        MouseEvent,
    },
    terminal::EnterAlternateScreen,
};

use errors::{error_backtrace, Report, Result};
use futures::{pin_mut, FutureExt, StreamExt};
use futures_util::future::select;
use log::{error, info};
use ratatui::backend::CrosstermBackend as Backend;

use midi::midi_messages::TimedMidiMessage;

use instant::Instant;
use midi::MidiInputPort;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;

pub type IO = std::io::Stderr;
#[must_use]
pub fn io() -> IO {
    std::io::stderr()
}
pub type Frame<'a> = ratatui::Frame<'a>;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Init,
    Error,
    Tick,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    DeviceChangeDetected,
    DeviceList(Vec<MidiInputPort>),
    Midi(TimedMidiMessage),
}

pub struct Tui {
    pub terminal: ratatui::Terminal<Backend<IO>>,
    pub task: JoinHandle<Result<()>>,
    pub cancellation_token: CancellationToken,
    pub event_tx: UnboundedSender<Event>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    pub paste: bool,
}

async fn create_interval_loop(event_tx: UnboundedSender<Event>, interval: Duration, event: Event) {
    let mut interval = tokio::time::interval(interval);
    loop {
        interval.tick().await;
        if let Err(_e) = event_tx.send(event.clone()) {
            break;
        }
    }
}

impl Tui {
    pub fn new(event_tx: UnboundedSender<Event>) -> Result<Self> {
        let tick_rate = 4.0;
        let frame_rate = 60.0;
        let terminal = ratatui::Terminal::new(Backend::new(io()))?;
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async { Ok(()) });
        let paste = false;

        Ok(Self { terminal, task, cancellation_token, event_tx, frame_rate, tick_rate, paste })
    }

    #[must_use]
    pub fn tick_rate(mut self, tick_rate: f64) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    #[must_use]
    pub fn frame_rate(mut self, frame_rate: f64) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    #[must_use]
    pub fn paste(mut self, paste: bool) -> Self {
        self.paste = paste;
        self
    }

    pub fn start(&mut self) {
        let tick_delay = Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = Duration::from_secs_f64(1.0 / self.frame_rate);
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let cancellation_token = self.cancellation_token.clone();
        let event_tx = self.event_tx.clone();

        self.task = tokio::spawn(async move {
            let reader = crossterm::event::EventStream::new();
            event_tx.send(Event::Init)?;

            let tick_loop = create_interval_loop(event_tx.clone(), tick_delay, Event::Tick);
            let render_loop = create_interval_loop(event_tx.clone(), render_delay, Event::Render);

            let crossterm_loop = reader.all(|e| async {
                if let Err(e) = Self::handle_crossterm_event(e, &event_tx) {
                    info!("{e:?}");
                    false
                } else {
                    true
                }
            });

            let cancelled = cancellation_token.cancelled().map(Ok::<_, Report>);
            pin_mut!(tick_loop, render_loop, crossterm_loop, cancelled);

            select(select(crossterm_loop, select(tick_loop, render_loop)), cancelled).await;
            Ok(())
        });
    }

    fn handle_crossterm_event(
        event: std::io::Result<crossterm::event::Event>,
        event_tx: &UnboundedSender<Event>,
    ) -> Result<()> {
        let event = match event {
            Ok(event) => event,
            Err(e) => {
                error_backtrace!("Received crossterm error: {e:?}");
                event_tx.send(Event::Error)?;
                return Ok(());
            }
        };

        match event {
            CrosstermEvent::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    event_tx.send(Event::Key(key))?;
                }
                return Ok(());
            }
            CrosstermEvent::Mouse(mouse) => event_tx.send(Event::Mouse(mouse)),
            CrosstermEvent::Resize(x, y) => event_tx.send(Event::Resize(x, y)),
            CrosstermEvent::FocusLost => event_tx.send(Event::FocusLost),
            CrosstermEvent::FocusGained => event_tx.send(Event::FocusGained),
            CrosstermEvent::Paste(s) => event_tx.send(Event::Paste(s)),
        }?;
        Ok(())
    }

    pub async fn stop(&mut self) {
        self.cancel();

        let now = Instant::now();

        while !self.task.is_finished() {
            sleep(Duration::from_millis(50)).await;

            if now.elapsed() > Duration::from_millis(100) {
                self.task.abort();
            }
            if now.elapsed() > Duration::from_secs(1) {
                error!("Failed to abort task in 1 second for unknown reason");
                break;
            }
        }
    }

    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(io(), EnterAlternateScreen, cursor::Hide, EnableMouseCapture, EnableFocusChange)?;
        if self.paste {
            crossterm::execute!(io(), EnableBracketedPaste)?;
        }
        self.start();
        Ok(())
    }

    pub async fn exit(&mut self) -> Result<()> {
        self.stop().await;
        // TODO put in lifecycle
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            reset_crossterm();
        }
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub async fn suspend(mut self) -> Result<()> {
        self.exit().await?;
        #[cfg(not(windows))]
        signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP)?;
        Ok(())
    }
}

impl Deref for Tui {
    type Target = ratatui::Terminal<Backend<IO>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}
