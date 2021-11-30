use core::fmt::Display;
use core::time::Duration;
use crossbeam_channel::{bounded, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use tracing::{error, info, warn};

pub struct TaskHandle {
    shutdown_sender: ShutdownHandle,
    join_handle: thread::JoinHandle<()>,
    stopped: Arc<RwLock<bool>>,
}

struct ShutdownHandle(Sender<()>);

impl TaskHandle {
    pub fn join(self) {
        let _ = self.join_handle.join();
    }

    pub fn shutdown(self) {
        let _ = self.shutdown_sender.0.send(());
    }

    pub fn shutdown_and_wait(self) {
        let _ = self.shutdown_sender.0.send(());
        let _ = self.join_handle.join();
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
        shutdown_sender: ShutdownHandle(shutdown_sender),
        join_handle,
        stopped,
    }
}

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
