use core::cell::RefCell;
use core::time::Duration;
use ibc::core::ics24_host::identifier::PortId;
use ibc_relayer::config::Config;
use std::thread::sleep;

use crate::bootstrap::binary::channel::bootstrap_channel_with_chains;
use crate::prelude::*;
use crate::relayer::supervisor::{spawn_supervisor, SupervisorHandle};

#[test]
fn test_client_expiration() -> Result<(), Error> {
    run_binary_chain_test(&ClientExpirationTest {
        supervisor_handle: RefCell::new(None),
    })
}

pub struct ClientExpirationTest {
    supervisor_handle: RefCell<Option<SupervisorHandle>>,
}

impl TestOverrides for ClientExpirationTest {
    fn modify_relayer_config(&self, config: &mut Config) {
        for mut chain_config in config.chains.iter_mut() {
            chain_config.trusting_period = Some(Duration::from_secs(20));
        }
    }

    fn spawn_supervisor(
        &self,
        config: &SharedConfig,
        registry: &SharedRegistry<impl ChainHandle + 'static>,
    ) -> Option<SupervisorHandle> {
        let handle = spawn_supervisor(config.clone(), registry.clone());
        self.supervisor_handle.replace(Some(handle.clone()));
        Some(handle)
    }
}

impl BinaryChainTest for ClientExpirationTest {
    fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
        &self,
        _config: &TestConfig,
        chains: ConnectedChains<ChainA, ChainB>,
    ) -> Result<(), Error> {
        self.supervisor_handle
            .borrow()
            .as_ref()
            .ok_or_else(|| eyre!("expected supervisor handle to be set"))?
            .stop();

        info!("Sleeping for 25 seconds to wait for IBC client to expire");

        sleep(Duration::from_secs(25));

        info!("Trying to bootstrap channel after client is expired");

        let port = PortId::unsafe_new("transfer");
        bootstrap_channel_with_chains(&chains, &port, &port)?;

        crate::suspend();
    }
}
