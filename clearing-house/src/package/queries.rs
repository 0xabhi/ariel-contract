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
    GetLength {},
    GetOrderState {},
    GetPartialLiquidationClosePercentage {},
    GetPartialLiquidationPenaltyPercentage {},
    GetFullLiquidationPenaltyPercentage {},
    GetPartialLiquidatorSharePercentage {},
    GetFullLiquidatorSharePercentage {},
    GetMaxDepositLimit {},
    GetFeeStructure {},
    GetCurveHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetDepositHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetFundingPaymentHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetFundingRateHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    GetLiquidationHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetTradeHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetMarketInfo {
        market_index: u64,
    },
}
