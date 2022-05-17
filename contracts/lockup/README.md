# Osmosis lockup

You can use this contract as a lockup account, this contract allows you to lock and unlock LP tokens,
and it is a recipient for lp rewards.

## Messages

- `Lock{}`: Lock LP tokens.
- `Unlock{}`: Unlock LP tokens.
- `Claim{}`: Get accumulated rewards ang LP tokens after lock period end.
- `UpdateAdmin{}`: Change admin account.

## Queries

- `Admin{}` - Get current admin.
