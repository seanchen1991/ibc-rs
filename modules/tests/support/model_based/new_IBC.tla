---- MODULE IBC ----

EXTENDS Integers, FiniteSets

(* @typeAlias: HEIGHT = [
        revisionNumber: Int,
        revisionHeight: Int
    ]; 
*)
(* @typeAlias: HEADER = [
        chainId: Str,
        height: HEIGHT
    ];
*)
(* @typeAlias: CLIENT = [
        heights: Set(HEIGHT)
    ];
*)
(* @typeAlias: CHAIN = [
        height: HEIGHT,
        clients: Int -> CLIENT,
        clientIdCounter: Int,
        connections: Int -> CONNECTION,
        connectionIdCounter: Int,
        connectionProofs: Set(ACTION)
    ];
*)
EXTypeAliases = TRUE

VARIABLES
    (* @type: [
            headers: Set(HEADER),
            chains: Str -> CHAIN,
            outcome: Str
        ]
    *)
    state

(*

PLAN:
    - For every step, choose a chain to run a method on and advance by one block
    - The chain will advance whether or not the method has errored
    - If the method produces a message for the other chain, add it to messages set
    - If the method can only be triggered in response to a message from the other chain, run it with a message from the messages set
    - Find some way to inject invalid and out of order messages without trying every possible combination of messages
*)

ChainIds == { "chainA", "chainB" }

\* @type: (Str) => Str
OtherChainId(chainId) ==
    IF chainId = "chainA":
        "chainB"
    ELSE
        "chainA"


\* retrieves `clientId`'s data
\* @type: ((Int -> CLIENT), Int) => CLIENT;
ICS02_GetClient(clients, clientId) ==
    clients[clientId]

\* check if `clientId` exists
\* @type: ((Int -> CLIENT), Int) => Bool;
ICS02_ClientExists(clients, clientId) ==
    ICS02_GetClient(clients, clientId).heights /= {}

\* update `clientId`'s data
\* @type: ((Int -> CLIENT), Int, CLIENT) => (Int -> CLIENT);
ICS02_SetClient(clients, clientId, client) ==
    [clients EXCEPT ![clientId] = client]

AdvanceChainHeight(chain) == 
    [chain EXCEPT ![height] = [chain.height EXCEPT ![revisionHeight] = chain.height.revisionHeight + 1]]

ICS02_CreateClient(chain, chainId, header) ==
    \* check if the client exists (it shouldn't)
    IF ICS02_ClientExists(chain.clients, chain.clientIdCounter) THEN
        \* if the client to be created already exists,
        \* then there's an error in the model
        outcome' = "ModelError"
        chains' = chains
        headers' = headers
    ELSE
        LET chain = AdvanceChainHeight(chain) IN
        outcome' = "Ics02CreateOk"
        chains' = [chains EXCEPT ![chainId] = chain]
        headers' = {[
            chainId |-> chainId,
            height |-> chain.height 
        ]}

ICS02_UpdateClient(chain, chainId, header) ==
    \* check if the client exists
    IF ~ICS02_ClientExists(chain.clients, clientId) THEN
        \* if the client does not exist, then set an error outcome
        state' = [state EXCEPT
            !.outcome = "Ics02ClientNotFound"
        ]
    ELSE
        \* if the client exists, check its height
        LET client == ICS02_GetClient(chain.clients, clientId) IN
        LET highestHeight == FindMaxHeight(client.heights) IN
        IF ~HigherRevisionHeight(height, highestHeight) THEN
            \* if the client's new height is not at the same revision number and a higher
            \* block height than the highest client height, then set an error outcome
            state' = [state EXCEPT
                !.outcome = "Ics02HeaderVerificationFailure"
            ]
        ELSE
            \* if the client's new height is higher than the highest client
            \* height, then update the client
            LET updatedClient == [client EXCEPT
                !.heights = client.heights \union {height}
            ] IN
            \* return result with updated state
            [
                clients |-> ICS02_SetClient(
                    chain.clients,
                    clientId,
                    updatedClient
                ),
                action |-> action_,
                outcome |-> "Ics02UpdateOk"
            ]

Next ==
    \E header in headers:
        LET chainId == OtherChainId(header.chainId) IN
        LET chain == chains[chainId] IN
            \/  



