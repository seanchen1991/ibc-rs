---- MODULE main ----

EXTENDS transfer

\* PacketAcknowledgedTest == action.name = IBCTransferAcknowledgePacketAction
PacketTimeoutTest == action.name = IBCTransferAcknowledgePacketAction


Invariant ==
    ~PacketTimeoutTest

====