use std::{
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use super::*;

#[test]
fn queued_commands_run_after_runtime_starts() {
    let pending_runtime = PendingTargetRuntime::new();
    let command_queue = pending_runtime.command_queue();
    let target = Arc::new(sync::Mutex::new(0_u8));

    command_queue.send("test command", |value| {
        *value = 1;
        Ok(())
    });
    pending_runtime.start(Arc::clone(&target), "test-target-command").expect("runtime should start");

    std::thread::sleep(Duration::from_millis(50));

    assert_eq!(*target.lock(), 1);
}

#[test]
fn queued_commands_reuse_one_worker_thread() {
    let pending_runtime = PendingTargetRuntime::new();
    let command_queue = pending_runtime.command_queue();
    let target = Arc::new(sync::Mutex::new(0_u8));
    let caller_thread = thread::current().id();
    let (sender, receiver) = mpsc::channel();

    pending_runtime.start(Arc::clone(&target), "test-target-command").expect("runtime should start");

    command_queue.send("test command", {
        let sender = sender.clone();
        move |value| {
            *value += 1;
            sender.send(thread::current().id()).expect("receiver should still be waiting for the command result");
            Ok(())
        }
    });
    command_queue.send("test command", move |value| {
        *value = 1;
        sender.send(thread::current().id()).expect("receiver should still be waiting for the command result");
        Ok(())
    });

    let first_thread = receiver.recv_timeout(Duration::from_secs(2)).expect("first command should run");
    let second_thread = receiver.recv_timeout(Duration::from_secs(2)).expect("second command should run");

    assert_ne!(first_thread, caller_thread, "commands should run away from the caller thread");
    assert_eq!(first_thread, second_thread, "commands should reuse the queue worker");
    assert_eq!(*target.lock(), 1);
}

#[test]
fn weak_command_queue_does_not_keep_worker_alive() {
    let pending_runtime = PendingTargetRuntime::<u8>::new();
    let command_queue = pending_runtime.command_queue();
    let weak_command_queue = command_queue.downgrade();

    drop(command_queue);
    drop(pending_runtime);

    assert!(weak_command_queue.upgrade().is_none());
}

#[test]
fn weak_command_queue_can_upgrade_while_owner_is_alive() {
    let pending_runtime = PendingTargetRuntime::<u8>::new();
    let command_queue = pending_runtime.command_queue();
    let weak_command_queue = command_queue.downgrade();

    assert!(weak_command_queue.upgrade().is_some());

    drop(command_queue);
    drop(pending_runtime);
}
