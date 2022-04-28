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
    GetUserPositions {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
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
    GetCurveHistoryLength {},
    GetCurveHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetDepositHistoryLength {},
    GetDepositHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetFundingPaymentHistoryLength {},
    GetFundingPaymentHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetFundingRateHistoryLength {},
    GetFundingRateHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    GetLiquidationHistoryLength {},
    GetLiquidationHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetTradeHistoryLength {},
    GetTradeHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetMarketInfo {
        market_index: u64,
    },
}
