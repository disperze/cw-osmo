# IBC Query

Contract to allow ibc queries from remote chains.

## IBC Packet

**PacketMsg**:
Data packet sent by a blockchain. It contains the following parameters:

| Parameter | Type   | Description                       |
|-----------|--------|-----------------------------------|
| Path      | string | Service query path                |
| Data      | string | Protobuf message (base64 encoded) |

Packet Msg example:
```json
{
  "path": "/osmosis.gamm.v1beta1.Query/SpotPrice",
  "data": "CMQEEkRpYmMvQkUxQkI0MkQ0QkUz....jY4RDdDNDFEQjRERkNFOTY3OEU4RU="
}
```

Packet ACK example:
```json
{
  "result": "ChY5NTUuNjcxNzc5MTY3NDI4NzM4MjAw"
}
```
> return protobuf result (base64 encoded)

## Contract QueryMsg

- `ListAccounts` - to list all accounts tied to open channels. ChannelID,
  account address on the remote chain (if known) and last updated price.
- `Account` - queries the above data for one channel
