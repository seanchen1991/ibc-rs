use core::time::Duration;
use crossbeam_channel::Receiver;
use std::sync::Arc;
use tracing::trace;

use crate::util::task::{spawn_background_task, TaskError, TaskHandle};
use crate::{chain::handle::ChainHandle, link::Link};

use super::error::RunError;
use super::WorkerCmd;

pub fn spawn_packet_cmd_worker<ChainA: ChainHandle + 'static, ChainB: ChainHandle + 'static>(
    cmd_rx: Receiver<WorkerCmd>,
    link: Arc<Link<ChainA, ChainB>>,
    clear_on_start: bool,
    clear_interval: u64,
) -> TaskHandle {
    let mut is_first_run: bool = true;
    spawn_background_task("packet_worker".to_string(), None, move || {
        let cmd = cmd_rx
            .recv()
            .map_err(|e| TaskError::Fatal(RunError::recv(e)))?;

        // TODO: add retry
        match cmd {
            WorkerCmd::IbcEvents { batch } => {
                link.a_to_b
                    .update_schedule(batch)
                    .map_err(|e| TaskError::Fatal(RunError::link(e)))?;
            }

            // Handle the arrival of an event signaling that the
            // source chain has advanced to a new block.
            WorkerCmd::NewBlock {
                height,
                new_block: _,
            } => {
                let should_first_clear = is_first_run && clear_on_start;
                is_first_run = false;

                let clear_inverval_enabled = clear_interval != 0;

                let is_at_clear_interval = height.revision_height % height.revision_height == 0;

                let should_clear_packet =
                    should_first_clear || (clear_inverval_enabled && is_at_clear_interval);

                // Schedule the clearing of pending packets. This may happen once at start,
                // and may be _forced_ at predefined block intervals.
                link.a_to_b
                    .schedule_packet_clearing(Some(height), should_clear_packet)
                    .map_err(|e| TaskError::Ignore(RunError::link(e)))?;
            }

            WorkerCmd::ClearPendingPackets => {
                link.a_to_b
                    .schedule_packet_clearing(None, true)
                    .map_err(|e| TaskError::Ignore(RunError::link(e)))?;
            }
        };

        Ok(())
    })
}

pub fn spawn_link_worker<ChainA: ChainHandle + 'static, ChainB: ChainHandle + 'static>(
    link: Arc<Link<ChainA, ChainB>>,
) -> TaskHandle {
    spawn_background_task(
        "link_worker".to_string(),
        Some(Duration::from_millis(500)),
        move || {
            link.a_to_b
                .refresh_schedule()
                .map_err(|e| TaskError::Ignore(RunError::link(e)))?;

            link.a_to_b
                .execute_schedule()
                .map_err(|e| TaskError::Ignore(RunError::link(e)))?;

            let summary = link.a_to_b.process_pending_txs();

            if !summary.is_empty() {
                trace!("Packet worker produced relay summary: {:?}", summary);
            }

            // telemetry!(self.packet_metrics(&summary));

            Ok(())
        },
    )
}

// #[cfg(feature = "telemetry")]
// fn packet_metrics(&self, summary: &RelaySummary) {
//     self.receive_packet_metrics(summary);
//     self.acknowledgment_metrics(summary);
//     self.timeout_metrics(summary);
// }

// #[cfg(feature = "telemetry")]
// fn receive_packet_metrics(&self, summary: &RelaySummary) {
//     use ibc::events::IbcEvent::WriteAcknowledgement;

//     let count = summary
//         .events
//         .iter()
//         .filter(|e| matches!(e, WriteAcknowledgement(_)))
//         .count();

//     telemetry!(
//         ibc_receive_packets,
//         &self.path.src_chain_id,
//         &self.path.src_channel_id,
//         &self.path.src_port_id,
//         count as u64,
//     );
// }

// #[cfg(feature = "telemetry")]
// fn acknowledgment_metrics(&self, summary: &RelaySummary) {
//     use ibc::events::IbcEvent::AcknowledgePacket;

//     let count = summary
//         .events
//         .iter()
//         .filter(|e| matches!(e, AcknowledgePacket(_)))
//         .count();

//     telemetry!(
//         ibc_acknowledgment_packets,
//         &self.path.src_chain_id,
//         &self.path.src_channel_id,
//         &self.path.src_port_id,
//         count as u64,
//     );
// }

// #[cfg(feature = "telemetry")]
// fn timeout_metrics(&self, summary: &RelaySummary) {
//     use ibc::events::IbcEvent::TimeoutPacket;
//     let count = summary
//         .events
//         .iter()
//         .filter(|e| matches!(e, TimeoutPacket(_)))
//         .count();

//     telemetry!(
//         ibc_timeout_packets,
//         &self.path.src_chain_id,
//         &self.path.src_channel_id,
//         &self.path.src_port_id,
//         count as u64,
//     );
// }
