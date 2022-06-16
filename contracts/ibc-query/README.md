# IBC Query

Contract to allow ibc queries from remote chains.

## IBC Packet

**PacketMsg**:
Data packet sent by a blockchain. It contains the following parameters:

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| query     | string | GammMsg query |


Packet Msg example:
- SpotPrice:
```json
{
  "spot_price": {
    "pool": "1",
    "token_in": "uosmo",
    "token_out": "uion"
  }
}
```

- EstimateSwap:
```json
{
  "estimate_swap": {
    "pool": "1",
    "sender": "osmo16vj8qhvhvjptnlre8ke8p37f54z9wy68p7hxf6",
    "amount": "1000000",
    "token_in": "uosmo",
    "token_out": "uion"
  }
}
```

## Contract QueryMsg

- `ListAccounts` - to list all accounts tied to open channels. ChannelID,
  account address on the remote chain (if known) and last updated price.
- `Account` - queries the above data for one channel
