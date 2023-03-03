# Fees


#### Fee Handler Router

[Fee Handler Router pallet](https://github.com/sygmaprotocol/sygma-substrate-pallets/blob/main/fee-handler-router/src/lib.rs) enables registration of different fee strategies per resource ID and domain ID, which facilitates [Sygma's approach to managing fees with granularity](https://github.com/sygmaprotocol/sygma-relayer/blob/main/docs/general/Fees.md#fees).

To configure the router, pallet implements [`set_fee_handler`](https://github.com/sygmaprotocol/sygma-substrate-pallets/blob/main/fee-handler-router/src/lib.rs#L77) method. With this method, the administrator can register a particular fee strategy for bridging a resource (specified by the `asset` parameter) to a specific destination domain (specified by the `domain` parameter).

#### Basic Fee Handler

This handler facilitates static fee strategy

#### Dynamic Fee Handler

This handler facilitates dynamic fee strategy

![](/docs/resources/dynamic-fees-substrate.png)