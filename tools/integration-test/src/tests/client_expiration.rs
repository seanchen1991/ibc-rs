use core::time::Duration;
use ibc::core::ics24_host::identifier::PortId;
use ibc_relayer::config::Config;
use std::thread::sleep;

use crate::bootstrap::binary::channel::bootstrap_channel_with_chains;
use crate::prelude::*;
use crate::relayer::supervisor::{spawn_supervisor, SupervisorHandle};

const CLIENT_EXPIRY: Duration = Duration::from_secs(20);

#[test]
fn test_client_expiration() -> Result<(), Error> {
    run_binary_chain_test(&ClientExpirationTest)
}

pub struct ClientExpirationTest;

impl TestOverrides for ClientExpirationTest {
    fn modify_relayer_config(&self, config: &mut Config) {
        for mut chain_config in config.chains.iter_mut() {
            chain_config.trusting_period = Some(CLIENT_EXPIRY);
        }
    }

    fn spawn_supervisor(
        &self,
        _config: &SharedConfig,
        _registry: &SharedRegistry<impl ChainHandle + 'static>,
    ) -> Option<SupervisorHandle> {
        None
    }
}

impl BinaryChainTest for ClientExpirationTest {
    fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
        &self,
        _config: &TestConfig,
        chains: ConnectedChains<ChainA, ChainB>,
    ) -> Result<(), Error> {
        let port = PortId::unsafe_new("transfer");

        let _supervisor = spawn_supervisor(chains.config.clone(), chains.registry.clone());

        let sleep_time = CLIENT_EXPIRY + Duration::from_secs(10);

        info!(
            "Sleeping for {} seconds to wait for IBC client to expire",
            sleep_time.as_secs()
        );

        sleep(sleep_time);

        info!("Trying to bootstrap channel after client is expired");
        bootstrap_channel_with_chains(&chains, &port, &port)?;

        crate::suspend();
    }
}
