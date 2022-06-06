---
id: h25di
name: ibc-rs Architecture & Folder Structure
file_version: 1.0.2
app_version: 0.8.8-0
---

This document describes the architecture of the ibc-rs repository. If you're looking for a high-level overview of the code base, you've come to the right place!

<br/>

# Terms

<br/>

Some important terms and acronyms that are commonly used include:

*   **IBC**: Refers to the **I**nter**B**lockchain **C**ommunication protocol, a distributed protocol that allows different sovereign blockchains to communicate with one another. The protocol has both on-chain and off-chain components.
    
*   **ICS**: Refers to the **I**nter**C**hain **S**tandards, which are standardization documents that capture the specifications of the IBC protocol across multiple documents.
    
*   **Module**: Refers to a piece of on-chain logic on an IBC-enabled chain.
    
*   **Relayer**: Refers to an off-chain process that is responsible for relaying packets between chains.
    
*   **Hermes**: Refers to the ibc-rs crate's particular relayer implementation.
    

# Bird's Eye View

<br/>

<div align="center"><img src="https://firebasestorage.googleapis.com/v0/b/swimmio-content/o/repositories%2FZ2l0aHViJTNBJTNBaWJjLXJzJTNBJTNBc2VhbmNoZW4xOTkx%2F4ef099e8-b996-4a89-9722-daf6d3de3af3.png?alt=media&token=39868ff1-b8c8-426a-9c7e-c7e02987e981" style="width:'50%'"/></div>

<br/>

