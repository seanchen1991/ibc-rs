use core::time::Duration;
use std::{thread, time::Instant};

use crossbeam_channel::{Receiver, TryRecvError};
use tracing::{debug, info, trace, warn};

use ibc::{core::ics02_client::events::UpdateClient, events::IbcEvent};

use crate::util::task::{spawn_background_task, TaskError, TaskHandle};
use crate::{
    chain::handle::{ChainHandle, ChainHandlePair},
    config::Clients as ClientsConfig,
    foreign_client::{
        ForeignClient, ForeignClientError, ForeignClientErrorDetail, MisbehaviourResults,
    },
    object::Client,
    telemetry,
};

use super::error::RunError;
use super::WorkerCmd;

pub struct ClientWorker<ChainA: ChainHandle, ChainB: ChainHandle> {
    client: Client,
    chains: ChainHandlePair<ChainA, ChainB>,
    cmd_rx: Receiver<WorkerCmd>,
    clients_cfg: ClientsConfig,
}

pub fn spawn_refresh_client<ChainA: ChainHandle + 'static, ChainB: ChainHandle + 'static>(
    mut client: ForeignClient<ChainA, ChainB>,
) -> TaskHandle {
    spawn_background_task(
        "refresh_client".to_string(),
        Some(Duration::from_secs(1)),
        move || -> Result<(), TaskError<ForeignClientError>> {
            let res = client.refresh().map_err(|e| {
                if let ForeignClientErrorDetail::ExpiredOrFrozen(_) = e.detail() {
                    warn!("failed to refresh client '{}': {}", client, e);
                }
                TaskError::Ignore(e)
            })?;

            if res.is_some() {
                telemetry!(ibc_client_updates, &client.dst_chain.id(), &client.id, 1);
            }

            Ok(())
        },
    )
}

pub fn detect_misbehavior_task<ChainA: ChainHandle + 'static, ChainB: ChainHandle + 'static>(
    receiver: Receiver<WorkerCmd>,
    client: ForeignClient<ChainB, ChainA>,
) -> Option<TaskHandle> {
    match client.detect_misbehaviour_and_submit_evidence(None) {
        MisbehaviourResults::ValidClient => {}
        MisbehaviourResults::VerificationError => {}
        MisbehaviourResults::EvidenceSubmitted(_) => {
            return None;
        }
        MisbehaviourResults::CannotExecute => {
            return None;
        }
    }

    let handle = spawn_background_task(
        "detect_misbehavior".to_string(),
        Some(Duration::from_millis(600)),
        move || -> Result<(), TaskError<TryRecvError>> {
            let cmd = receiver.try_recv().map_err(|e| TaskError::Ignore(e))?;

            match cmd {
                WorkerCmd::IbcEvents { batch } => {
                    trace!("[{}] worker received batch: {:?}", client, batch);

                    for event in batch.events {
                        if let IbcEvent::UpdateClient(update) = event {
                            debug!("[{}] client was updated", client);

                            match client.detect_misbehaviour_and_submit_evidence(Some(update)) {
                                MisbehaviourResults::ValidClient => {}
                                MisbehaviourResults::VerificationError => {}
                                MisbehaviourResults::EvidenceSubmitted(_) => {
                                    // if evidence was submitted successfully then exit
                                    return Err(TaskError::Abort);
                                }
                                MisbehaviourResults::CannotExecute => {
                                    // skip misbehaviour checking if chain does not have support for it (i.e. client
                                    // update event does not include the header)
                                    return Err(TaskError::Abort);
                                }
                            }
                        }
                    }
                }

                WorkerCmd::NewBlock { .. } => {}
                WorkerCmd::ClearPendingPackets => {}

                WorkerCmd::Shutdown => {
                    return Err(TaskError::Abort);
                }
            }

            Ok(())
        },
    );

    Some(handle)
}

