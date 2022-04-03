# Osmosis ICS20 Swap

Swap assets and add liquidity in osmosis from remote chains.

## Messages

- `Transfer{}`: Transfer native tokens over IBC (ics20).

## IBC Packet

**Ics20Packet**
Data packet sent by a blockchain to Osmosis chain to make custom action. It contains the following parameters:

| Parameter | Type   | Description               |
|-----------|--------|---------------------------|
| Denom     | string | From ICS20                |
| Amount    | string | From ICS20                |
| Sender    | string | From ICS20                |
| Receiver  | string | From ICS20                |
| action    | object | Osmosis action (optional) |

Osmosis actions supported:

- `Swap`: Swap assets
- `JoinPool`: Add liquidity
- `ExitPool`: Exit liquidity


### SwapPacket

| Parameter         | Type                                                                                                             | Description       |
|-------------------|------------------------------------------------------------------------------------------------------------------|-------------------|
| Routes            | [SwapAmountInRoute](https://github.com/osmosis-labs/osmosis/blob/v6.2.0/proto/osmosis/gamm/v1beta1/tx.proto#L81) | From osmosis      |
| TokenOutMinAmount | string                                                                                                           | Min output amount |


### JoinPoolPacket

| Parameter         | Type   | Description             |
|-------------------|--------|-------------------------|
| PoolID            | string | Pool asset ID           |
| ShareOutMinAmount | string | Min share output amount |

### ExitPoolPacket

| Parameter         | Type   | Description       |
|-------------------|--------|-------------------|
| TokenOutDenom     | string | Output denom      |
| TokenOutMinAmount | string | Min output amount |

### LockPacket

| Parameter         | Type   | Description                     |
|-------------------|--------|---------------------------------|
| Duration          | string | A valid osmosis lockup duration |

### ClaimPacket

| Parameter | Type   | Description                  |
|-----------|--------|------------------------------|
| Denom     | string | Denom of accumulated rewards |

### UnlockPacket

| Parameter | Type   | Description |
|-----------|--------|-------------|
| ID        | string | Lockup ID   |

### AmountResultAck

Ack result for actions (Swap, JoinPool, ExitPool, ClaimRewards)

| Parameter | Type   |
|-----------|--------|
| Denom     | string |
| Amount    | string |

### LockResultAck

Ack result for LockPacket action.

| Parameter | Type   | Description |
|-----------|--------|-------------|
| LockID    | string | Lockup ID   |