At its highest level, ibc-rs implements the InterBlockchain Communication protocol which is captured in [specifications](https://github.com/cosmos/ibc#interchain-standards) in the [cosmos/ibc](https://github.com/cosmos/ibc) repository. ibc-rs exposes modules that implement the specified protocol logic. The IBC protocol can be understood as having two separate components: on-chain and off-chain logic. The relayer, which is the main off-chain component, is a standalone process, of which Hermes is an implementation. On-chain components can be thought of as modules or smart contracts that run as part of a chain. The main on-chain components deal with the abstractions of clients, connections, and channels.

<br/>

# Code Map

<br/>

This section talks briefly about the various directories and modules in ibc-rs.

<br/>

## `📄 modules`

<br/>

This crate contains the main data structures and on-chain logic of the IBC protocol; the fundamental pieces. There is the conceptual notion of 'handlers', which are pieces of code that each handle a particular type of message. The most notable handlers are`📄 modules/src/core/ics02_client/handler` `📄 modules/src/core/ics04_channel/handler`, and `📄 modules/src/core/ics03_connection/handler`.

<br/>

> The naming of directories in this crate follows a slightly different convention compared to the other crates in ibc-rs due to the fact that they adhere to the naming convention of the ICS standards. Modules that implement a part of the standard are prefixed with the standard's designation.

<br/>

### `📄 modules/src/core`

<br/>

Consists of the designs and logic pertaining to the transport, authentication, and ordering layers of the IBC protocol, the fundamental pieces.

<br/>

##### ICS 02 - Client

Clients encapsulate all of the verification methods of another IBC-enabled chain in order to ensure that the other chain adheres to the IBC protocol and does not exhibit misbehaviour. Clients "track" the metadata of the other chain's blocks, and each chain has a client for every other chain that it communicates with.

##### ICS 03 - Connection

Connections associate a chain with another chain by connecting a client on the local chain with a client on the remote chain. This association is pair-wise unique and is established between two chains following a 4-step handshake process.

##### ICS 04 - Channel

Channels are an abstraction layer that facilitate communication between applications and the chains those applications are built upon. One important function that channels can fulfill is guaranteeing that data packets sent between an application and its chain are well-ordered.

##### ICS 05 - Port

The port standard specifies an allocation scheme by which modules can bind to uniquely-named ports allocated by the IBC handler in order to facilitate module-to-module traffic. These ports are used to open channels and can be transferred or released by the module which originally bound them.

##### ICS 23 - Commitment

Commitments (sometimes called _vector commitments_) define an efficient cryptographic construction to prove inclusion or non-inclusion of values in at particular paths in state. This scheme provides a guarantee of a particular state transition that has occurred on one chain which can be verified on another chain.

#### `📄 modules/src/applications`

Consists of various packet encoding and processing semantics which underpin the various types of transactions that users can perform on any IBC-compliant chain.

##### ICS 20 - Fungible Token Transfer

Specifies the packet data structure, state machine handling logic, and encoding details used for transferring fungible tokens between IBC chains. This process preserves asset fungibility and ownership while limiting the impact of Byzantine faults.

#### `📄 modules/src/clients`

Consists of implementations of client verification algorithms (following the base client interface that is defined in `Core`) for specific types of chains. A chain uses these verification algorithms to verify the state of a remote chain.

##### ICS 07 - Tendermint

The Tendermint client implements a client verification algorithm for blockchains which use the Tendermint consensus algorithm. This enables state machines of various sorts replicated using the Tendermint consensus algorithm to interface with other replicated state machines or solo machines over IBC.

#### `📄 modules/src/relayer`

Contains utilities for testing the `📄 modules` crate against the Hermes IBC relayer, acting as scaffolding that enables the two crates to interact for testing, and other, purposes.

<br/>

##### ICS 18 - Relayer

Relayer algorithms are the "physical" connection layer of IBC — off-chain processes responsible for relaying data between two chains running the IBC protocol by scanning the state of each chain, constructing appropriate datagrams, and executing them on the opposite chain as allowed by the protocol.

<br/>

### `📄 relayer`

<br/>

This crate provides the logic for relaying datagrams between chains. The process of relaying packets is an off-chain process that is kicked off by submitting transactions to read from or write to an IBC-enabled chain's state. More broadly, a relayer enables a chain to ascertain another chain's state by accessing its clients, connections, channels, or anything that is IBC-related.

<br/>

### `📄 relayer-cli`

<br/>

A CLI wrapper around the `📄 relayer` crate for running and issuing commands via a relayer. This crate is the one that exposes the Hermes binary.

<br/>

### `📄 relayer-rest`

<br/>

An add-on to the CLI mainly for exposing some internal runtime details of Hermes for debugging and observability reasons.

<br/>

### `📄 proto-compiler`

<br/>

A CLI tool to automate the compilation of proto buffers, which allows Hermes developers to go from a type specified in proto files to generate client gRPC code or server gRPC code.

<br/>

### `📄 proto`

<br/>

Depends on the `📄 proto-compiler` crate's generated proto files.

<br/>

Consists of protobuf-generated Rust types which are necessary for interacting with the Cosmos SDK. Also contains client and server methods that the relayer library includes for accessing the gRPC calls of a chain.

<br/>

### `📄 telemetry`

<br/>

Used by Hermes to gather telemetry data and expose them via a [Prometheus](https://prometheus.io/) endpoint.

<br/>

# Cross-Cutting Concerns

<br/>

## Testing

<br/>

Most of the components in the `📄 modules` crate have basic unit testing coverage. These unit tests make use of mocked up chain components in order to ensure that message payloads are being sent and received as expected.

<br/>

We also run end-to-end tests to more thoroughly test IBC modules in a more heterogenous fashion.

<br/>

## Error Handling

<br/>

Most errors occur within the relayer as a result of either I/O operations or user misconfiguration. I/O-related errors can be sub-categorized into web socket errors and chain RPC errors. The latter occur when full nodes are out of sync with the rest of the network, which result in transactions that are based off of conflicting chain states. Such errors are usually either resolved by retrying the transaction, or might require operator intervention in order to flush the transaction from the mempool in conjunction with restarting the full node.

The [flex-error](https://github.com/informalsystems/flex-error) library is the main tool used to handle errors in the code. This [demo](https://github.com/informalsystems/flex-error/blob/master/flex-error-demo-full/src/main.rs) showcases some of the main patterns of how the flex-error crate is used. For a more real-world example, `📄 relayer/src/error.rs` defines all of the possible errors that the relayer might propagate.

<br/>

This file was generated by Swimm. [Click here to view it in the app](https://app.swimm.io/repos/Z2l0aHViJTNBJTNBaWJjLXJzJTNBJTNBc2VhbmNoZW4xOTkx/docs/h25di).