fn process_cmd<ChainA: ChainHandle, ChainB: ChainHandle>(
    cmd: WorkerCmd,
    client: &ForeignClient<ChainB, ChainA>,
) -> Next {
    match cmd {
        WorkerCmd::IbcEvents { batch } => {
            trace!("[{}] worker received batch: {:?}", client, batch);

            for event in batch.events {
                if let IbcEvent::UpdateClient(update) = event {
                    debug!("[{}] client was updated", client);

                    // Run misbehaviour. If evidence submitted the loop will exit in next
                    // iteration with frozen client
                    if detect_misbehaviour(client, Some(update)) {
                        telemetry!(
                            ibc_client_misbehaviour,
                            &client.dst_chain.id(),
                            &client.id,
                            1
                        );
                    }
                }
            }

            Next::Continue
        }

        WorkerCmd::NewBlock { .. } => Next::Continue,
        WorkerCmd::ClearPendingPackets => Next::Continue,

        WorkerCmd::Shutdown => Next::Abort,
    }
}

fn detect_misbehaviour<ChainA: ChainHandle, ChainB: ChainHandle>(
    client: &ForeignClient<ChainB, ChainA>,
    update: Option<UpdateClient>,
) -> bool {
    match client.detect_misbehaviour_and_submit_evidence(update) {
        MisbehaviourResults::ValidClient => false,
        MisbehaviourResults::VerificationError => {
            // can retry in next call
            false
        }
        MisbehaviourResults::EvidenceSubmitted(_) => {
            // if evidence was submitted successfully then exit
            true
        }
        MisbehaviourResults::CannotExecute => {
            // skip misbehaviour checking if chain does not have support for it (i.e. client
            // update event does not include the header)
            true
        }
    }
}

impl<ChainA: ChainHandle, ChainB: ChainHandle> ClientWorker<ChainA, ChainB> {
    pub fn new(
        client: Client,
        chains: ChainHandlePair<ChainA, ChainB>,
        cmd_rx: Receiver<WorkerCmd>,
        clients_cfg: ClientsConfig,
    ) -> Self {
        Self {
            client,
            chains,
            cmd_rx,
            clients_cfg,
        }
    }

    /// Run the event loop for events associated with a [`Client`].
    pub fn run(self) -> Result<(), RunError> {
        let mut client = ForeignClient::restore(
            self.client.dst_client_id.clone(),
            self.chains.b.clone(),
            self.chains.a.clone(),
        );

        info!(
            "[{}] running client worker with misbehaviour={} and refresh={}",
            client, self.clients_cfg.misbehaviour, self.clients_cfg.refresh
        );

        // initial check for evidence of misbehaviour for all updates
        let skip_misbehaviour =
            !self.clients_cfg.misbehaviour || detect_misbehaviour(&client, None);

        // remember the time of the last refresh so we backoff
        let mut last_refresh = Instant::now() - Duration::from_secs(61);

        loop {
            thread::sleep(Duration::from_millis(600));

            // Clients typically need refresh every 2/3 of their
            // trusting period (which can e.g., two weeks).
            // Backoff refresh checking to attempt it every 1 second.
            if self.clients_cfg.refresh && last_refresh.elapsed() > Duration::from_secs(1) {
                // Run client refresh, exit only if expired or frozen
                match client.refresh() {
                    Ok(Some(_)) => {
                        telemetry!(
                            ibc_client_updates,
                            &self.client.dst_chain_id,
                            &self.client.dst_client_id,
                            1
                        );
                    }
                    Err(e) => {
                        if let ForeignClientErrorDetail::ExpiredOrFrozen(_) = e.detail() {
                            warn!("failed to refresh client '{}': {}", client, e);

                            // This worker has completed its job as the client cannot be refreshed any
                            // further, and can therefore exit without an error.
                            return Ok(());
                        }
                    }
                    _ => (),
                };

                last_refresh = Instant::now();
            }

            if skip_misbehaviour {
                continue;
            }

            if let Ok(cmd) = self.cmd_rx.try_recv() {
                match process_cmd(cmd, &client) {
                    Next::Continue => continue,
                    Next::Abort => break,
                };
            }
        }

        Ok(())
    }

    /// Get a reference to the client worker's chains.
    pub fn chains(&self) -> &ChainHandlePair<ChainA, ChainB> {
        &self.chains
    }

    /// Get a reference to the client worker's object.
    pub fn object(&self) -> &Client {
        &self.client
    }
}

pub enum Next {
    Abort,
    Continue,
}
