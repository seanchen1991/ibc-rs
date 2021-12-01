/*!
   Extension to the [`Supervisor`] data type.
*/

// use crossbeam_channel::{unbounded, Sender};
// use ibc_relayer::chain::handle::ChainHandle;
// use ibc_relayer::config::SharedConfig;
// use ibc_relayer::registry::SharedRegistry;
// use ibc_relayer::supervisor::cmd::SupervisorCmd;
// use ibc_relayer::supervisor::spawn_supervisor_tasks;
// use ibc_relayer::util::task::TaskHandle;

// use crate::error::Error;

// /**
//     A wrapper around the SupervisorCmd sender so that we can
//     send stop signal to the supervisor before stopping the
//     chain drivers to prevent the supervisor from raising
//     errors caused by closed connections.
// */
// pub struct SupervisorHandle {
//     pub sender: Sender<SupervisorCmd>,
//     tasks: Vec<TaskHandle>,
// }

// /**
//    Spawn a supervisor for testing purpose using the provided
//    [`SharedConfig`] and [`SharedRegistry`]. Returns a
//    [`SupervisorHandle`] that stops the supervisor when the
//    value is dropped.
// */
// pub fn spawn_supervisor(
//     config: SharedConfig,
//     registry: SharedRegistry<impl ChainHandle + 'static>,
// ) -> Result<SupervisorHandle, Error> {
//     let (sender, receiver) = unbounded();

//     let tasks = spawn_supervisor_tasks(config, registry, None, receiver, false)?;

//     Ok(SupervisorHandle { sender, tasks })
// }

// impl SupervisorHandle {
//     /**
//        Explicitly stop the running supervisor. This is useful in tests where
//        the supervisor has to be stopped and restarted explicitly.

//        Note that after stopping the supervisor, the only way to restart it
//        is by respawning a new supervisor using [`spawn_supervisor`].
//     */
//     pub fn shutdown(self) {
//         for task in self.tasks {
//             task.shutdown_and_wait();
//         }
//     }
// }
