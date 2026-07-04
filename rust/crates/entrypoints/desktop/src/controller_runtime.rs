use std::{
    sync::{Arc, Weak, mpsc},
    thread,
};

use bpm_detection_core::bpm_detection_receiver::BPMDetectionReceiver;
use errors::{LogErrorWithExt, Result};

use crate::controller::DesktopController;

pub type SharedDesktopController<B> = Arc<sync::Mutex<DesktopController<B>>>;

type RuntimeCommand<T> = Box<dyn FnOnce(&mut T) -> Result<()> + Send + 'static>;

struct QueuedRuntimeCommand<T> {
    error_message: &'static str,
    command: RuntimeCommand<T>,
}

struct TargetCommandQueue<T>
where
    T: Send + 'static,
{
    inner: Arc<TargetCommandQueueInner<T>>,
}

struct WeakTargetCommandQueue<T>
where
    T: Send + 'static,
{
    inner: Weak<TargetCommandQueueInner<T>>,
}

struct TargetCommandQueueInner<T>
where
    T: Send + 'static,
{
    sender: mpsc::Sender<QueuedRuntimeCommand<T>>,
}

struct PendingTargetRuntime<T>
where
    T: Send + 'static,
{
    command_queue: TargetCommandQueue<T>,
    command_receiver: mpsc::Receiver<QueuedRuntimeCommand<T>>,
}

impl<T> PendingTargetRuntime<T>
where
    T: Send + 'static,
{
    fn new() -> Self {
        let (sender, command_receiver) = mpsc::channel();
        Self {
            command_queue: TargetCommandQueue { inner: Arc::new(TargetCommandQueueInner { sender }) },
            command_receiver,
        }
    }

    fn command_queue(&self) -> TargetCommandQueue<T> {
        self.command_queue.clone()
    }

    fn start(self, target: Arc<sync::Mutex<T>>, thread_name: &'static str) -> Result<()> {
        thread::Builder::new().name(thread_name.to_string()).spawn(move || {
            while let Ok(command) = self.command_receiver.recv() {
                let mut target = target.lock();
                (command.command)(&mut target).log_error_msg(command.error_message).ok();
            }
        })?;

        Ok(())
    }
}

impl<T> Clone for TargetCommandQueue<T>
where
    T: Send + 'static,
{
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T> TargetCommandQueue<T>
where
    T: Send + 'static,
{
    fn send(&self, error_message: &'static str, command: impl FnOnce(&mut T) -> Result<()> + Send + 'static) {
        self.inner
            .sender
            .send(QueuedRuntimeCommand { error_message, command: Box::new(command) })
            .log_error_msg("Could not queue desktop controller command")
            .ok();
    }

    fn downgrade(&self) -> WeakTargetCommandQueue<T> {
        WeakTargetCommandQueue { inner: Arc::downgrade(&self.inner) }
    }
}

impl<T> WeakTargetCommandQueue<T>
where
    T: Send + 'static,
{
    fn upgrade(&self) -> Option<TargetCommandQueue<T>> {
        self.inner.upgrade().map(|inner| TargetCommandQueue { inner })
    }
}

pub struct PendingDesktopControllerRuntime<B>
where
    B: BPMDetectionReceiver,
{
    pending_runtime: PendingTargetRuntime<DesktopController<B>>,
}

pub struct DesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    queue: TargetCommandQueue<DesktopController<B>>,
}

/// Non-owning command queue reference used by callbacks stored inside the desktop controller.
///
/// The strong `DesktopControllerCommandQueue` is owned by desktop bootstrap/GUI state and controls worker lifetime.
/// Callbacks captured by `DesktopController` must not keep that worker alive, otherwise the controller, callback, and
/// command queue can form a reference cycle. When this weak handle no longer upgrades, shutdown has started and the
/// callback should simply stop enqueueing work.
pub struct WeakDesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    queue: WeakTargetCommandQueue<DesktopController<B>>,
}

impl<B> PendingDesktopControllerRuntime<B>
where
    B: BPMDetectionReceiver,
{
    /// Create the command sender before the desktop controller exists.
    ///
    /// This is intentionally a pending runtime, not an optional controller holder. Native MIDI setup may need callbacks
    /// while the controller is still being constructed, especially on macOS where hotplug notification must be
    /// registered before other MIDI initialization. Those callbacks can enqueue commands immediately; commands run only
    /// after `start` receives the fully constructed controller.
    #[must_use]
    pub fn new() -> Self {
        Self { pending_runtime: PendingTargetRuntime::new() }
    }

    #[must_use]
    pub fn command_queue(&self) -> DesktopControllerCommandQueue<B> {
        DesktopControllerCommandQueue { queue: self.pending_runtime.command_queue() }
    }

    /// Start the single desktop controller command worker once the controller exists.
    ///
    /// Commands sent before startup are buffered by the channel and run after this method starts the worker. This keeps
    /// hotplug callbacks safe during native MIDI initialization without exposing an unset controller state to callers.
    ///
    /// # Errors
    ///
    /// Returns an error if the command worker thread cannot be started.
    pub fn start(self, controller: SharedDesktopController<B>) -> Result<()> {
        self.pending_runtime.start(controller, "desktop-controller-command")
    }
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
    pub fn send(
        &self,
        error_message: &'static str,
        command: impl FnOnce(&mut DesktopController<B>) -> Result<()> + Send + 'static,
    ) {
        self.queue.send(error_message, command);
    }

    #[must_use]
    pub fn downgrade(&self) -> WeakDesktopControllerCommandQueue<B> {
        // Callbacks stored by `DesktopController` use a weak queue handle so they can request work while the desktop
        // runtime is alive without becoming part of the ownership chain that keeps that runtime alive.
        WeakDesktopControllerCommandQueue { queue: self.queue.downgrade() }
    }
}

impl<B> WeakDesktopControllerCommandQueue<B>
where
    B: BPMDetectionReceiver,
{
    #[must_use]
    pub fn upgrade(&self) -> Option<DesktopControllerCommandQueue<B>> {
        // `None` is a lifecycle signal: the strong queue owner is gone, so callbacks should leave quietly.
        self.queue.upgrade().map(|queue| DesktopControllerCommandQueue { queue })
    }
}

impl<B> Default for PendingDesktopControllerRuntime<B>
where
    B: BPMDetectionReceiver,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../tests/unit/controller_runtime.rs"]
mod tests;
