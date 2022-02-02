use crate::prelude::*;

use crate::applications::ics20_fungible_token_transfer::context::Ics20Context;
use crate::core::ics02_client::context::{ClientKeeper, ClientReader};
use crate::core::ics03_connection::connection::Counterparty;
use crate::core::ics03_connection::context::{ConnectionKeeper, ConnectionReader};
use crate::core::ics04_channel::channel::Order;
use crate::core::ics04_channel::context::{ChannelKeeper, ChannelReader};
use crate::core::ics04_channel::packet::Packet;
use crate::core::ics04_channel::Version;
use crate::core::ics05_port::capabilities::Capability;
use crate::core::ics05_port::context::PortReader;
use crate::core::ics24_host::identifier::{ChannelId, ConnectionId, PortId};
use crate::core::ics26_routing::error::Error;
use crate::signer::Signer;

/// This trait captures all the functional dependencies (i.e., context) which the ICS26 module
/// requires to be able to dispatch and process IBC messages. In other words, this is the
/// representation of a chain from the perspective of the IBC module of that chain.
pub trait Ics26Context:
    ClientReader
    + ClientKeeper
    + ConnectionReader
    + ConnectionKeeper
    + ChannelKeeper
    + ChannelReader
    + PortReader
    + Ics20Context
    + Clone
{
}

pub trait Module {
    #[allow(clippy::too_many_arguments)]
    fn on_chan_open_init(
        &mut self,
        order: Order,
        connection_hops: &[ConnectionId],
        port_id: PortId,
        channel_id: ChannelId,
        channel_cap: Capability,
        counterparty: Counterparty,
        version: Version,
    ) -> Result<(), Error>;

    #[allow(clippy::too_many_arguments)]
    fn on_chan_open_try(
        &mut self,
        order: Order,
        connection_hops: &[ConnectionId],
        port_id: PortId,
        channel_id: ChannelId,
        channel_cap: Capability,
        counterparty: Counterparty,
        counterparty_version: Version,
    ) -> Result<String, Error>;

    fn on_chan_open_ack(
        &mut self,
        port_id: PortId,
        channel_id: ChannelId,
        counterparty_version: Version,
    ) -> Result<(), Error>;

    fn on_chan_open_confirm(&mut self, port_id: PortId, channel_id: ChannelId)
        -> Result<(), Error>;

    fn on_chan_close_init(&mut self, port_id: PortId, channel_id: ChannelId) -> Result<(), Error>;

    fn on_chan_close_confirm(
        &mut self,
        port_id: PortId,
        channel_id: ChannelId,
    ) -> Result<(), Error>;

    fn on_recv_packet(&mut self, packet: Packet, relayer: Signer) -> Result<Vec<u8>, Error>;

    fn on_acknowledgement_packet(
        &mut self,
        packet: Packet,
        acknowledgement: Vec<u8>,
        relayer: Signer,
    ) -> Result<(), Error>;

    fn on_timeout_packet(&mut self, packet: Packet, relayer: Signer) -> Result<(), Error>;
}
