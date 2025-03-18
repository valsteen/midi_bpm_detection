use errors::Result;
use futures::stream::StreamExt;
use log::{error, info};
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook_tokio::Signals;
use tokio::task::JoinHandle;

pub fn spawn_signal_task<F>(callback: F) -> Result<JoinHandle<()>>
where
    F: Fn() -> Result<()> + Send + Sync + 'static,
{
    let mut signals = Signals::new([SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
    Ok(tokio::spawn(async move {
        info!("spawn signal handling task");
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP | SIGTERM | SIGINT | SIGQUIT => {
                    info!("Received signal {}, shutting down", signal);
                    // Shutdown the system;
                    if callback().is_err() {
                        error!("Failed to signal quit event");
                        break;
                    }
                }
                _ => {
                    error!("Received unexpected signal {}", signal);
                }
            }
        }
    }))
}
