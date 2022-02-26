# CW20 ICS20

This is a fork from [Cosmwasm ICS20 contract](https://github.com/CosmWasm/cw-plus/tree/v0.12.1/contracts/cw20-ics20)
that extends functionality to receive native tokens from other chains.

## Messages

- `AllowExternalToken{}`: Register tokens from other chain.


## Queries

Additional queries

* `Port{}` - returns the port ID this contract has bound, so you can create channels. This info can be queried
  via wasmd contract info query, but we expose another query here for convenience.
* `ListChannels{}` - returns a (currently unpaginated) list of all channels that have been created on this contract.
  Returns their local channelId along with some basic metadata, like the remote port/channel and the connection they
  run on top of.
* `Channel{id}` - returns more detailed information on one specific channel. In addition to the information available
  in the list view, it returns the current outstanding balance on that channel, as well as the total amount that
  has ever been sent on the channel.

