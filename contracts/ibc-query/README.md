# IBC gamm contract

This is a contract to demonstrate osmosis query price over ibc.

## Contract

**ExecuteMsg**:

- `SpotPrice` - this will send `GammPrice` packet to query spot price in remote osmosis chain 
 and store the info locally

Msg example:
```json
{
  "spot_price": {
    "channel": "channel-13",
    "pool": "2",
    "token_in": "uosmo",
    "token_out": "ibc/0F192F25408BEF0845A4EFF1FB52CF4D390C224D21543F30DE84651745A6F9A2",
    "timeout": 1200
  }
}
```

**QueryMsg**:

- `ListAccounts` - to list all accounts tied to open channels. ChannelID,
  account address on the remote chain (if known) and last updated price.
- `Account` - queries the above data for one channel
