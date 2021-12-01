use core::fmt::Display;
use core::mem;
use core::time::Duration;
use crossbeam_channel::{bounded, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use tracing::{error, info, warn};

pub struct TaskHandle {
    task_name: String,
    shutdown_sender: Sender<()>,
    stopped: Arc<RwLock<bool>>,
    _join_handle: JoinHandle,
}

struct JoinHandle(Option<thread::JoinHandle<()>>);

impl TaskHandle {
    pub fn join(self) {}

    pub fn shutdown(&self) {
        let _ = self.shutdown_sender.send(());
    }

    pub fn shutdown_and_wait(self) {
        let _ = self.shutdown_sender.send(());
    }

    pub fn is_stopped(&self) -> bool {
        *self.stopped.read().unwrap()
    }
}

pub enum TaskError<E> {
    Abort,
    Ignore(E),
    Fatal(E),
}

pub fn spawn_background_task<E: Display>(
    task_name: String,
    interval_pause: Option<Duration>,
    mut step_runner: impl FnMut() -> Result<(), TaskError<E>> + Send + Sync + 'static,
) -> TaskHandle {
    let stopped = Arc::new(RwLock::new(false));
    let write_stopped = stopped.clone();

    let (shutdown_sender, receiver) = bounded(1);

    let task_name_2 = task_name.clone();

    let join_handle = thread::spawn(move || {
        loop {
            match receiver.try_recv() {
                Ok(()) => {
                    info!(
                        "received shutdown signal, shuttting down task {}",
                        task_name
                    );
                    break;
                }
                _ => match step_runner() {
                    Ok(()) => {}
                    Err(TaskError::Abort) => {
                        info!("task is aborting: {}", task_name);
                        break;
                    }
                    Err(TaskError::Ignore(e)) => {
                        warn!("task {} encountered ignorable error: {}", task_name, e);
                    }
                    Err(TaskError::Fatal(e)) => {
                        error!(
                            "aborting task {} after encountering fatal error: {}",
                            task_name, e
                        );
                        break;
                    }
                },
            }
            if let Some(interval) = interval_pause {
                thread::sleep(interval);
            }
        }

        *write_stopped.write().unwrap() = true;
    });

    TaskHandle {
        task_name: task_name_2,
        shutdown_sender,
        stopped,
        _join_handle: JoinHandle(Some(join_handle)),
    }
}

impl Drop for JoinHandle {
    fn drop(&mut self) {
        if let Some(handle) = mem::take(&mut self.0) {
            let _ = handle.join();
        }
    }
}

impl Drop for TaskHandle {
    fn drop(&mut self) {
        info!(
            "task {} is being dropped, waiting for it to shutdown",
            self.task_name,
        );
        let _ = self.shutdown_sender.send(());
    }
}
