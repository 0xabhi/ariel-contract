use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetUser {
        user_address: String,
    },
    GetUserMarketPosition {
        user_address: String,
        index: u64,
    },
    // GetUserPositions {
    //     user_address: String,
    //     start_after: Option<String>,
    //     limit: Option<u32>,
    // },
    GetAdmin {},
    IsExchangePaused {},
    IsFundingPaused {},
    AdminControlsPrices {},
    GetVaults {},
    GetMarginRatio {},
    GetOracle {},
    GetMarketLength {},
    GetOracleGuardRails {},
    GetOrderState {},
    GetPartialLiquidationClosePercentage {},
    GetPartialLiquidationPenaltyPercentage {},
    GetFullLiquidationPenaltyPercentage {},
    GetPartialLiquidatorSharePercentage {},
    GetFullLiquidatorSharePercentage {},
    GetMaxDepositLimit {},
    GetFeeStructure {},
    GetMarketInfo {
        market_index: u64,
    },
}
