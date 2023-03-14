# Fees

#### Fee Handler Router

[Fee Handler Router pallet](https://github.com/sygmaprotocol/sygma-substrate-pallets/blob/main/fee-handler-router/src/lib.rs) enables registration of different fee strategies per resource ID and domain ID, which facilitates [Sygma's approach to managing fees with granularity](https://github.com/sygmaprotocol/sygma-relayer/blob/main/docs/general/Fees.md#fees).

To configure the router, pallet implements [`set_fee_handler`](https://github.com/sygmaprotocol/sygma-substrate-pallets/blob/main/fee-handler-router/src/lib.rs#L77) method. With this method, the administrator can register a particular fee strategy for bridging a resource (specified by the `asset` parameter) to a specific destination domain (specified by the `domain` parameter).

#### Basic Fee Handler

This handler facilitates static fee strategy

#### Dynamic Fee Handler

Dynamic Fee Handler processes fee estimate message that was provided on deposit and calculates fee amount that should be transfered as fee. Fee calculation inside fee handler is following this logic:

##### Token transfer: fee is paid in a token being transferred

destination -> EVM

`final_fee = feeOracleMsg.dstGasPrice * _gasUsed * feeOracleMsg.ter`

destination -> Substrate

`final_fee = feeOracleMsg.inclusionFee * feeOracleMsg.ter`

##### Generic messages: fee is paid in base currency

destination -> EVM

`final_fee = feeOracleMsg.dstGasPrice * feeOracleMsg.msgGasLimit * feeOracleMsg.ber`

destination -> Substrate

`final_fee = feeOracleMsg.inclusionFee * feeOracleMsg.ber`

![](/docs/resources/dynamic-fees-substrate.png)