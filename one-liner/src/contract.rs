use std::{cmp::{max, min}, ops::{Div, Mul}};
use num_integer::Roots;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::{Map, Item};
use cw_controllers::Admin;

use cosmwasm_std::{
    Addr, Api, BalanceResponse, BankQuery, MessageInfo, QuerierWrapper, QueryRequest, StdError,
    StdResult, Uint128, to_binary, CosmosMsg, Decimal, DepsMut, Env, Response,
    WasmMsg, Deps, Fraction, coins, Order, entry_point, Binary
};
use cw2::set_contract_version;
use cw_storage_plus::{Bound, PrimaryKey};

use crate::ContractError;

#[derive(Serialize, Deserialize, Debug, PartialEq, JsonSchema, Clone, Copy)]
pub struct Number128 {
    pub amount: Uint128,
    pub is_positive: bool,
}

impl Number128 {
    pub const fn new(value: i128) -> Self {
        Number128{
            amount: Uint128::new(value.unsigned_abs()),
            is_positive: value.is_positive()
        }
    }
    pub const fn zero() -> Self {
        Number128 { amount: Uint128::zero(), is_positive: true }
    }
    /// Returns a copy of the internal data
    pub const fn i128(&self) -> i128 {
        if self.is_positive {
            self.amount.u128() as i128
        } else {
            -(self.amount.u128() as i128)
        }
    }
}

// PRECISIONS
pub const AMM_RESERVE_PRECISION: Uint128 = Uint128::new(10_000_000_000_000); //expo = -13;
pub const MARK_PRICE_PRECISION: Uint128 =  Uint128::new(10_000_000_000); //expo = -10
pub const QUOTE_PRECISION: Uint128 =  Uint128::new(1_000_000); // expo = -6
pub const FUNDING_PAYMENT_PRECISION: Uint128 = Uint128::new(10_000); // expo = -4
pub const MARGIN_PRECISION: Uint128 = Uint128::new(10_000); // expo = -4
pub const PEG_PRECISION: Uint128 = Uint128::new(1_000); //expo = -3
pub const PRICE_SPREAD_PRECISION: i128 = 10_000; // expo = -4
pub const PRICE_SPREAD_PRECISION_U128: Uint128 = Uint128::new(10_000); // expo = -4

// PRECISION CONVERSIONS
pub const PRICE_TO_PEG_PRECISION_RATIO: Uint128 = Uint128::new(10_000_000); // MARK_PRICE_PRECISION / PEG_PRECISION; // expo: 7
pub const PRICE_TO_PEG_QUOTE_PRECISION_RATIO: Uint128 = Uint128::new(10_000); // MARK_PRICE_PRECISION / QUOTE_PRECISION; // expo: 4
pub const AMM_TO_QUOTE_PRECISION_RATIO: Uint128 = Uint128::new(10_000_000); // AMM_RESERVE_PRECISION / QUOTE_PRECISION; // expo: 7
pub const AMM_TO_QUOTE_PRECISION_RATIO_I128: Uint128 = Uint128::new(10_000_000); // AMM_RESERVE_PRECISION / QUOTE_PRECISION ; // expo: 7
pub const AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO: Uint128 = Uint128::new(10_000_000_000);    // AMM_RESERVE_PRECISION * PEG_PRECISION / QUOTE_PRECISION; // expo: 10
pub const QUOTE_TO_BASE_AMT_FUNDING_PRECISION: Uint128 = Uint128::new(1_000_000_000_000_000_000_000); // AMM_RESERVE_PRECISION * MARK_PRICE_PRECISION * FUNDING_PAYMENT_PRECISION / QUOTE_PRECISION; // expo: 21

pub const PRICE_TO_QUOTE_PRECISION_RATIO: Uint128 = Uint128::new(10_000); // MARK_PRICE_PRECISION / QUOTE_PRECISION; // expo: 4
pub const MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO: Uint128 =
    Uint128::new(MARK_PRICE_PRECISION.u128() * AMM_TO_QUOTE_PRECISION_RATIO.u128()); // expo 17

// FEE REBATES
pub const UPDATE_K_ALLOWED_PRICE_CHANGE: Uint128 = Uint128::new(1_000_000_000); // MARK_PRICE_PRECISION / Uint128::new(10));

// TIME PERIODS
pub const ONE_HOUR: Uint128 =  Uint128::new(3600);

// FEES
pub const SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR: Uint128 = Uint128::new(5);
pub const SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR: Uint128 = Uint128::new(100);

pub const DEFAULT_FEE_NUMERATOR: Uint128 = Uint128::new(1);
pub const DEFAULT_FEE_DENOMINATOR: Uint128 = Uint128::new(1000);

pub const DEFAULT_DISCOUNT_TOKEN_FIRST_TIER_MINIMUM_BALANCE: Uint128 = Uint128::new(1_000_000_000_000); // 1000

pub const DEFAULT_DISCOUNT_TOKEN_FIRST_TIER_DISCOUNT_NUMERATOR: Uint128 = Uint128::new(20);
pub const DEFAULT_DISCOUNT_TOKEN_FIRST_TIER_DISCOUNT_DENOMINATOR: Uint128 = Uint128::new(100);

pub const DEFAULT_DISCOUNT_TOKEN_SECOND_TIER_MINIMUM_BALANCE: Uint128 = Uint128::new(100_000_000_000);

pub const DEFAULT_DISCOUNT_TOKEN_SECOND_TIER_DISCOUNT_NUMERATOR: Uint128 = Uint128::new(15);
pub const DEFAULT_DISCOUNT_TOKEN_SECOND_TIER_DISCOUNT_DENOMINATOR: Uint128 = Uint128::new(100);

pub const DEFAULT_DISCOUNT_TOKEN_THIRD_TIER_MINIMUM_BALANCE: Uint128 = Uint128::new(10_000_000_000);

pub const DEFAULT_DISCOUNT_TOKEN_THIRD_TIER_DISCOUNT_NUMERATOR: Uint128 = Uint128::new(10);
pub const DEFAULT_DISCOUNT_TOKEN_THIRD_TIER_DISCOUNT_DENOMINATOR: Uint128 = Uint128::new(100);

pub const DEFAULT_DISCOUNT_TOKEN_FOURTH_TIER_MINIMUM_BALANCE: Uint128 = Uint128::new(1_000_000_000);

pub const DEFAULT_DISCOUNT_TOKEN_FOURTH_TIER_DISCOUNT_NUMERATOR: Uint128 = Uint128::new(5);
pub const DEFAULT_DISCOUNT_TOKEN_FOURTH_TIER_DISCOUNT_DENOMINATOR: Uint128 = Uint128::new(100);

pub const DEFAULT_REFERRER_REWARD_NUMERATOR: Uint128 = Uint128::new(5);
pub const DEFAULT_REFERRER_REWARD_DENOMINATOR: Uint128 = Uint128::new(100);

pub const DEFAULT_REFEREE_DISCOUNT_NUMERATOR: Uint128 = Uint128::new(5);
pub const DEFAULT_REFEREE_DISCOUNT_DENOMINATOR: Uint128 = Uint128::new(100);

// CONSTRAINTS
pub const MAX_LIQUIDATION_SLIPPAGE: Uint128 = Uint128::new(100); // expo = -2
pub const MAX_LIQUIDATION_SLIPPAGE_U128: Uint128 = Uint128::new(100); // expo = -2
pub const MAX_MARK_TWAP_DIVERGENCE: Uint128 = Uint128::new(5_000); // expo = -3
pub const MAXIMUM_MARGIN_RATIO: Uint128 = MARGIN_PRECISION;
pub const MINIMUM_MARGIN_RATIO: Uint128 =  Uint128::new(200);// MARGIN_PRECISION / Uint128::new(50);

// iterator limits
pub const MAX_LIMIT: u32 = 20;
pub const DEFAULT_LIMIT: u32 = 10;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Type {
    Repeg,
    UpdateK,
}

impl Default for Type {
    // UpOnly
    fn default() -> Self {
        Type::Repeg
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurveRecord {
    pub ts: u64,
    pub record_id: u64,
    pub market_index: u64,
    pub peg_multiplier_before: Uint128,
    pub peg_multiplier_after: Uint128,
    pub base_asset_reserve_before: Uint128,
    pub base_asset_reserve_after: Uint128,
    pub quote_asset_reserve_before: Uint128,
    pub quote_asset_reserve_after: Uint128,
    pub sqrt_k_before: Uint128,
    pub sqrt_k_after: Uint128,
    pub base_asset_amount_long: Uint128,
    pub base_asset_amount_short: Uint128,
    pub base_asset_amount: Number128,
    pub open_interest: Uint128,
    pub total_fee: Uint128,
    pub total_fee_minus_distributions: Uint128,
    pub adjustment_cost: Number128,
    pub oracle_price: Number128
}

pub const CURVEHISTORY: Map<String,  CurveRecord> = Map::new("curve_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositRecord {
    pub ts: u64,
    pub record_id: u64,
    pub user: Addr,
    pub direction: DepositDirection,
    pub collateral_before: Uint128,
    pub cumulative_deposits_before: Uint128,
    pub amount: u64,
}

pub const DEPOSIT_HISTORY: Map<(Addr, String),  DepositRecord> = Map::new("deposit_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingPaymentRecord {
    pub ts: u64,
    pub record_id: u64,
    pub user: Addr,
    pub market_index: u64,
    pub funding_payment: Number128,
    pub base_asset_amount: Number128,
    pub user_last_cumulative_funding: Number128,
    pub user_last_funding_rate_ts: u64,
    pub amm_cumulative_funding_long: Number128,
    pub amm_cumulative_funding_short: Number128,
}

pub const FUNDING_PAYMENT_HISTORY: Map<(&Addr, String),  FundingPaymentRecord> = Map::new("funding_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingRateRecord {
    pub ts: u64,
    pub record_id: u64,
    pub market_index: u64,
    pub funding_rate: Number128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub oracle_price_twap: Number128,
    pub mark_price_twap: Uint128,
}

pub const FUNDING_RATE_HISTORY: Map<String,  FundingRateRecord> = Map::new("funding_payment_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationRecord {
    pub ts: u64,
    pub record_id: u64,
    pub user: Addr,
    pub partial: bool,
    pub base_asset_value: Uint128,
    pub base_asset_value_closed: Uint128,
    pub liquidation_fee: Uint128,
    pub fee_to_liquidator: u64,
    pub fee_to_insurance_fund: u64,
    pub liquidator: Addr,
    pub total_collateral: Uint128,
    pub collateral: Uint128,
    pub unrealized_pnl: Number128,
    pub margin_ratio: Uint128,
}

pub const LIQUIDATION_HISTORY: Map<(Addr, String),  LiquidationRecord> = Map::new("liquidation_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderAction {
    Place,
    Cancel,
    Fill,
    Expire,
}

impl Default for OrderAction {
    // UpOnly
    fn default() -> Self {
        OrderAction::Place
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TradeRecord {
    pub ts: u64,
    pub user: Addr,
    pub direction: PositionDirection,
    pub base_asset_amount: Uint128,
    pub quote_asset_amount: Uint128,
    pub mark_price_before: Uint128,
    pub mark_price_after: Uint128,
    pub fee: Uint128,
    pub referrer_reward: Uint128,
    pub referee_discount: Uint128,
    pub token_discount: Uint128,
    pub liquidation: bool,
    pub market_index: u64,
    pub oracle_price: Number128,
}

pub const TRADE_HISTORY: Map<(&Addr, String),  TradeRecord> = Map::new("trade_history");


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Market {
    pub market_name: String,
    pub initialized: bool,
    pub base_asset_amount_long: Number128,
    pub base_asset_amount_short: Number128,
    pub base_asset_amount: Number128, // net market bias
    pub open_interest: Uint128,     // number of users in a position
    pub amm: Amm,
    pub margin_ratio_initial: u32,
    pub margin_ratio_partial: u32,
    pub margin_ratio_maintenance: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Amm {
    pub oracle: Addr,
    pub oracle_source: OracleSource,
    pub base_asset_reserve: Uint128,
    pub quote_asset_reserve: Uint128,
    pub cumulative_repeg_rebate_long: Uint128,
    pub cumulative_repeg_rebate_short: Uint128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub last_funding_rate: Number128,
    pub last_funding_rate_ts: u64,
    pub funding_period: u64,
    pub sqrt_k: Uint128,
    pub peg_multiplier: Uint128,
    pub total_fee: Uint128,
    pub last_mark_price_twap: Uint128,
    pub last_mark_price_twap_ts: u64,
    pub total_fee_minus_distributions: Uint128,
    pub total_fee_withdrawn: Uint128,
    pub minimum_quote_asset_trade_size: Uint128,
    pub last_oracle_price_twap_ts: u64,
    pub last_oracle_price: Number128,
    pub last_oracle_price_twap: Number128,
    pub minimum_base_asset_trade_size: Uint128,
}

pub const MARKETS: Map<String, Market> = Map::new("markets");

impl Amm {
    pub fn mark_price(&self) -> Result<Uint128, ContractError> {
        calculate_price(
            self.quote_asset_reserve,
            self.base_asset_reserve,
            self.peg_multiplier,
        )
    }

    pub fn get_oracle_price(
        &self
    ) -> Result<OraclePriceData, ContractError> {
        Ok(OraclePriceData {
            price: self.last_oracle_price,
            confidence: Uint128::from(100 as u32),
            delay: 0,
            has_sufficient_number_of_data_points: true,
        })
    }

    pub fn get_oracle_twap(&self) -> Result<Option<i128>, ContractError> {
        if self.last_mark_price_twap.ne(&Uint128::zero()) {
            Ok(Some(self.last_oracle_price_twap.i128()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum LiquidationType {
    NONE,
    PARTIAL,
    FULL,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationStatus {
    pub liquidation_type: LiquidationType,
    pub margin_requirement: Uint128,
    pub total_collateral: Uint128,
    pub unrealized_pnl: i128,
    pub adjusted_total_collateral: Uint128,
    pub base_asset_value: Uint128,
    pub margin_ratio: Uint128,
    pub market_statuses: Vec<MarketStatus>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketStatus {
    pub market_index: u64,
    pub partial_margin_requirement: Uint128,
    pub maintenance_margin_requirement: Uint128,
    pub base_asset_value: Uint128,
    pub mark_price_before: Uint128,
    pub close_position_slippage: Option<i128>,
    pub oracle_status: OracleStatus,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub exchange_paused: bool,
    pub funding_paused: bool,
    pub admin_controls_prices: bool,
    pub collateral_vault: Addr,
    pub insurance_vault: Addr,
    pub oracle: Addr,
    pub margin_ratio_initial: Uint128,
    pub margin_ratio_maintenance: Uint128,
    pub margin_ratio_partial: Uint128,
    
    pub partial_liquidation_close_percentage: Decimal,
    pub partial_liquidation_penalty_percentage: Decimal,
    pub full_liquidation_penalty_percentage: Decimal,

    pub partial_liquidation_liquidator_share_denominator: u64,
    pub full_liquidation_liquidator_share_denominator: u64,

    pub max_deposit: Uint128,
    pub markets_length: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Length {
    pub curve_history_length: u64,
    pub deposit_history_length: u64,
    pub funding_payment_history_length: u64,
    pub funding_rate_history_length: u64,
    pub liquidation_history_length: u64,
    pub order_history_length: u64,
    pub trade_history_length: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OrderState {
    pub min_order_quote_asset_amount: Uint128, 
    pub reward: Decimal,
    pub time_based_reward_lower_bound: Uint128,
}

pub const STATE: Item<State> = Item::new("state");
pub const ADMIN: Admin = Admin::new("admin");
pub const FEESTRUCTURE: Item<FeeStructure> = Item::new("fee_structure");
pub const ORACLEGUARDRAILS: Item<OracleGuardRails> = Item::new("oracle_guard_rails");
pub const ORDERSTATE: Item<OrderState> = Item::new("order_state");
pub const LENGTH : Item<Length> = Item::new("length");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct User {
    pub collateral: Uint128,
    pub cumulative_deposits: Uint128,
    pub total_fee_paid: Uint128,
    pub total_token_discount: Uint128,
    pub total_referral_reward: Uint128,
    pub total_referee_discount: Uint128,
    pub referrer: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Position {
    pub market_index: u64,
    pub base_asset_amount: Number128,
    pub quote_asset_amount: Uint128,
    pub last_cumulative_funding_rate: Number128,
    pub last_cumulative_repeg_rebate: Uint128,
    pub last_funding_rate_ts: u64,
    pub order_length: u64,
}

pub const USERS: Map<&Addr, User> = Map::new("users");
pub const POSITIONS: Map<(&Addr, String), Position> = Map::new("market_positions");

impl Position {
    pub fn is_for(&self, market_index: u64) -> bool {
        self.market_index == market_index && (self.is_open_position() || self.has_open_order())
    }

    pub fn is_available(&self) -> bool {
        !self.is_open_position() && !self.has_open_order()
    }

    pub fn is_open_position(&self) -> bool {
        self.base_asset_amount.i128() != 0
    }

    pub fn has_open_order(&self) -> bool {
        self.order_length != 0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub collateral_vault: String,
    pub insurance_vault: String,
    pub admin_controls_prices: bool,
    pub oracle: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // market initializer updates AMM structure
    InitializeMarket {
        market_index: u64,
        market_name: String,
        amm_base_asset_reserve: Uint128,
        amm_quote_asset_reserve: Uint128,
        amm_periodicity: u64,
        amm_peg_multiplier: Uint128,
        oracle_source: OracleSource,
        margin_ratio_initial: u32,
        margin_ratio_partial: u32,
        margin_ratio_maintenance: u32,
    },
    //deposit collateral, updates user struct
    DepositCollateral {
        amount: u64,
        referrer: Option<String>
    },
    //user function withdraw collateral, updates user struct
    WithdrawCollateral {
        amount: u64,
    },
    OpenPosition {
        direction: PositionDirection,
        quote_asset_amount: Uint128,
        market_index: u64,
        limit_price: Option<Uint128>,
    },
    ClosePosition {
        market_index: u64,
    },

    Liquidate {
        user: String,
        market_index: u64,
    },
    MoveAMMPrice {
        base_asset_reserve: Uint128,
        quote_asset_reserve: Uint128,
        market_index: u64,
    },
    //user function
    WithdrawFees {
        market_index: u64,
        amount: u64,
    },

    // withdraw from insurance vault sends token but no logic

    //admin function
    WithdrawFromInsuranceVaultToMarket {
        market_index: u64,
        amount: u64,
    },
    //admin function
    RepegAMMCurve {
        new_peg_candidate: Uint128,
        market_index: u64,
    },

    UpdateAMMOracleTwap {
        market_index: u64,
    },

    ResetAMMOracleTwap {
        market_index: u64,
    },
    //user calls it we get the user identification from msg address sender
    SettleFundingPayment {},
    UpdateFundingRate {
        market_index: u64,
    },
    UpdateK {
        market_index: u64,
        sqrt_k: Uint128,
    },
    UpdateMarginRatio {
        market_index: u64,
        margin_ratio_initial: u32,
        margin_ratio_partial: u32,
        margin_ratio_maintenance: u32,
    },
    UpdatePartialLiquidationClosePercentage {
        value: Decimal,
    },
    UpdatePartialLiquidationPenaltyPercentage {
        value: Decimal,
    },
    UpdateFullLiquidationPenaltyPercentage {
        value: Decimal,
    },
    UpdatePartialLiquidationLiquidatorShareDenominator {
        denominator: u64,
    },
    UpdateFullLiquidationLiquidatorShareDenominator {
        denominator: u64,
    },
    UpdateFee {
        fee_: Decimal,
        first_tier_minimum_balance: Uint128,
        first_tier_discount: Decimal,
        second_tier_minimum_balance: Uint128,
        second_tier_discount: Decimal,
        third_tier_minimum_balance: Uint128,
        third_tier_discount: Decimal,
        fourth_tier_minimum_balance: Uint128,
        fourth_tier_discount: Decimal,
        referrer_reward: Decimal,
        referee_discount: Decimal,
    },
    UpdateOraceGuardRails {
        use_for_liquidations: bool,
        mark_oracle_divergence: Decimal,
        slots_before_stale: i64,
        confidence_interval_max_size: Uint128,
        too_volatile_ratio: i128,
    },
    UpdateOrderState {
        min_order_quote_asset_amount: Uint128,
        reward: Decimal,
        time_based_reward_lower_bound: Uint128,
    },
    UpdateMarketOracle {
        market_index: u64,
        oracle: String,
        oracle_source: OracleSource,
    },
    UpdateOracleAddress {
        oracle: String,
    },
    UpdateMarketMinimumQuoteAssetTradeSize {
        market_index: u64,
        minimum_trade_size: Uint128,
    },

    UpdateMarketMinimumBaseAssetTradeSize {
        market_index: u64,
        minimum_trade_size: Uint128,
    },
    // will move to admin controller
    UpdateAdmin {
        admin: String,
    },
    UpdateMaxDeposit {
        max_deposit: Uint128,
    },
    UpdateExchangePaused {
        exchange_paused: bool,
    },
    DisableAdminControlsPrices {},
    UpdateFundingPaused {
        funding_paused: bool,
    },
    OracleFeeder {
        market_index: u64,
        price: i128,
    },
}

pub fn addr_validate_to_lower(api: &dyn Api, addr: &str) -> StdResult<Addr> {
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!(
            "Address {} should be lowercase",
            addr
        )));
    }
    api.addr_validate(addr)
}

pub fn assert_sent_uusd_balance(message_info: &MessageInfo, input_amount: u128) -> StdResult<()> {
    let amount = Uint128::from(input_amount);
    match message_info.funds.iter().find(|x| x.denom == "uusd") {
        Some(coin) => {
            if amount == coin.amount {
                Ok(())
            } else {
                Err(StdError::generic_err(
                    "Native token balance mismatch between the argument and the transferred",
                ))
            }
        }
        None => {
            if amount.is_zero() {
                Ok(())
            } else {
                Err(StdError::generic_err(
                    "Native token balance mismatch between the argument and the transferred",
                ))
            }
        }
    }
}

pub fn query_balance(querier: &QuerierWrapper, account_addr: Addr) -> StdResult<u128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: String::from(account_addr),
        denom: "uusd".to_string(),
    }))?;
    Ok(balance.amount.amount.u128())
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VaultInterface {
    Withdraw{
        to_address: Addr,
        amount: u128
    },
    Deposit {}

}

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserResponse {
    pub collateral: Uint128,
    pub cumulative_deposits: Uint128,
    pub total_fee_paid: Uint128,
    pub total_token_discount: Uint128,
    pub total_referral_reward: Uint128,
    pub total_referee_discount: Uint128,
    pub referrer: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserPositionResponse {
    pub base_asset_amount: Number128,
    pub quote_asset_amount: Uint128,
    pub last_cumulative_funding_rate: Number128,
    pub last_cumulative_repeg_rebate: Uint128,
    pub last_funding_rate_ts: u64,
    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PositionResponse {
    pub base_asset_amount: Number128,
    pub quote_asset_amount: Uint128,
    pub last_cumulative_funding_rate: Number128,
    pub last_cumulative_repeg_rebate: Uint128,
    pub last_funding_rate_ts: u64,
    pub direction: PositionDirection,
    pub initial_size: Uint128,
    pub entry_notional: Number128,
    pub entry_price: Uint128,
    pub pnl: Number128
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AdminResponse {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsExchangePausedResponse {
    pub exchange_paused: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsFundingPausedResponse {
    pub funding_paused: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AdminControlsPricesResponse {
    pub admin_controls_prices: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VaultsResponse {
    pub insurance_vault: String,
    pub collateral_vault: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleResponse {
    pub oracle: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarginRatioResponse {
    pub margin_ratio_initial: Uint128,
    pub margin_ratio_maintenance: Uint128,
    pub margin_ratio_partial: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PartialLiquidationClosePercentageResponse {
    pub value: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PartialLiquidationPenaltyPercentageResponse {
    pub value: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FullLiquidationPenaltyPercentageResponse {
    pub value: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PartialLiquidatorSharePercentageResponse {
    pub denominator: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FullLiquidatorSharePercentageResponse {
    pub denominator: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MaxDepositLimitResponse {
    pub max_deposit: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeStructureResponse {
    pub fee: Decimal,
    pub first_tier_minimum_balance: Uint128,
    pub first_tier_discount : Decimal,
    pub second_tier_minimum_balance : Uint128,
    pub second_tier_discount : Decimal,
    pub third_tier_minimum_balance : Uint128,
    pub third_tier_discount : Decimal,
    pub fourth_tier_minimum_balance : Uint128,
    pub fourth_tier_discount : Decimal,
    pub referrer_reward : Decimal,
    pub referee_discount : Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleGuardRailsResponse {
    pub use_for_liquidations: bool,
    // oracle price divergence rails
    pub mark_oracle_divergence: Decimal,
    // validity guard rails
    pub slots_before_stale: Number128,
    pub confidence_interval_max_size: Uint128,
    pub too_volatile_ratio: Number128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OrderStateResponse {
    pub min_order_quote_asset_amount: Uint128, // minimum est. quote_asset_amount for place_order to succeed
    pub reward: Decimal,
    pub time_based_reward_lower_bound: Uint128, // minimum filler reward for time-based reward
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LengthResponse {
    pub curve_history_length: u64,
    pub deposit_history_length: u64,
    pub funding_payment_history_length: u64,
    pub funding_rate_history_length: u64,
    pub liquidation_history_length: u64,
    pub order_history_length: u64,
    pub trade_history_length: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketLengthResponse {
    pub length: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurveHistoryResponse {
    pub ts: u64,
    pub record_id: u64,
    pub market_index: u64,
    pub peg_multiplier_before: Uint128,
    pub base_asset_reserve_before: Uint128,
    pub quote_asset_reserve_before: Uint128,
    pub sqrt_k_before: Uint128,
    pub peg_multiplier_after: Uint128,
    pub base_asset_reserve_after: Uint128,
    pub quote_asset_reserve_after: Uint128,
    pub sqrt_k_after: Uint128,
    pub base_asset_amount_long: Uint128,
    pub base_asset_amount_short: Uint128,
    pub base_asset_amount: Number128,
    pub open_interest: Uint128,
    pub total_fee: Uint128,
    pub total_fee_minus_distributions: Uint128,
    pub adjustment_cost: Number128,
    pub oracle_price: Number128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositHistoryResponse {
    pub ts: u64,
    pub record_id: u64,
    pub user: String,
    pub direction: DepositDirection,
    pub collateral_before: Uint128,
    pub cumulative_deposits_before: Uint128,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingPaymentHistoryResponse {
    pub ts: u64,
    pub record_id: u64,
    pub user: String,
    pub market_index: u64,
    pub funding_payment: Number128,
    pub base_asset_amount: Number128,
    pub user_last_cumulative_funding: Number128,
    pub user_last_funding_rate_ts: u64,
    pub amm_cumulative_funding_long: Number128,
    pub amm_cumulative_funding_short: Number128,
}
    
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingRateHistoryResponse {
    pub ts: u64,
    pub record_id: u64,
    pub market_index: u64,
    pub funding_rate: Number128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub oracle_price_twap: Number128,
    pub mark_price_twap: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationHistoryResponse {
    pub ts: u64,
    pub record_id: u64,
    pub user: String,
    pub partial: bool,
    pub base_asset_value: Uint128,
    pub base_asset_value_closed: Uint128,
    pub liquidation_fee: Uint128,
    pub fee_to_liquidator: u64,
    pub fee_to_insurance_fund: u64,
    pub liquidator: String,
    pub total_collateral: Uint128,
    pub collateral: Uint128,
    pub unrealized_pnl: Number128,
    pub margin_ratio: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TradeHistoryResponse {
    pub ts: u64,
    pub user: String,
    pub direction: PositionDirection,
    pub base_asset_amount: Uint128,
    pub quote_asset_amount: Uint128,
    pub mark_price_before: Uint128,
    pub mark_price_after: Uint128,
    pub fee: Uint128,
    pub referrer_reward: Uint128,
    pub referee_discount: Uint128,
    pub token_discount: Uint128,
    pub liquidation: bool,
    pub market_index: u64,
    pub oracle_price: Number128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketInfoResponse {
    pub market_name: String,
    pub initialized: bool,
    pub base_asset_amount_long: Number128,
    pub base_asset_amount_short: Number128,
    pub base_asset_amount: Number128, // net market bias
    pub open_interest: Uint128,
    pub oracle: String,
    pub oracle_source: OracleSource,
    pub base_asset_reserve: Uint128,
    pub quote_asset_reserve: Uint128,
    pub cumulative_repeg_rebate_long: Uint128,
    pub cumulative_repeg_rebate_short: Uint128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub last_funding_rate: Number128,
    pub last_funding_rate_ts: u64,
    pub funding_period: u64,
    pub last_oracle_price_twap: Number128,
    pub last_mark_price_twap: Uint128,
    pub last_mark_price_twap_ts: u64,
    pub sqrt_k: Uint128,
    pub peg_multiplier: Uint128,
    pub total_fee: Uint128,
    pub total_fee_minus_distributions: Uint128,
    pub total_fee_withdrawn: Uint128,
    pub minimum_trade_size: Uint128,
    pub last_oracle_price_twap_ts: u64,
    pub last_oracle_price: Number128,
    pub minimum_base_asset_trade_size: Uint128,
    pub minimum_quote_asset_trade_size: Uint128
}


#[derive(Clone, Debug, JsonSchema, Copy, Serialize, Deserialize, PartialEq)]
pub enum PositionDirection {
    Long,
    Short,
}

impl Default for PositionDirection {
    // UpOnly
    fn default() -> Self {
        PositionDirection::Long
    }
}

#[derive(Clone, Debug, JsonSchema, Copy, Serialize, Deserialize, PartialEq)]
pub enum SwapDirection {
    Add,
    Remove,
}

impl Default for SwapDirection {
    fn default() -> Self {
        SwapDirection::Add
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub enum DepositDirection {
    DEPOSIT,
    WITHDRAW,
}

impl Default for DepositDirection {
    fn default() -> Self {
        DepositDirection::DEPOSIT
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OracleSource {
    Oracle,
}

impl Default for OracleSource {
    // UpOnly
    fn default() -> Self {
        OracleSource::Oracle
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleStatus {
    pub price_data: OraclePriceData,
    pub oracle_mark_spread_pct: Number128,
    pub is_valid: bool,
    pub mark_too_divergent: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OraclePriceData {
    pub price: Number128,
    pub confidence: Uint128,
    pub delay: i64,
    pub has_sufficient_number_of_data_points: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderStatus {
    Init,
    Open,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderType {
    Market,
    Limit,
    TriggerMarket,
    TriggerLimit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderTriggerCondition {
    Above,
    Below,
}

impl Default for OrderTriggerCondition {
    // UpOnly
    fn default() -> Self {
        OrderTriggerCondition::Above
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderDiscountTier {
    None,
    First,
    Second,
    Third,
    Fourth,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeStructure {
    pub fee: Decimal,

    pub first_tier_minimum_balance: Uint128,
    pub first_tier_discount: Decimal,

    pub second_tier_minimum_balance: Uint128,
    pub second_tier_discount: Decimal,

    pub third_tier_minimum_balance: Uint128,
    pub third_tier_discount: Decimal,

    pub fourth_tier_minimum_balance: Uint128,
    pub fourth_tier_discount: Decimal,


    pub referrer_reward: Decimal,
    pub referee_discount: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleGuardRails {
    pub use_for_liquidations: bool,
    // oracle price divergence rails
    pub mark_oracle_divergence: Decimal,
    // validity guard rails
    pub slots_before_stale: i64,
    pub confidence_interval_max_size: Uint128,
    pub too_volatile_ratio: Number128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OrderParams {
    pub order_type: OrderType,
    pub direction: PositionDirection,
    pub quote_asset_amount: Uint128,
    pub base_asset_amount: Uint128,
    pub price: Uint128,
    pub market_index: u64,
    pub reduce_only: bool,
    pub post_only: bool,
    pub immediate_or_cancel: bool,
    pub trigger_price: Uint128,
    pub trigger_condition: OrderTriggerCondition,
    pub position_limit: Uint128,
    pub oracle_price_offset: Number128,
}

pub fn calculate_price(
    quote_asset_reserve: Uint128,
    base_asset_reserve: Uint128,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    let peg_quote_asset_amount = quote_asset_reserve
        .checked_mul(peg_multiplier)?;

    let res = peg_quote_asset_amount.checked_mul(PRICE_TO_PEG_PRECISION_RATIO)?.checked_div(base_asset_reserve)?;

    Ok(res)
}

pub fn calculate_terminal_price(market: &mut Market) -> Result<Uint128, ContractError> {
    let swap_direction = if market.base_asset_amount.i128() > 0 {
        SwapDirection::Add
    } else {
        SwapDirection::Remove
    };
    let (new_quote_asset_amount, new_base_asset_amount) = calculate_swap_output(
        Uint128::from(market.base_asset_amount.i128().unsigned_abs()),
        market.amm.base_asset_reserve,
        swap_direction,
        market.amm.sqrt_k,
    )?;

    let terminal_price = calculate_price(
        new_quote_asset_amount,
        new_base_asset_amount,
        market.amm.peg_multiplier,
    )?;

    Ok(terminal_price)
}

pub fn calculate_new_mark_twap(
    a: &Amm,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<Uint128, ContractError> {
    let since_last = max(
        1,
        now.checked_sub(a.last_mark_price_twap_ts)
            .ok_or_else(|| (ContractError::MathError))?,
    );
    let from_start = max(
        1,
        a.funding_period
            .checked_sub(since_last)
            .ok_or_else(|| (ContractError::MathError))?,
    );
    let current_price = match precomputed_mark_price {
        Some(mark_price) => mark_price,
        None => get_mark_price(&a)?,
    };

    let new_twap = (calculate_twap(
        current_price.u128() as i128,
        a.last_mark_price_twap.u128() as i128,
        since_last as i128,
        from_start as i128,
    )?).unsigned_abs();

    return Ok(Uint128::from(new_twap));
}

pub fn calculate_new_oracle_price_twap(
    a: &Amm,
    now: u64,
    oracle_price: i128,
) -> Result<i128, ContractError> {
    let since_last = max(
        1,
        now.checked_sub(a.last_oracle_price_twap_ts)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    let from_start = max(
        1 as u64,
        a.funding_period
            .checked_sub(since_last)
            .ok_or_else(|| (ContractError::MathError))?,
        );
    let new_twap = calculate_twap(
        oracle_price,
        a.last_oracle_price_twap.i128(),
        since_last as i128,
        from_start as i128,
    )?;

    return Ok(new_twap);
}

pub fn calculate_twap(
    new_data: i128,
    old_data: i128,
    new_weight: i128,
    old_weight: i128,
) -> Result<i128, ContractError> {
    let denominator = new_weight
        .checked_add(old_weight)
        .ok_or_else(|| (ContractError::MathError))?;
    let prev_twap_99 = old_data.checked_mul(old_weight).ok_or_else(|| (ContractError::MathError))?;
    let latest_price_01 = new_data.checked_mul(new_weight).ok_or_else(|| (ContractError::MathError))?;
    let new_twap = prev_twap_99
        .checked_add(latest_price_01)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(denominator)
        .ok_or_else(|| (ContractError::MathError));
    return new_twap;
}

pub fn calculate_swap_output(
    swap_amount: Uint128,
    input_asset_amount: Uint128,
    direction: SwapDirection,
    invariant_sqrt: Uint128,
) -> Result<(Uint128, Uint128), ContractError> {
    let invariant = invariant_sqrt
        .checked_mul(invariant_sqrt)?;

    if direction == SwapDirection::Remove && swap_amount > input_asset_amount {
        return Err(ContractError::TradeSizeTooLarge);
    }

    let new_input_amount = if let SwapDirection::Add = direction {
        input_asset_amount
            .checked_add(swap_amount)?
    } else {
        input_asset_amount
            .checked_sub(swap_amount)?
    };

    let new_output_amount = invariant
        .checked_div(new_input_amount)?;

    return Ok((new_output_amount, new_input_amount));
}

pub fn calculate_quote_asset_amount_swapped(
    quote_asset_reserve_before: Uint128,
    quote_asset_reserve_after: Uint128,
    swap_direction: SwapDirection,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    let quote_asset_reserve_change = match swap_direction {
        SwapDirection::Add => quote_asset_reserve_before
            .checked_sub(quote_asset_reserve_after)?,

        SwapDirection::Remove => quote_asset_reserve_after
            .checked_sub(quote_asset_reserve_before)?,
    };

    let mut quote_asset_amount =
    reserve_to_asset_amount(quote_asset_reserve_change, peg_multiplier)?;

    // when a user goes long base asset, make the base asset slightly more expensive
    // by adding one unit of quote asset
    if swap_direction == SwapDirection::Remove {
        quote_asset_amount = quote_asset_amount
            .checked_add(Uint128::from(1 as u64))?;
    }

    Ok(quote_asset_amount)
}


pub fn normalise_oracle_price(
    a: &Amm,
    oracle_price: &OraclePriceData,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let OraclePriceData {
        price: oracle_price,
        confidence: oracle_conf,
        ..
    } = *oracle_price;

    let mark_price = match precomputed_mark_price {
        Some(mark_price) => mark_price.u128() as i128,
        None => a.mark_price()?.u128() as i128,
    };

    let mark_price_1bp = mark_price.checked_div(10000).ok_or_else(|| (ContractError::MathError))?;
    let conf_int = oracle_conf.u128() as i128;

    //  normalises oracle toward mark price based on the oracle’s confidence interval
    //  if mark above oracle: use oracle+conf unless it exceeds .9999 * mark price
    //  if mark below oracle: use oracle-conf unless it less than 1.0001 * mark price
    //  (this guarantees more reasonable funding rates in volatile periods)
    let normalised_price = if mark_price > oracle_price.i128() {
        min(
            max(
                mark_price
                    .checked_sub(mark_price_1bp)
                    .ok_or_else(|| (ContractError::MathError))?,
                oracle_price.i128(),
            ),
            oracle_price.i128()
                .checked_add(conf_int)
                .ok_or_else(|| (ContractError::MathError))?,
        )
    } else {
        max(
            min(
                mark_price
                    .checked_add(mark_price_1bp)
                    .ok_or_else(|| (ContractError::MathError))?,
                oracle_price.i128(),
            ),
            oracle_price.i128()
                .checked_sub(conf_int)
                .ok_or_else(|| (ContractError::MathError))?,
        )
    };

    Ok(normalised_price)
}


pub fn calculate_oracle_mark_spread(
    a: &Amm,
    oracle_price_data: &OraclePriceData,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(i128, i128), ContractError> {
    let mark_price = match precomputed_mark_price {
        Some(mark_price) => mark_price.u128() as i128,
        None => a.mark_price()?.u128() as i128,
    };

    let oracle_price = oracle_price_data.price.i128();

    let price_spread = mark_price
        .checked_sub(oracle_price)
        .ok_or_else(|| (ContractError::MathError))?;

    Ok((oracle_price, price_spread))

}

pub fn calculate_oracle_mark_spread_pct(
    a: &Amm,
    oracle_price_data: &OraclePriceData,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let (oracle_price, price_spread) =
        calculate_oracle_mark_spread(a, oracle_price_data, precomputed_mark_price)?;

    price_spread
        .checked_mul(PRICE_SPREAD_PRECISION)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(oracle_price)
        .ok_or_else(|| (ContractError::MathError))
}

pub fn is_oracle_mark_too_divergent(
    price_spread_pct: i128,
    oracle_guard_rails: &OracleGuardRails,
) -> Result<bool, ContractError> {
    let max_divergence = oracle_guard_rails
        .mark_oracle_divergence.numerator()
        .checked_mul(PRICE_SPREAD_PRECISION_U128.u128())
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(oracle_guard_rails.mark_oracle_divergence.denominator())
        .ok_or_else(|| (ContractError::MathError))?;

    // Ok(max_divergence.lt(&Uint128::from(price_spread_pct.unsigned_abs())))
    Ok(Uint128::from(price_spread_pct.unsigned_abs()).gt(&Uint128::from(max_divergence)))
}

pub fn calculate_mark_twap_spread_pct(a: &Amm, mark_price: Uint128) -> Result<i128, ContractError> {
    let mark_price = mark_price.u128() as i128;
    let mark_twap = a.last_mark_price_twap.u128() as i128;

    let price_spread = mark_price
        .checked_sub(mark_twap)
        .ok_or_else(|| (ContractError::MathError))?;

    price_spread
        .checked_mul(PRICE_SPREAD_PRECISION)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(mark_twap)
        .ok_or_else(|| (ContractError::MathError))
}

pub fn use_oracle_price_for_margin_calculation(
    price_spread_pct: i128,
    oracle_guard_rails: &OracleGuardRails,
) -> Result<bool, ContractError> {
    let max_divergence = oracle_guard_rails
        .mark_oracle_divergence.numerator()
        .checked_mul(PRICE_SPREAD_PRECISION_U128.u128())
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(3)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(oracle_guard_rails.mark_oracle_divergence.denominator())
        .ok_or_else(|| (ContractError::MathError))?;

    Ok(price_spread_pct.unsigned_abs() > max_divergence)
}


pub fn is_oracle_valid(
    a: &Amm,
    oracle_price_data: &OraclePriceData,
    valid_oracle_guard_rails: &OracleGuardRails,
) -> Result<bool, ContractError> {
    let OraclePriceData {
        price: oracle_price,
        confidence: oracle_conf,
        delay: oracle_delay,
        has_sufficient_number_of_data_points,
        ..
    } = *oracle_price_data;

    let is_oracle_price_nonpositive = oracle_price.i128() <= 0;

    let is_oracle_price_too_volatile = ((oracle_price.i128()
        .checked_div(max(1, a.last_oracle_price_twap.i128()))
        .ok_or_else(|| (ContractError::MathError))?)
    .gt(&valid_oracle_guard_rails.too_volatile_ratio.i128()))
        || ((a
            .last_oracle_price_twap.i128()
            .checked_div(max(1, oracle_price.i128()))
            .ok_or_else(|| (ContractError::MathError))?)
        .gt(&valid_oracle_guard_rails.too_volatile_ratio.i128()));

    let conf_denom_of_price = Uint128::from(oracle_price.i128().unsigned_abs())
        .checked_div(Uint128::from(max(1 as u128, oracle_conf.u128())))?;

    let is_conf_too_large =
        conf_denom_of_price.lt(&valid_oracle_guard_rails.confidence_interval_max_size);

    let is_stale = oracle_delay.gt(&valid_oracle_guard_rails.slots_before_stale);

    Ok(!(is_stale
        || !has_sufficient_number_of_data_points
        || is_oracle_price_nonpositive
        || is_oracle_price_too_volatile
        || is_conf_too_large))
}

pub fn calculate_max_base_asset_amount_to_trade(
    amm: &Amm,
    limit_price: Uint128,
) -> Result<(Uint128, PositionDirection), ContractError> {
    let invariant = amm.sqrt_k
        .checked_mul(amm.sqrt_k)?;

    let new_base_asset_reserve_squared = invariant
        .checked_mul(MARK_PRICE_PRECISION)?
        .checked_div(limit_price)?
        .checked_mul(amm.peg_multiplier)?
        .checked_div(PEG_PRECISION)?;

    let new_base_asset_reserve = new_base_asset_reserve_squared.u128().sqrt();

    if new_base_asset_reserve > amm.base_asset_reserve.u128() {
        let max_trade_amount = Uint128::from(new_base_asset_reserve)
            .checked_sub(amm.base_asset_reserve)?;
        Ok((max_trade_amount, PositionDirection::Short))
    } else {
        let max_trade_amount = amm
            .base_asset_reserve
            .checked_sub(Uint128::from(new_base_asset_reserve))?;
        Ok((max_trade_amount, PositionDirection::Long))
    }
}

pub fn should_round_trade(
    a: &Amm,
    quote_asset_amount: Uint128,
    base_asset_value: Uint128,
) -> Result<bool, ContractError> {
    let difference = if quote_asset_amount > base_asset_value {
        quote_asset_amount
            .checked_sub(base_asset_value)?
    } else {
        base_asset_value
            .checked_sub(quote_asset_amount)?
    };

    let quote_asset_reserve_amount = asset_to_reserve_amount(difference, a.peg_multiplier)?;

    Ok(quote_asset_reserve_amount < a.minimum_quote_asset_trade_size)
}

pub fn get_mark_price(a: &Amm) -> Result<Uint128, ContractError> {
    calculate_price(
        a.quote_asset_reserve,
        a.base_asset_reserve,
        a.peg_multiplier,
    )
}

pub fn calculate_fee_for_trade(
    quote_asset_amount: Uint128,
    fee_structure: &FeeStructure,
    discount_token_amt: Uint128,
    referrer: &Option<Addr>,
) -> Result<(Uint128, Uint128, Uint128, Uint128, Uint128), ContractError> {
    let fee = quote_asset_amount
        .checked_mul(Uint128::from(fee_structure.fee.numerator()))?
        .checked_div(Uint128::from(fee_structure.fee.denominator()))?;

    let token_discount = calculate_token_discount(fee, fee_structure, discount_token_amt)?;

    let (referrer_reward, referee_discount) =
        calculate_referral_reward_and_referee_discount(fee, fee_structure, referrer)?;

    let user_fee = fee
        .checked_sub(token_discount)?
        .checked_sub(referee_discount)?;

    let fee_to_market = user_fee
        .checked_sub(referrer_reward)?;

    return Ok((
        user_fee,
        fee_to_market,
        token_discount,
        referrer_reward,
        referee_discount,
    ));
}

fn calculate_token_discount(
    fee: Uint128,
    fee_structure: &FeeStructure,
    discount_token_amt: Uint128,
) -> Result<Uint128, ContractError> {
    if discount_token_amt.is_zero() {
        return Ok(Uint128::zero());
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.first_tier_minimum_balance, fee_structure.first_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.second_tier_minimum_balance, fee_structure.second_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.third_tier_minimum_balance, fee_structure.third_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.fourth_tier_minimum_balance, fee_structure.fourth_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }


    Ok(Uint128::zero())
}

fn calculate_token_discount_for_tier(
    fee: Uint128,
    tier_minimum_balance: Uint128,
    discount : Decimal,
    discount_token_amt: Uint128,
) -> Result<Option<Uint128>, ContractError> {
    if belongs_to_tier(tier_minimum_balance, discount_token_amt) {
        return try_calculate_token_discount_for_tier(fee, discount);
    }
    Ok(None)
}

fn try_calculate_token_discount_for_tier(fee: Uint128, discount : Decimal) -> Result<Option<Uint128>, ContractError> {
    let res = fee.checked_mul(Uint128::from(discount.numerator()))?.checked_div(Uint128::from(discount.denominator()))?;
    Ok(Some(res))
}

fn belongs_to_tier(tier_minimum_balance: Uint128, discount_token_amt: Uint128) -> bool {
    discount_token_amt.ge(&tier_minimum_balance)
}

fn calculate_referral_reward_and_referee_discount(
    fee: Uint128,
    fee_structure: &FeeStructure,
    referrer: &Option<Addr>,
) -> Result<(Uint128, Uint128), ContractError> {
    if referrer.is_none() {
        return Ok((Uint128::zero(), Uint128::zero()));
    }

    let referrer_reward = fee
        .checked_mul(Uint128::from(fee_structure.referrer_reward.numerator()))?
        .checked_div(Uint128::from(fee_structure.referrer_reward.denominator()))?;

    let referee_discount = fee
        .checked_mul(Uint128::from(fee_structure.referee_discount.numerator()))?
        .checked_div(Uint128::from(fee_structure.referee_discount.denominator()))?;

    return Ok((referrer_reward, referee_discount));
}


pub fn calculate_order_fee_tier(
    fee_structure: &FeeStructure,
    discount_token_amt: Uint128,
) -> Result<OrderDiscountTier, ContractError> {
    if discount_token_amt.is_zero() {
        return Ok(OrderDiscountTier::None);
    }

    if belongs_to_tier(
        fee_structure.first_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::First);
    }

    if belongs_to_tier(
        fee_structure.second_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::Second);
    }

    if belongs_to_tier(
        fee_structure.third_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::Third);
    }

    if belongs_to_tier(
        fee_structure.fourth_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::Fourth);
    }

    Ok(OrderDiscountTier::None)
}

pub fn calculate_fee_for_order(
    quote_asset_amount: Uint128,
    fee_structure: &FeeStructure,
    filler_reward_structure: &OrderState,
    order_fee_tier: &OrderDiscountTier,
    order_ts: u64,
    now: u64,
    referrer: &Option<Addr>,
    filler_is_user: bool,
    quote_asset_amount_surplus: Uint128,
) -> Result<(Uint128, Uint128, Uint128, Uint128, Uint128, Uint128), ContractError> {
    // if there was a quote_asset_amount_surplus, the order was a maker order and fee_to_market comes from surplus
    if !quote_asset_amount_surplus.is_zero() {
        let fee = quote_asset_amount_surplus;
        let filler_reward: Uint128 = if filler_is_user {
            Uint128::zero()
        } else {
            calculate_filler_reward(fee, order_ts, now, filler_reward_structure)?
        };
        let fee_to_market = fee.checked_sub(filler_reward)?;

        Ok((Uint128::zero(), fee_to_market, Uint128::zero(), filler_reward, Uint128::zero(), Uint128::zero()))
    } else {
        let fee = quote_asset_amount
            .checked_mul(Uint128::from(fee_structure.fee.numerator()))?
            .checked_div(Uint128::from(fee_structure.fee.denominator()))?;

        let token_discount =
            calculate_token_discount_for_limit_order(fee, fee_structure, order_fee_tier)?;

        let (referrer_reward, referee_discount) =
            calculate_referral_reward_and_referee_discount(fee, fee_structure, referrer)?;

        let user_fee = fee
            .checked_sub(referee_discount)?
            .checked_sub(token_discount)?;

        let filler_reward: Uint128 = if filler_is_user {
            Uint128::zero()
        } else {
            calculate_filler_reward(user_fee, order_ts, now, filler_reward_structure)?
        };

        let fee_to_market = user_fee
            .checked_sub(filler_reward)?
            .checked_sub(referrer_reward)?;

        Ok((
            user_fee,
            fee_to_market,
            token_discount,
            filler_reward,
            referrer_reward,
            referee_discount,
        ))
    }
}

fn calculate_token_discount_for_limit_order(
    fee: Uint128,
    fee_structure: &FeeStructure,
    order_discount_tier: &OrderDiscountTier,
) -> Result<Uint128, ContractError> {
    match order_discount_tier {
        OrderDiscountTier::None => Ok(Uint128::zero()),
        OrderDiscountTier::First => {
            try_calculate_token_discount_for_tier(fee, fee_structure.first_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
        OrderDiscountTier::Second => {
            try_calculate_token_discount_for_tier(fee, fee_structure.second_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
        OrderDiscountTier::Third => {
            try_calculate_token_discount_for_tier(fee, fee_structure.third_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
        OrderDiscountTier::Fourth => {
            try_calculate_token_discount_for_tier(fee, fee_structure.fourth_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
    }
}

fn calculate_filler_reward(
    fee: Uint128,
    order_ts: u64,
    now: u64,
    filler_reward_structure: &OrderState,
) -> Result<Uint128, ContractError> {
    // incentivize keepers to prioritize filling older orders (rather than just largest orders)
    // for sufficiently small-sized order, reward based on fraction of fee paid

    let size_filler_reward = fee
        .checked_mul(Uint128::from(filler_reward_structure.reward.numerator()))?
        .checked_div(Uint128::from(filler_reward_structure.reward.denominator()))?;

    let min_time_filler_reward = filler_reward_structure.time_based_reward_lower_bound.u128();
    let time_since_order = max(
        1,
        now.checked_sub(order_ts).ok_or_else(|| (ContractError::MathError))?,
    );
    let time_filler_reward = (time_since_order as u128)
        .checked_mul(100_000_000) // 1e8
        .ok_or_else(|| (ContractError::MathError))?
        .nth_root(4)
        .checked_mul(min_time_filler_reward)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(100) // 1e2 = sqrt(sqrt(1e8))
        .ok_or_else(|| (ContractError::MathError))?;

    // lesser of size-based and time-based reward
    let fee = min(size_filler_reward.u128(), time_filler_reward);

    Ok(Uint128::from(fee))
}

/// With a virtual AMM, there can be an imbalance between longs and shorts and thus funding can be asymmetric.
/// To account for this, amm keeps track of the cumulative funding rate for both longs and shorts.
/// When there is a period with asymmetric funding, the clearing house will pay/receive funding from/to it's collected fees.
pub fn calculate_funding_rate_long_short(
    market: &Market,
    funding_rate: i128,
) -> Result<(i128, i128, Uint128), ContractError> {
    // Calculate the funding payment owed by the net_market_position if funding is not capped
    // If the net market position owes funding payment, the clearing house receives payment
    let net_market_position = market.base_asset_amount.i128().clone();
    let net_market_position_funding_payment =
        calculate_funding_payment_in_quote_precision(funding_rate, net_market_position)?;
    let uncapped_funding_pnl = -net_market_position_funding_payment;

    // If the uncapped_funding_pnl is positive, the clearing house receives money.
    if uncapped_funding_pnl >= 0 {
        let new_total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(Uint128::from(uncapped_funding_pnl.unsigned_abs()))?;
        return Ok((funding_rate, funding_rate, new_total_fee_minus_distributions));
    }

    let (capped_funding_rate, capped_funding_pnl) =
        calculate_capped_funding_rate(&market, uncapped_funding_pnl, funding_rate)?;

    let new_total_fee_minus_distributions = market
        .amm
        .total_fee_minus_distributions
        .checked_sub(Uint128::from(capped_funding_pnl.unsigned_abs()))?;

    // clearing house is paying part of funding imbalance
    if capped_funding_pnl != 0 {
        let total_fee_minus_distributions_lower_bound = market
            .amm
            .total_fee
            .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
            .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?;

        // makes sure the clearing house doesn't pay more than the share of fees allocated to `distributions`
        if new_total_fee_minus_distributions.lt(&total_fee_minus_distributions_lower_bound) {
            return Err(ContractError::InvalidFundingProfitability.into());
        }
    }
    
    let funding_rate_long = if funding_rate < 0 {
        capped_funding_rate
    } else {
        funding_rate
    };

    let funding_rate_short = if funding_rate > 0 {
        capped_funding_rate
    } else {
        funding_rate
    };

    return Ok((funding_rate_long, funding_rate_short, new_total_fee_minus_distributions));
}

fn calculate_capped_funding_rate(
    market: &Market,
    uncapped_funding_pnl: i128, // if negative, users would net recieve from clearinghouse
    funding_rate: i128,
) -> Result<(i128, i128), ContractError> {
    // The funding_rate_pnl_limit is the amount of fees the clearing house can use before it hits it's lower bound
    let total_fee_minus_distributions_lower_bound = market
        .amm
        .total_fee
        .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
        .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?;

    // limit to 2/3 of current fee pool per funding period
    let funding_rate_pnl_limit =
        if market.amm.total_fee_minus_distributions > total_fee_minus_distributions_lower_bound {
            -(market
                    .amm
                    .total_fee_minus_distributions
                    .checked_sub(total_fee_minus_distributions_lower_bound)?
                    .checked_mul(Uint128::from(2 as u32))?
                    .checked_div(Uint128::from(3 as u32))?
                    .u128() as i128)
        } else {
            0
        };

    // if theres enough in fees, give user's uncapped funding
    // if theres a little/nothing in fees, give the user's capped outflow funding
    let capped_funding_pnl = max(uncapped_funding_pnl, funding_rate_pnl_limit);
    let capped_funding_rate = if uncapped_funding_pnl < funding_rate_pnl_limit {
        // Calculate how much funding payment is already available from users
        let funding_payment_from_users = if funding_rate > 0 {
            calculate_funding_payment_in_quote_precision(
                funding_rate,
                market.base_asset_amount_long.i128(),
            )
        } else {
            calculate_funding_payment_in_quote_precision(
                funding_rate,
                market.base_asset_amount_short.i128(),
            )
        }?;

        // increase the funding_rate_pnl_limit by accounting for the funding payment already being made by users
        // this makes it so that the capped rate includes funding payments from users and clearing house collected fees
        let funding_rate_pnl_limit = funding_rate_pnl_limit
            .checked_sub(funding_payment_from_users.abs())
            .ok_or_else(|| (ContractError::MathError))?;

        if funding_rate < 0 {
            // longs receive
            calculate_funding_rate_from_pnl_limit(
                funding_rate_pnl_limit,
                market.base_asset_amount_long.i128(),
            )?
        } else {
            // shorts receive
            calculate_funding_rate_from_pnl_limit(
                funding_rate_pnl_limit,
                market.base_asset_amount_short.i128(),
            )?
        }
    } else {
        funding_rate
    };

    return Ok((capped_funding_rate, capped_funding_pnl));
}

pub fn calculate_funding_payment(
    amm_cumulative_funding_rate: i128,
    market_position: &Position,
) -> Result<i128, ContractError> {
    let funding_rate_delta = amm_cumulative_funding_rate
        .checked_sub(market_position.last_cumulative_funding_rate.i128())
        .ok_or_else(|| (ContractError::MathError))?;

    let funding_rate_payment =
        _calculate_funding_payment(funding_rate_delta, market_position.base_asset_amount.i128())?;

    return Ok(funding_rate_payment);
}

fn _calculate_funding_payment(
    funding_rate_delta: i128,
    base_asset_amount: i128,
) -> Result<i128, ContractError> {
    let funding_rate_delta_sign: i128 = if funding_rate_delta > 0 { 1 } else { -1 };

    let funding_rate_payment_magnitude = funding_rate_delta.unsigned_abs()
            .checked_mul(base_asset_amount.unsigned_abs())
            .ok_or_else(|| (ContractError::MathError))?
            .checked_div(MARK_PRICE_PRECISION.u128())
            .ok_or_else(|| (ContractError::MathError))?
            .checked_div(FUNDING_PAYMENT_PRECISION.u128())
            .ok_or_else(|| (ContractError::MathError))?;

    // funding_rate: longs pay shorts
    let funding_rate_payment_sign: i128 = if base_asset_amount > 0 { -1 } else { 1 };

    let funding_rate_payment = (funding_rate_payment_magnitude as i128)
        .checked_mul(funding_rate_payment_sign)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_mul(funding_rate_delta_sign)
        .ok_or_else(|| (ContractError::MathError))?;

    return Ok(funding_rate_payment);
}

fn calculate_funding_rate_from_pnl_limit(
    pnl_limit: i128,
    base_asset_amount: i128,
) -> Result<i128, ContractError> {
    if base_asset_amount == 0 {
        return Ok(0);
    }

    let pnl_limit_biased = if pnl_limit < 0 {
        pnl_limit.checked_add(1).ok_or_else(|| (ContractError::MathError))?
    } else {
        pnl_limit
    };

    let funding_rate = pnl_limit_biased
        .checked_mul(QUOTE_TO_BASE_AMT_FUNDING_PRECISION.u128() as i128)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(base_asset_amount)
        .ok_or_else(|| (ContractError::MathError));

    return funding_rate;
}

fn calculate_funding_payment_in_quote_precision(
    funding_rate_delta: i128,
    base_asset_amount: i128,
) -> Result<i128, ContractError> {
    let funding_payment = _calculate_funding_payment(funding_rate_delta, base_asset_amount)?;
    let funding_payment_collateral = funding_payment
        .checked_div(AMM_TO_QUOTE_PRECISION_RATIO.u128() as i128)
        .ok_or_else(|| (ContractError::MathError))?;

    Ok(funding_payment_collateral)
}

pub fn block_operation(
    a: &Amm,
    guard_rails: &OracleGuardRails,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(bool, OraclePriceData), ContractError> {
    let OracleStatus {
        price_data: oracle_price_data,
        is_valid: oracle_is_valid,
        mark_too_divergent: is_oracle_mark_too_divergent,
        oracle_mark_spread_pct: _,
    } = get_oracle_status(
        a,
        guard_rails,
        precomputed_mark_price,
    )?;

    let block = !oracle_is_valid || is_oracle_mark_too_divergent;
    Ok((block, oracle_price_data))
}
 
pub fn get_oracle_status(
    a: &Amm,
    guard_rails: &OracleGuardRails,
    precomputed_mark_price: Option<Uint128>,
) -> Result<OracleStatus, ContractError> {
    let oracle_price_data = a.get_oracle_price()?;
    let oracle_is_valid = is_oracle_valid(a, &oracle_price_data, &guard_rails)?;
    let oracle_mark_spread_pct =
        calculate_oracle_mark_spread_pct(a, &oracle_price_data, precomputed_mark_price)?;
    let is_oracle_mark_too_divergent =
        is_oracle_mark_too_divergent(oracle_mark_spread_pct, &guard_rails)?;

    Ok(OracleStatus {
        price_data: oracle_price_data,
        oracle_mark_spread_pct: Number128::new(oracle_mark_spread_pct) ,
        is_valid: oracle_is_valid,
        mark_too_divergent: is_oracle_mark_too_divergent,
    })
}

pub fn calculate_quote_asset_amount_for_maker_order(
    base_asset_amount: Uint128,
    limit_price: Uint128,
) -> Result<Uint128, ContractError> {
    let res = base_asset_amount
    .checked_mul(limit_price)?
    .checked_div(MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO)?;
    Ok(res)
}

pub fn limit_price_satisfied(
    limit_price: Uint128,
    quote_asset_amount: Uint128,
    base_asset_amount: Uint128,
    direction: PositionDirection,
) -> Result<bool, ContractError> {
    let price = quote_asset_amount
        .checked_mul(MARK_PRICE_PRECISION * AMM_TO_QUOTE_PRECISION_RATIO)?
        .checked_div(base_asset_amount)?;

    match direction {
        PositionDirection::Long => {
            if price > limit_price {
                return Ok(false);
            }
        }
        PositionDirection::Short => {
            if price < limit_price {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

/// Calculates how much of withdrawal must come from collateral vault and how much comes from insurance vault
pub fn calculate_withdrawal_amounts(
    amount: Uint128,
    balance_collateral: Uint128,
    balance_insurance: Uint128
) -> Result<(Uint128, Uint128), ContractError> {
    return Ok(
        if balance_collateral.u128() >= amount.u128() {
            (amount, Uint128::zero())
        } else if balance_insurance.u128() > amount.u128() - balance_collateral.u128()
        {
            (balance_collateral, amount.checked_sub(balance_collateral)?)
        } else {
            (balance_collateral, balance_insurance)
        }
    );
}

pub fn calculate_updated_collateral(collateral: Uint128, pnl: i128) -> Result<Uint128, ContractError> {
    return Ok(if pnl.is_negative() && pnl.unsigned_abs() > collateral.u128() {
        Uint128::zero()
    } else if pnl > 0 {
        collateral
            .checked_add(Uint128::from(pnl.unsigned_abs()))?
    } else {
        collateral
            .checked_sub(Uint128::from(pnl.unsigned_abs()))?
    });
}


pub fn calculate_slippage(
    exit_value: Uint128,
    base_asset_amount: Uint128,
    mark_price_before: i128,
) -> Result<i128, ContractError> {
    let amm_exit_price = exit_value
        .checked_mul(MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO)?
        .checked_div(base_asset_amount)?;

    Ok((amm_exit_price.u128() as i128)
        .checked_sub(mark_price_before).unwrap_or(0 as i128))
}

pub fn calculate_slippage_pct(
    slippage: i128,
    mark_price_before: i128,
) -> Result<i128, ContractError> {
    slippage
        .checked_mul(PRICE_SPREAD_PRECISION)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(mark_price_before)
        .ok_or_else(|| (ContractError::MathError))
}

pub fn reserve_to_asset_amount(
    quote_asset_reserve: Uint128,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(quote_asset_reserve
        .checked_mul(peg_multiplier)?
        .checked_div(AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO)?
    )
}

pub fn asset_to_reserve_amount(
    quote_asset_amount: Uint128,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(quote_asset_amount
        .checked_mul(AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO)?
        .checked_div(peg_multiplier)?
    )
}


pub fn calculate_base_asset_value_and_pnl(
    market_position: &Position,
    a: &Amm,
) -> Result<(Uint128, i128), ContractError> {
    return _calculate_base_asset_value_and_pnl(
        market_position.base_asset_amount.i128(),
        market_position.quote_asset_amount,
        a,
    );
}

pub fn _calculate_base_asset_value_and_pnl(
    base_asset_amount: i128,
    quote_asset_amount: Uint128,
    a: &Amm,
) -> Result<(Uint128, i128), ContractError> {
    if base_asset_amount == 0 {
        return Ok((Uint128::zero(), (0 as i128)));
    }

    let swap_direction = swap_direction_to_close_position(base_asset_amount);

    let (new_quote_asset_reserve, _new_base_asset_reserve) = calculate_swap_output(
        Uint128::from(base_asset_amount.unsigned_abs()),
        a.base_asset_reserve,
        swap_direction,
        a.sqrt_k,
    )?;

    let base_asset_value = calculate_quote_asset_amount_swapped(
        a.quote_asset_reserve,
        new_quote_asset_reserve,
        swap_direction,
        a.peg_multiplier,
    )?;

    let pnl = calculate_pnl(base_asset_value, quote_asset_amount, swap_direction)?;

    return Ok((base_asset_value, pnl));
}

pub fn calculate_base_asset_value_and_pnl_with_oracle_price(
    market_position: &Position,
    oracle_price: i128,
) -> Result<(Uint128, i128), ContractError> {
    if market_position.base_asset_amount.i128() == 0 {
        return Ok((Uint128::zero(), 0));
    }

    let swap_direction = swap_direction_to_close_position(market_position.base_asset_amount.i128());

    let oracle_price = if oracle_price > 0 {
        Uint128::from(oracle_price.unsigned_abs())
    } else {
        Uint128::zero()
    };

    let base_asset_value = Uint128::from(market_position
        .base_asset_amount.i128()
        .unsigned_abs())
        .checked_mul(oracle_price)?
        .checked_div(AMM_RESERVE_PRECISION * PRICE_TO_QUOTE_PRECISION_RATIO)?;

    let pnl = calculate_pnl(
        base_asset_value,
        market_position.quote_asset_amount,
        swap_direction,
    )?;

    Ok((Uint128::from(base_asset_value), pnl))
}

pub fn direction_to_close_position(base_asset_amount: i128) -> PositionDirection {
    if base_asset_amount > 0 {
        PositionDirection::Short
    } else {
        PositionDirection::Long
    }
}

pub fn swap_direction_to_close_position(base_asset_amount: i128) -> SwapDirection {
    if base_asset_amount >= 0 {
        SwapDirection::Add
    } else {
        SwapDirection::Remove
    }
}

pub fn calculate_pnl(
    exit_value: Uint128,
    entry_value: Uint128,
    swap_direction_to_close: SwapDirection,
) -> Result<i128, ContractError> {
    let exit_value_i128 =  exit_value.u128() as i128;
    let entry_value_i128 = entry_value.u128() as i128;
    Ok(match swap_direction_to_close {
        SwapDirection::Add => exit_value_i128
            .checked_sub(entry_value_i128).ok_or_else(|| (ContractError::MathError {}))?,
        SwapDirection::Remove => entry_value_i128
            .checked_sub(exit_value_i128).ok_or_else(|| (ContractError::MathError {}))?,
    })
}


pub fn update_mark_twap(
    deps: &mut DepsMut,
    market_index: u64,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<Uint128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mark_twap = calculate_new_mark_twap(&market.amm, now, precomputed_mark_price)?;
    market.amm.last_mark_price_twap = mark_twap;
    market.amm.last_mark_price_twap_ts = now;
    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;
    return Ok(mark_twap);
}

pub fn update_oracle_price_twap(
    deps: &mut DepsMut,
    market_index: u64,
    now: u64,
    oracle_price: i128,
) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mut a = market.amm.clone();
    let new_oracle_price_spread = oracle_price
        .checked_sub(a.last_oracle_price_twap.i128())
        .ok_or_else(|| (ContractError::MathError))?;

    // cap new oracle update to 33% delta from twap
    let oracle_price_33pct = oracle_price.checked_div(3).ok_or_else(|| (ContractError::MathError))?;

    let capped_oracle_update_price =
        if new_oracle_price_spread.unsigned_abs() > oracle_price_33pct.unsigned_abs() {
            if oracle_price > a.last_oracle_price_twap.i128() {
                a.last_oracle_price_twap.i128()
                    .checked_add(oracle_price_33pct)
                    .ok_or_else(|| (ContractError::MathError))?
            } else {
                a.last_oracle_price_twap.i128()
                    .checked_sub(oracle_price_33pct)
                    .ok_or_else(|| (ContractError::MathError))?
            }
        } else {
            oracle_price
        };

    // sanity check
    let oracle_price_twap: i128;
    if capped_oracle_update_price > 0 && oracle_price > 0 {
        oracle_price_twap = calculate_new_oracle_price_twap(&a, now, capped_oracle_update_price)?;
        a.last_oracle_price = Number128::new(capped_oracle_update_price);
        a.last_oracle_price_twap = Number128::new(oracle_price_twap);
        a.last_oracle_price_twap_ts = now;
    } else {
        oracle_price_twap = a.last_oracle_price_twap.i128()
    }

    market.amm = a;
    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    Ok(oracle_price_twap)
}

/// To find the cost of adjusting k, compare the the net market value before and after adjusting k
/// Increasing k costs the protocol money because it reduces slippage and improves the exit price for net market position
/// Decreasing k costs the protocol money because it increases slippage and hurts the exit price for net market position
pub fn adjust_k_cost(deps: &mut DepsMut, market_index: u64, new_sqrt_k: Uint128) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    // Find the net market value before adjusting k
    let (current_net_market_value, _) =
        _calculate_base_asset_value_and_pnl(market.base_asset_amount.i128(), Uint128::zero(), &market.amm)?;

    let ratio_scalar = MARK_PRICE_PRECISION;

    let sqrt_k_ratio = new_sqrt_k
        .checked_mul(ratio_scalar)?
        .checked_div(Uint128::from(market.amm.sqrt_k))?;

    // if decreasing k, max decrease ratio for single transaction is 2.5%
    if sqrt_k_ratio
        < ratio_scalar
            .checked_mul(Uint128::from(975 as u64))?
            .checked_div(Uint128::from(1000 as u64))?
    {
        return Err(ContractError::InvalidUpdateK.into());
    }
    let new_sqrt_k_val= new_sqrt_k;
    let new_base_asset_reserve = Uint128::from(market.amm.base_asset_reserve)
        .checked_mul(sqrt_k_ratio)?
        .checked_div(ratio_scalar)?;

        let new_quote_asset_reserve = market.amm.quote_asset_reserve
        .checked_mul(sqrt_k_ratio)?
        .checked_div(ratio_scalar)?;

    market.amm.sqrt_k = new_sqrt_k_val;
    market.amm.base_asset_reserve = new_base_asset_reserve;
    market.amm.quote_asset_reserve = new_quote_asset_reserve;

    let (_new_net_market_value, cost) = _calculate_base_asset_value_and_pnl(
        market.base_asset_amount.i128(),
        current_net_market_value,
        &market.amm,
    )?;

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    Ok(cost)
}

pub fn swap_quote_asset(
    deps: &mut DepsMut,
    market_index: u64,
    quote_asset_amount: Uint128,
    direction: SwapDirection,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let a = market.amm.clone();
    update_mark_twap(deps, market_index, now, precomputed_mark_price)?;
    let quote_asset_reserve_amount =
        asset_to_reserve_amount(quote_asset_amount, a.peg_multiplier)?;

    if quote_asset_reserve_amount < a.minimum_quote_asset_trade_size {
        return Err(ContractError::TradeSizeTooSmall);
    }

    let initial_base_asset_reserve = a.base_asset_reserve;
    let (new_base_asset_reserve, new_quote_asset_reserve) = calculate_swap_output(
        quote_asset_reserve_amount,
        a.quote_asset_reserve,
        direction,
        a.sqrt_k,
    )?;

    market.amm.base_asset_reserve = new_base_asset_reserve;
    market.amm.quote_asset_reserve = new_quote_asset_reserve;

    let base_asset_amount = (initial_base_asset_reserve.u128() as i128)
        .checked_sub(new_base_asset_reserve.u128() as i128)
        .ok_or_else(|| (ContractError::MathError))?;

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    return Ok(base_asset_amount);
}

pub fn swap_base_asset(
    deps: &mut DepsMut,
    market_index: u64,
    base_asset_swap_amount: Uint128,
    direction: SwapDirection,
    now: u64,
    precomputed_mark_price: Option<Uint128>
) -> Result<Uint128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let a = market.amm.clone();
    
    update_mark_twap(deps, market_index, now, precomputed_mark_price)?;

    let initial_quote_asset_reserve = a.quote_asset_reserve;
    let (new_quote_asset_reserve, new_base_asset_reserve) = calculate_swap_output(
        base_asset_swap_amount,
        a.base_asset_reserve,
        direction,
        a.sqrt_k,
    )?;

    market.amm.base_asset_reserve = new_base_asset_reserve;
    market.amm.quote_asset_reserve = new_quote_asset_reserve;

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    calculate_quote_asset_amount_swapped(
        initial_quote_asset_reserve,
        new_quote_asset_reserve,
        direction,
        a.peg_multiplier,
    )

}

pub fn move_price(
    deps: &mut DepsMut, 
    market_index: u64,
    base_asset_reserve: Uint128,
    quote_asset_reserve: Uint128,
) -> Result<(), ContractError> {
    let k = base_asset_reserve
        .mul(quote_asset_reserve);

    let mut mark = MARKETS.load(deps.storage, market_index.to_string())?;
    
    mark.amm.base_asset_reserve = base_asset_reserve;
    mark.amm.quote_asset_reserve = quote_asset_reserve;
    mark.amm.sqrt_k = Uint128::from(k.u128().sqrt());

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(mark)
    })?;
    Ok(())
}

/// Funding payments are settled lazily. The amm tracks its cumulative funding rate (for longs and shorts)
/// and the user's market position tracks how much funding the user been cumulatively paid for that market.
/// If the two values are not equal, the user owes/is owed funding.
pub fn settle_funding_payment(
    deps: &mut DepsMut,
    user_addr: &Addr,
    now: u64,
) -> Result<(), ContractError> {
    let existing_user = USERS.may_load(deps.storage, &user_addr.clone())?;
    let mut funding_payment: i128 = 0;
    let mut user;
    if existing_user.is_some(){
        user = existing_user.unwrap();
    }
    else{
        return Ok(());
    }
    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(mut m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }
                let market = MARKETS.load(deps.storage, n.to_string())?;
                let amm_cumulative_funding_rate = if m.base_asset_amount.i128() > 0 {
                    market.amm.cumulative_funding_rate_long.i128()
                } else {
                    market.amm.cumulative_funding_rate_short.i128()
                };
                if amm_cumulative_funding_rate != m.last_cumulative_funding_rate.i128() {
                    let market_funding_rate_payment =
                        calculate_funding_payment(amm_cumulative_funding_rate, &m)?;

                    let mut len = LENGTH.load(deps.storage)?;
                    let funding_payment_history_info_length = len.funding_payment_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
                    len.funding_payment_history_length = funding_payment_history_info_length;
                    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
                        Ok(len)
                    })?;
                    FUNDING_PAYMENT_HISTORY.save(
                        deps.storage,
                        (user_addr, funding_payment_history_info_length.to_string()),
                        &FundingPaymentRecord {
                            ts: now,
                            record_id: funding_payment_history_info_length,
                            user: user_addr.clone(),
                            market_index: n,
                            funding_payment: Number128::new(market_funding_rate_payment), //10e13
                            user_last_cumulative_funding: m.last_cumulative_funding_rate, //10e14
                            user_last_funding_rate_ts: m.last_funding_rate_ts,
                            amm_cumulative_funding_long: market.amm.cumulative_funding_rate_long, //10e14
                            amm_cumulative_funding_short: market.amm.cumulative_funding_rate_short, //10e14
                            base_asset_amount: m.base_asset_amount,
                        },
                    )?;
                    funding_payment = funding_payment
                        .checked_add(market_funding_rate_payment)
                        .ok_or_else(|| (ContractError::MathError))?;
        
                    m.last_cumulative_funding_rate = Number128::new(amm_cumulative_funding_rate);
                    m.last_funding_rate_ts = market.amm.last_funding_rate_ts;
        
                    POSITIONS.update(
                        deps.storage,
                        (user_addr, n.to_string()),
                        |_p| -> Result<Position, ContractError> { Ok(m) },
                    )?;
                }
            }
            Err(_) => continue, 
        }
        
    }

    let funding_payment_collateral = funding_payment
        .checked_div(AMM_TO_QUOTE_PRECISION_RATIO_I128.u128() as i128)
        .ok_or_else(|| (ContractError::MathError))?;

    user.collateral = calculate_updated_collateral(user.collateral, funding_payment_collateral)?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok(())
}

pub fn update_funding_rate(
    deps: &mut DepsMut,
    market_index: u64,
    now: u64,
    funding_paused: bool,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(), ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;

    let time_since_last_update = now
        .checked_sub(market.amm.last_funding_rate_ts)
        .ok_or_else(|| (ContractError::MathError))?;

    // Pause funding if oracle is invalid or if mark/oracle spread is too divergent
    let (block_funding_rate_update, oracle_price_data) = block_operation(
        &market.amm,
        &guard_rails,
        precomputed_mark_price,
    )?;

    let normalised_oracle_price =
        normalise_oracle_price(&market.amm, &oracle_price_data, precomputed_mark_price)?;

    // round next update time to be available on the hour
    let mut next_update_wait = market.amm.funding_period;
    if market.amm.funding_period > 1 {
        let last_update_delay = market
            .amm
            .last_funding_rate_ts
            .rem_euclid(market.amm.funding_period);
        if last_update_delay != 0 {
            let max_delay_for_next_period = market
                .amm
                .funding_period
                .checked_div(3)
                .ok_or_else(|| (ContractError::MathError))?;
            if last_update_delay > max_delay_for_next_period {
                // too late for on the hour next period, delay to following period
                next_update_wait = market
                    .amm
                    .funding_period
                    .checked_mul(2)
                    .ok_or_else(|| (ContractError::MathError))?
                    .checked_sub(last_update_delay)
                    .ok_or_else(|| (ContractError::MathError))?;
            } else {
                // allow update on the hour
                next_update_wait = market
                    .amm
                    .funding_period
                    .checked_sub(last_update_delay)
                    .ok_or_else(|| (ContractError::MathError))?;
            }
        }
    }

    if !funding_paused && !block_funding_rate_update && time_since_last_update >= next_update_wait {
        let oracle_price_twap =
            update_oracle_price_twap(deps, market_index, now, normalised_oracle_price)?;
        let mark_price_twap = update_mark_twap(deps, market_index, now, None)?;

        let one_hour_i64 = ONE_HOUR.u128() as i64;
        let period_adjustment = (24_i64)
            .checked_mul(one_hour_i64)
            .ok_or_else(|| (ContractError::MathError))?
            .checked_div(max(one_hour_i64, market.amm.funding_period as i64))
            .ok_or_else(|| (ContractError::MathError))?;

        // funding period = 1 hour, window = 1 day
        // low periodicity => quickly updating/settled funding rates => lower funding rate payment per interval
        let price_spread = (mark_price_twap.u128()  as i128)
            .checked_sub(oracle_price_twap).ok_or_else(|| (ContractError::MathError))?;

        let funding_rate = price_spread
            .checked_mul(FUNDING_PAYMENT_PRECISION.u128() as i128)
            .ok_or_else(|| (ContractError::MathError))?
            .checked_div(period_adjustment as i128)
            .ok_or_else(|| (ContractError::MathError))?;

        let (funding_rate_long, funding_rate_short, new_total_fee_minus_distributions) =
            calculate_funding_rate_long_short(&market, funding_rate)?;

        market.amm.total_fee_minus_distributions = new_total_fee_minus_distributions;

        market.amm.cumulative_funding_rate_long = Number128::new(market
            .amm
            .cumulative_funding_rate_long.i128()
            .checked_add(funding_rate_long)
            .ok_or_else(|| (ContractError::MathError))?);

        market.amm.cumulative_funding_rate_short = Number128::new(market
            .amm
            .cumulative_funding_rate_short.i128()
            .checked_add(funding_rate_short)
            .ok_or_else(|| (ContractError::MathError))?);

        market.amm.last_funding_rate = Number128::new(funding_rate);
        market.amm.last_funding_rate_ts = now;

        MARKETS.update(
            deps.storage,
            market_index.to_string(),
            |_m| -> Result<Market, ContractError> { Ok(market.clone()) },
        )?;

        let mut len = LENGTH.load(deps.storage)?;
        let funding_rate_history_info_length = len.funding_rate_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
        len.funding_rate_history_length = funding_rate_history_info_length;
        LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
            Ok(len)
        })?;
        FUNDING_RATE_HISTORY.save(
            deps.storage,
            funding_rate_history_info_length.to_string(),
            &FundingRateRecord {
                ts: now,
                record_id: funding_rate_history_info_length,
                market_index,
                funding_rate: Number128::new(funding_rate),
                cumulative_funding_rate_long: market.amm.cumulative_funding_rate_long,
                cumulative_funding_rate_short: market.amm.cumulative_funding_rate_short,
                mark_price_twap,
                oracle_price_twap: Number128::new(oracle_price_twap),
            },
        )?;
    };

    Ok(())
}

pub fn meets_initial_margin_requirement(
    deps: &mut DepsMut,
    user_addr: &Addr,
) -> Result<bool, ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;

    let mut initial_margin_requirement: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }
                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;
                let (position_base_asset_value, position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;
                initial_margin_requirement = initial_margin_requirement
                    .checked_add(
                        position_base_asset_value
                            .checked_mul(market.margin_ratio_initial.into())?,
                    )?;

                unrealized_pnl = unrealized_pnl
                    .checked_add(position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;
            },
            Err(_) => continue,
        }
    }

    initial_margin_requirement = initial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;

    Ok(total_collateral.u128() >= initial_margin_requirement.u128())
}

pub fn meets_partial_margin_requirement(
    deps: &DepsMut,
    user_addr: &Addr,
) -> Result<bool, ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;

    let mut partial_margin_requirement: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }
                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;

                let (position_base_asset_value, position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;
                partial_margin_requirement = partial_margin_requirement
                    .checked_add(
                        position_base_asset_value
                            .checked_mul(market.margin_ratio_partial.into())?,
                    )?;

                unrealized_pnl = unrealized_pnl
                    .checked_add(position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;
            }
            Err(_) => continue,
        }
    }

    partial_margin_requirement = partial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;

    Ok(total_collateral >= partial_margin_requirement)
}

pub fn calculate_free_collateral(
    deps: &DepsMut,
    user_addr: &Addr,
    market_to_close: Option<u64>,
) -> Result<(Uint128, Uint128), ContractError> {
    let mut closed_position_base_asset_value: Uint128 = Uint128::zero();
    let mut initial_margin_requirement: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;

    let user = USERS.load(deps.storage, user_addr)?;

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }

                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;
                let (position_base_asset_value, position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;

                if market_to_close.is_some() && market_to_close.unwrap() == n
                {
                    closed_position_base_asset_value = position_base_asset_value;
                } else {
                    initial_margin_requirement = initial_margin_requirement
                        .checked_add(
                            position_base_asset_value
                                .checked_mul(market.margin_ratio_initial.into())?,
                        )?;
                }

                unrealized_pnl = unrealized_pnl
                    .checked_add(position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;
            }
            Err(_) => continue,
        }
    }

    initial_margin_requirement = initial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;

    let free_collateral = if initial_margin_requirement < total_collateral {
        total_collateral
            .checked_sub(initial_margin_requirement)?
    } else {
        Uint128::zero()
    };

    Ok((free_collateral, closed_position_base_asset_value))
}

pub fn calculate_liquidation_status(
    deps: &DepsMut,
    user_addr: &Addr,
) -> Result<LiquidationStatus, ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;

    let mut partial_margin_requirement: Uint128 = Uint128::zero();
    let mut maintenance_margin_requirement: Uint128 = Uint128::zero();
    let mut base_asset_value: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;
    let mut adjusted_unrealized_pnl: i128 = 0;
    let mut market_statuses: Vec<MarketStatus> = Vec::new();

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }

                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;
                let (amm_position_base_asset_value, amm_position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;

                base_asset_value = base_asset_value
                    .checked_add(amm_position_base_asset_value)?;
                unrealized_pnl = unrealized_pnl
                    .checked_add(amm_position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;

                // Block the liquidation if the oracle is invalid or the oracle and mark are too divergent
                let mark_price_before = market.amm.mark_price()?;

                let oracle_status = get_oracle_status(
                    &market.amm,
                    &oracle_guard_rails,
                    Some(mark_price_before),
                )?;

                let market_partial_margin_requirement: Uint128;
                let market_maintenance_margin_requirement: Uint128;
                let mut close_position_slippage = None;
                if oracle_status.is_valid
                    && use_oracle_price_for_margin_calculation(
                        oracle_status.oracle_mark_spread_pct.i128(),
                        &oracle_guard_rails,
                    )?
                {
                    let exit_slippage = calculate_slippage(
                        amm_position_base_asset_value,
                        Uint128::from( m.base_asset_amount.i128().unsigned_abs()),
                        mark_price_before.u128() as i128,
                    )?;
                    close_position_slippage = Some(exit_slippage);

                    let oracle_exit_price = oracle_status
                        .price_data
                        .price.i128()
                        .checked_add(exit_slippage)
                        .ok_or_else(|| (ContractError::HelpersError))?;

                    let (oracle_position_base_asset_value, oracle_position_unrealized_pnl) =
                        calculate_base_asset_value_and_pnl_with_oracle_price(
                            &m,
                            oracle_exit_price,
                        )?;

                    let oracle_provides_better_pnl =
                        oracle_position_unrealized_pnl > amm_position_unrealized_pnl;
                    if oracle_provides_better_pnl {
                        adjusted_unrealized_pnl = adjusted_unrealized_pnl
                            .checked_add(oracle_position_unrealized_pnl)
                            .ok_or_else(|| (ContractError::HelpersError))?;

                        market_partial_margin_requirement = (oracle_position_base_asset_value)
                            .checked_mul(market.margin_ratio_partial.into())?;

                        partial_margin_requirement = partial_margin_requirement
                            .checked_add(market_partial_margin_requirement)?;

                        market_maintenance_margin_requirement = oracle_position_base_asset_value
                            .checked_mul(market.margin_ratio_maintenance.into())?;

                        maintenance_margin_requirement = maintenance_margin_requirement
                            .checked_add(market_maintenance_margin_requirement)?;
                    } else {
                        adjusted_unrealized_pnl = adjusted_unrealized_pnl
                            .checked_add(amm_position_unrealized_pnl)
                            .ok_or_else(|| (ContractError::HelpersError))?;

                        market_partial_margin_requirement = (amm_position_base_asset_value)
                            .checked_mul(market.margin_ratio_partial.into())?;

                        partial_margin_requirement = partial_margin_requirement
                            .checked_add(market_partial_margin_requirement)?;

                        market_maintenance_margin_requirement = amm_position_base_asset_value
                            .checked_mul(market.margin_ratio_maintenance.into())?;

                        maintenance_margin_requirement = maintenance_margin_requirement
                            .checked_add(market_maintenance_margin_requirement)?;
                    }
                } else {
                    adjusted_unrealized_pnl = adjusted_unrealized_pnl
                        .checked_add(amm_position_unrealized_pnl)
                        .ok_or_else(|| (ContractError::HelpersError))?;

                    market_partial_margin_requirement = (amm_position_base_asset_value)
                        .checked_mul(market.margin_ratio_partial.into())?;

                    partial_margin_requirement = partial_margin_requirement
                        .checked_add(market_partial_margin_requirement)?;

                    market_maintenance_margin_requirement = amm_position_base_asset_value
                        .checked_mul(market.margin_ratio_maintenance.into())?;

                    maintenance_margin_requirement = maintenance_margin_requirement
                        .checked_add(market_maintenance_margin_requirement)?;
                }

                market_statuses.push(MarketStatus {
                    market_index: n,
                    partial_margin_requirement: market_partial_margin_requirement.div(MARGIN_PRECISION),
                    maintenance_margin_requirement: market_maintenance_margin_requirement
                        .div(MARGIN_PRECISION),
                    base_asset_value: amm_position_base_asset_value,
                    mark_price_before,
                    oracle_status,
                    close_position_slippage,
                });
            }
            Err(_) => todo!(),
        }
    }

    partial_margin_requirement = partial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    maintenance_margin_requirement = maintenance_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;
    let adjusted_total_collateral =
        calculate_updated_collateral(user.collateral, adjusted_unrealized_pnl)?;

    let requires_partial_liquidation = adjusted_total_collateral < partial_margin_requirement;
    let requires_full_liquidation = adjusted_total_collateral < maintenance_margin_requirement;

    let liquidation_type = if requires_full_liquidation {
        LiquidationType::FULL
    } else if requires_partial_liquidation {
        LiquidationType::PARTIAL
    } else {
        LiquidationType::NONE
    };

    let margin_requirement = match liquidation_type {
        LiquidationType::FULL => maintenance_margin_requirement,
        LiquidationType::PARTIAL => partial_margin_requirement,
        LiquidationType::NONE => partial_margin_requirement,
    };

    // Sort the market statuses such that we close the markets with biggest margin requirements first
    if liquidation_type == LiquidationType::FULL {
        market_statuses.sort_by(|a, b| {
            b.maintenance_margin_requirement
                .cmp(&a.maintenance_margin_requirement)
        });
    } else if liquidation_type == LiquidationType::PARTIAL {
        market_statuses.sort_by(|a, b| {
            b.partial_margin_requirement
                .cmp(&a.partial_margin_requirement)
        });
    }

    let margin_ratio = if base_asset_value.is_zero() {
        Uint128::MAX
    } else {
        total_collateral
            .checked_mul(MARGIN_PRECISION)?
            .checked_div(base_asset_value)?
    };

    Ok(LiquidationStatus {
        liquidation_type,
        margin_requirement,
        total_collateral,
        unrealized_pnl,
        adjusted_total_collateral,
        base_asset_value,
        market_statuses,
        margin_ratio,
    })
}
pub fn validate_margin(
    margin_ratio_initial: u32,
    margin_ratio_partial: u32,
    margin_ratio_maintenance: u32,
) -> Result<bool, ContractError> {
    if !(MINIMUM_MARGIN_RATIO.u128()..=MAXIMUM_MARGIN_RATIO.u128()).contains(&(margin_ratio_initial as u128)) {
        return Err(ContractError::InvalidMarginRatio);
    }

    if margin_ratio_initial < margin_ratio_partial {
        return Err(ContractError::InvalidMarginRatio);
    }

    if !(MINIMUM_MARGIN_RATIO.u128()..=MAXIMUM_MARGIN_RATIO.u128()).contains(&(margin_ratio_partial as u128)) {
        return Err(ContractError::InvalidMarginRatio);
    }

    if margin_ratio_partial < margin_ratio_maintenance {
        return Err(ContractError::InvalidMarginRatio);
    }

    if !(MINIMUM_MARGIN_RATIO.u128()..=MAXIMUM_MARGIN_RATIO.u128()).contains(&(margin_ratio_maintenance as u128)) {
        return Err(ContractError::InvalidMarginRatio);
    }

    Ok(true)
}

pub fn increase(
    deps: &mut DepsMut,
    direction: PositionDirection,
    quote_asset_amount: Uint128,
    market_index: u64,
    user_addr: &Addr,
    position_index: u64,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;
    if quote_asset_amount.is_zero() {
        return Ok(0 as i128);
    }

    // Update funding rate if this is a new position
    if market_position.base_asset_amount.i128() == 0 {
        market_position.last_cumulative_funding_rate = match direction {
            PositionDirection::Long => market.amm.cumulative_funding_rate_long,
            PositionDirection::Short => market.amm.cumulative_funding_rate_short,
        };

        market.open_interest = market.open_interest.checked_add(Uint128::from(1 as u128))?;
    }

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_add(quote_asset_amount)?;

    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Add,
        PositionDirection::Short => SwapDirection::Remove,
    };

    let base_asset_acquired = swap_quote_asset(
        deps,
        market_index,
        quote_asset_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    // update the position size on market and user
    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_acquired)
            .ok_or_else(|| (ContractError::MathError))?,
    );
    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_acquired)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_acquired)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_acquired)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, market_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    Ok(base_asset_acquired)
}

pub fn reduce(
    deps: &mut DepsMut,
    direction: PositionDirection,
    quote_asset_swap_amount: Uint128,
    user_addr: &Addr,
    market_index: u64,
    position_index: u64,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let mut user = USERS.load(deps.storage, user_addr)?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;
    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Add,
        PositionDirection::Short => SwapDirection::Remove,
    };

    let base_asset_swapped = swap_quote_asset(
        deps,
        market_index,
        quote_asset_swap_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    let base_asset_amount_before = market_position.base_asset_amount;
    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_swapped)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() == 0 {
        market.open_interest = market.open_interest.checked_sub(Uint128::from(1 as u128))?;
    }

    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_swapped)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_swapped)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_swapped)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    let base_asset_amount_change = base_asset_amount_before
        .i128()
        .checked_sub(market_position.base_asset_amount.i128())
        .ok_or_else(|| (ContractError::MathError))?
        .abs();

    let initial_quote_asset_amount_closed = market_position
        .quote_asset_amount
        .checked_mul(Uint128::from(base_asset_amount_change.unsigned_abs()))?
        .checked_div(Uint128::from(
            base_asset_amount_before.i128().unsigned_abs(),
        ))?;

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_sub(initial_quote_asset_amount_closed)?;

    let pnl = if market_position.base_asset_amount.i128() > 0 {
        (quote_asset_swap_amount.u128() as i128)
            .checked_sub(initial_quote_asset_amount_closed.u128() as i128)
            .ok_or_else(|| (ContractError::MathError))?
    } else {
        (initial_quote_asset_amount_closed.checked_sub(quote_asset_swap_amount)?).u128() as i128
    };

    user.collateral = calculate_updated_collateral(user.collateral, pnl)?;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok(base_asset_swapped)
}

pub fn close(
    deps: &mut DepsMut,
    user_addr: &Addr,
    market_index: u64,
    position_index: u64,
    now: u64,
    maker_limit_price: Option<Uint128>,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(Uint128, i128, Uint128), ContractError> {
    let mut user = USERS.load(deps.storage, user_addr)?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;
    // If user has no base asset, return early
    if market_position.base_asset_amount.i128() == 0 {
        return Ok((Uint128::zero(), 0, Uint128::zero()));
    }

    let swap_direction = if market_position.base_asset_amount.i128() > 0 {
        SwapDirection::Add
    } else {
        SwapDirection::Remove
    };

    let quote_asset_swapped = swap_base_asset(
        deps,
        market_index,
        Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    let (quote_asset_amount, quote_asset_amount_surplus) = match maker_limit_price {
        Some(limit_price) => calculate_quote_asset_amount_surplus(
            swap_direction,
            quote_asset_swapped,
            Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
            limit_price,
        )?,
        None => (quote_asset_swapped, Uint128::zero()),
    };

    let pnl = calculate_pnl(
        quote_asset_swapped,
        market_position.quote_asset_amount,
        swap_direction,
    )?;

    user.collateral = calculate_updated_collateral(user.collateral, pnl)?;
    market_position.last_cumulative_funding_rate = Number128::zero();
    market_position.last_funding_rate_ts = 0;

    market.open_interest = market.open_interest.checked_sub(Uint128::from(1 as u128))?;

    market_position.quote_asset_amount = Uint128::zero();

    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_sub(market_position.base_asset_amount.i128())
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_sub(market_position.base_asset_amount.i128())
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_sub(market_position.base_asset_amount.i128())
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    let base_asset_amount = market_position.base_asset_amount.i128();
    market_position.base_asset_amount = Number128::zero();

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok((
        quote_asset_amount,
        base_asset_amount,
        quote_asset_amount_surplus,
    ))
}

pub fn add_new_position(
    deps: &mut DepsMut,
    user_addr: &Addr,
    market_index: u64,
) -> Result<u64, ContractError> {

    let new_market_position = Position {
        market_index,
        base_asset_amount: Number128::zero(),
        quote_asset_amount: Uint128::zero(),
        last_cumulative_funding_rate: Number128::zero(),
        last_cumulative_repeg_rebate: Uint128::zero(),
        last_funding_rate_ts: 0,
        order_length: 0,
    };

    POSITIONS.update(
        deps.storage,
        (user_addr, market_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(new_market_position) },
    )?;

    Ok(market_index)
}

pub fn increase_with_base_asset_amount(
    deps: &mut DepsMut,
    direction: PositionDirection,
    base_asset_amount: Uint128,
    user_addr: &Addr,
    position_index: u64,
    now: u64,
    maker_limit_price: Option<Uint128>,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(Uint128, Uint128), ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;

    let market_index = position_index;

    if base_asset_amount.is_zero() {
        return Ok((Uint128::zero(), Uint128::zero()));
    }

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    // Update funding rate if this is a new position
    if market_position.base_asset_amount.i128() == 0 {
        market_position.last_cumulative_funding_rate = match direction {
            PositionDirection::Long => market.amm.cumulative_funding_rate_long,
            PositionDirection::Short => market.amm.cumulative_funding_rate_short,
        };

        market.open_interest = market.open_interest.checked_add(Uint128::from(1 as u64))?;
    }

    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Remove,
        PositionDirection::Short => SwapDirection::Add,
    };

    let quote_asset_swapped = swap_base_asset(
        deps,
        market_index,
        base_asset_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    let (quote_asset_amount, quote_asset_amount_surplus) = match maker_limit_price {
        Some(limit_price) => calculate_quote_asset_amount_surplus(
            swap_direction,
            quote_asset_swapped,
            base_asset_amount,
            limit_price,
        )?,
        None => (quote_asset_swapped, Uint128::zero()),
    };

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_add(quote_asset_amount)?;

    let base_asset_amount = match direction {
        PositionDirection::Long => (base_asset_amount.u128() as i128),
        PositionDirection::Short => -(base_asset_amount.u128() as i128),
    };

    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );
    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok((quote_asset_amount, quote_asset_amount_surplus))
}

pub fn reduce_with_base_asset_amount(
    deps: &mut DepsMut,
    direction: PositionDirection,
    base_asset_amount: Uint128,
    user_addr: &Addr,
    position_index: u64,
    now: u64,
    maker_limit_price: Option<Uint128>,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(Uint128, Uint128), ContractError> {
    let mut user = USERS.load(deps.storage, user_addr)?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;

    let market_index = position_index;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Remove,
        PositionDirection::Short => SwapDirection::Add,
    };

    let quote_asset_swapped = swap_base_asset(
        deps,
        market_index,
        base_asset_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    let (quote_asset_amount, quote_asset_amount_surplus) = match maker_limit_price {
        Some(limit_price) => calculate_quote_asset_amount_surplus(
            swap_direction,
            quote_asset_swapped,
            base_asset_amount,
            limit_price,
        )?,
        None => (quote_asset_swapped, Uint128::zero()),
    };

    let base_asset_amount = match direction {
        PositionDirection::Long => (base_asset_amount.u128() as i128),
        PositionDirection::Short => -(base_asset_amount.u128() as i128),
    };

    let base_asset_amount_before = market_position.base_asset_amount.i128();
    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() == 0 {
        market.open_interest = market.open_interest.checked_sub(Uint128::from(1 as u128))?;
    }

    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    let base_asset_amount_change = base_asset_amount_before
        .checked_sub(market_position.base_asset_amount.i128())
        .ok_or_else(|| (ContractError::MathError))?
        .abs();

    let initial_quote_asset_amount_closed = market_position
        .quote_asset_amount
        .checked_mul(Uint128::from(base_asset_amount_change.unsigned_abs()))?
        .checked_div(Uint128::from(base_asset_amount_before.unsigned_abs()))?;

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_sub(initial_quote_asset_amount_closed)?;

    let pnl = if PositionDirection::Short == direction {
        (quote_asset_amount.u128() as i128)
            .checked_sub(initial_quote_asset_amount_closed.u128() as i128)
            .ok_or_else(|| (ContractError::MathError))?
    } else {
        (initial_quote_asset_amount_closed.u128() as i128)
            .checked_sub(quote_asset_amount.u128() as i128)
            .ok_or_else(|| (ContractError::MathError))?
    };

    user.collateral = calculate_updated_collateral(user.collateral, pnl)?;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok((quote_asset_amount, quote_asset_amount_surplus))
}

pub fn update_position_with_base_asset_amount(
    deps: &mut DepsMut,
    base_asset_amount: Uint128,
    direction: PositionDirection,
    user_addr: &Addr,
    position_index: u64,
    mark_price_before: Uint128,
    now: u64,
    maker_limit_price: Option<Uint128>,
) -> Result<(bool, bool, Uint128, Uint128, Uint128), ContractError> {
    let market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;

    let market_index = position_index;

    // A trade is risk increasing if it increases the users leverage
    // If a trade is risk increasing and brings the user's margin ratio below initial requirement
    // the trade fails
    // If a trade is risk increasing and it pushes the mark price too far away from the oracle price
    // the trade fails
    let mut potentially_risk_increasing = true;
    let mut reduce_only = false;

    // The trade increases the the user position if
    // 1) the user does not have a position
    // 2) the trade is in the same direction as the user's existing position
    let quote_asset_amount;
    let quote_asset_amount_surplus;
    let increase_position = market_position.base_asset_amount.i128() == 0
        || market_position.base_asset_amount.i128() > 0 && direction == PositionDirection::Long
        || market_position.base_asset_amount.i128() < 0 && direction == PositionDirection::Short;
    if increase_position {
        let (_quote_asset_amount, _quote_asset_amount_surplus) = increase_with_base_asset_amount(
            deps,
            direction,
            base_asset_amount,
            user_addr,
            position_index,
            now,
            maker_limit_price,
            Some(mark_price_before),
        )?;
        quote_asset_amount = _quote_asset_amount;
        quote_asset_amount_surplus = _quote_asset_amount_surplus;
    } else if market_position.base_asset_amount.i128().unsigned_abs() > base_asset_amount.u128() {
        let (_quote_asset_amount, _quote_asset_amount_surplus) = reduce_with_base_asset_amount(
            deps,
            direction,
            base_asset_amount,
            user_addr,
            position_index,
            now,
            maker_limit_price,
            Some(mark_price_before),
        )?;
        quote_asset_amount = _quote_asset_amount;
        quote_asset_amount_surplus = _quote_asset_amount_surplus;

        reduce_only = true;
        potentially_risk_increasing = false;
    } else {
        // after closing existing position, how large should trade be in opposite direction
        let base_asset_amount_after_close = base_asset_amount.checked_sub(Uint128::from(
            market_position.base_asset_amount.i128().unsigned_abs(),
        ))?;

        // If the value of the new position is less than value of the old position, consider it risk decreasing
        if base_asset_amount_after_close.u128()
            < market_position.base_asset_amount.i128().unsigned_abs()
        {
            potentially_risk_increasing = false;
        }

        let (quote_asset_amount_closed, _, quote_asset_amount_surplus_closed) = close(
            deps,
            user_addr,
            market_index,
            position_index,
            now,
            maker_limit_price,
            Some(mark_price_before),
        )?;

        let (quote_asset_amount_opened, quote_asset_amount_surplus_opened) =
            increase_with_base_asset_amount(
                deps,
                direction,
                base_asset_amount_after_close,
                user_addr,
                position_index,
                now,
                maker_limit_price,
                Some(mark_price_before),
            )?;

        // means position was closed and it was reduce only
        if quote_asset_amount_opened.is_zero() {
            reduce_only = true;
        }

        quote_asset_amount = quote_asset_amount_closed.checked_add(quote_asset_amount_opened)?;

        quote_asset_amount_surplus =
            quote_asset_amount_surplus_closed.checked_add(quote_asset_amount_surplus_opened)?;
    }

    Ok((
        potentially_risk_increasing,
        reduce_only,
        base_asset_amount,
        quote_asset_amount,
        quote_asset_amount_surplus,
    ))
}

pub fn update_position_with_quote_asset_amount(
    deps: &mut DepsMut,
    quote_asset_amount: Uint128,
    direction: PositionDirection,
    user_addr: &Addr,
    position_index: u64,
    mark_price_before: Uint128,
    now: u64,
) -> Result<(bool, bool, Uint128, Uint128, Uint128), ContractError> {
    let market_position;
    let existing_position =
        POSITIONS.may_load(deps.storage, (&user_addr.clone(), position_index.to_string()))?;
    match existing_position {
        Some(exp) => {
            market_position = exp;
        }
        None => {
            market_position = Position {
                market_index: position_index,
                base_asset_amount: Number128::zero(),
                quote_asset_amount: Uint128::zero(),
                last_cumulative_funding_rate: Number128::zero(),
                last_cumulative_repeg_rebate: Uint128::zero(),
                last_funding_rate_ts: 0,
                order_length: 0,
            };
            POSITIONS.save(
                deps.storage,
                (&user_addr.clone(), position_index.to_string()),
                &market_position,
            )?;
        }
    }
    let market_index = market_position.market_index;
    let market = MARKETS.load(deps.storage, market_index.to_string())?;

    // A trade is risk increasing if it increases the users leverage
    // If a trade is risk increasing and brings the user's margin ratio below initial requirement
    // the trade fails
    // If a trade is risk increasing and it pushes the mark price too far away from the oracle price
    // the trade fails
    let mut potentially_risk_increasing = true;
    let mut reduce_only = false;

    let mut quote_asset_amount = quote_asset_amount;
    let base_asset_amount;
    // The trade increases the the user position if
    // 1) the user does not have a position
    // 2) the trade is in the same direction as the user's existing position
    let increase_position = market_position.base_asset_amount.i128() == 0
        || market_position.base_asset_amount.i128() > 0 && direction == PositionDirection::Long
        || market_position.base_asset_amount.i128() < 0 && direction == PositionDirection::Short;
    if increase_position {
        base_asset_amount = increase(
            deps,
            direction,
            quote_asset_amount,
            market_index,
            &user_addr.clone(),
            position_index,
            now,
            Some(mark_price_before),
        )?
        .unsigned_abs();
    } else {
        let (base_asset_value, _unrealized_pnl) =
            calculate_base_asset_value_and_pnl(&market_position, &market.amm)?;

        // if the quote_asset_amount is close enough in value to base_asset_value,
        // round the quote_asset_amount to be the same as base_asset_value
        if should_round_trade(&market.amm, quote_asset_amount, base_asset_value)? {
            quote_asset_amount = base_asset_value;
        }

        // we calculate what the user's position is worth if they closed to determine
        // if they are reducing or closing and reversing their position
        if base_asset_value > quote_asset_amount {
            base_asset_amount = reduce(
                deps,
                direction,
                quote_asset_amount,
                &user_addr.clone(),
                market_index,
                position_index,
                now,
                Some(mark_price_before),
            )?
            .unsigned_abs();

            potentially_risk_increasing = false;
            reduce_only = true;
        } else {
            // after closing existing position, how large should trade be in opposite direction
            let quote_asset_amount_after_close =
                quote_asset_amount.checked_sub(base_asset_value)?;

            // If the value of the new position is less than value of the old position, consider it risk decreasing
            if quote_asset_amount_after_close < base_asset_value {
                potentially_risk_increasing = false;
            }

            let (_, base_asset_amount_closed, _) = close(
                deps,
                &user_addr.clone(),
                market_index,
                position_index,
                now,
                None,
                Some(mark_price_before),
            )?;
            let base_asset_amount_closed = base_asset_amount_closed.unsigned_abs();

            let base_asset_amount_opened = increase(
                deps,
                direction,
                quote_asset_amount_after_close,
                market_index,
                &user_addr.clone(),
                position_index,
                now,
                Some(mark_price_before),
            )?
            .unsigned_abs();

            // means position was closed and it was reduce only
            if base_asset_amount_opened == 0 {
                reduce_only = true;
            }

            base_asset_amount = base_asset_amount_closed
                .checked_add(base_asset_amount_opened)
                .ok_or_else(|| (ContractError::MathError))?;
        }
    }

    Ok((
        potentially_risk_increasing,
        reduce_only,
        Uint128::from(base_asset_amount),
        quote_asset_amount,
        Uint128::zero(),
    ))
}

fn calculate_quote_asset_amount_surplus(
    swap_direction: SwapDirection,
    quote_asset_swapped: Uint128,
    base_asset_amount: Uint128,
    limit_price: Uint128,
) -> Result<(Uint128, Uint128), ContractError> {
    let quote_asset_amount =
        calculate_quote_asset_amount_for_maker_order(base_asset_amount, limit_price)?;

    let quote_asset_amount_surplus = match swap_direction {
        SwapDirection::Remove => quote_asset_amount.checked_sub(quote_asset_swapped)?,
        SwapDirection::Add => quote_asset_swapped.checked_sub(quote_asset_amount)?,
    };

    Ok((quote_asset_amount, quote_asset_amount_surplus))
}


pub fn repeg(
    deps: &mut DepsMut,
    market_index: u64,
    new_peg_candidate: Uint128
) -> Result<i128, ContractError> {

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;

    if new_peg_candidate == market.amm.peg_multiplier {
        return Err(ContractError::InvalidRepegRedundant.into());
    }

    let terminal_price_before = calculate_terminal_price(&mut market)?;
    let adjustment_cost = adjust_peg_cost(&mut market, new_peg_candidate)?;

    
    market.amm.peg_multiplier = new_peg_candidate;

    let oracle_price_data = market.amm.get_oracle_price()?;	
    let oracle_price = oracle_price_data.price.i128();	
    let oracle_conf = oracle_price_data.confidence;
    let oracle_is_valid =	
        is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;	
    
    // if oracle is valid: check on size/direction of repeg
    if oracle_is_valid {
        let terminal_price_after = calculate_terminal_price(&mut market)?;

        let mark_price_after = calculate_price(
            market.amm.quote_asset_reserve,
            market.amm.base_asset_reserve,
            market.amm.peg_multiplier,
        )?;

        let oracle_conf_band_top = Uint128::from(oracle_price.unsigned_abs())
            .checked_add(oracle_conf)?;

        let oracle_conf_band_bottom = Uint128::from(oracle_price.unsigned_abs())
            .checked_sub(oracle_conf)?;

        if oracle_price.unsigned_abs() > terminal_price_after.u128() {
            // only allow terminal up when oracle is higher
            if terminal_price_after < terminal_price_before {
                return Err(ContractError::InvalidRepegDirection.into());
            }

            // only push terminal up to top of oracle confidence band
            if oracle_conf_band_bottom < terminal_price_after {
                return Err(ContractError::InvalidRepegProfitability.into());
            }

            // only push mark up to top of oracle confidence band
            if mark_price_after > oracle_conf_band_top {
                return Err(ContractError::InvalidRepegProfitability.into());
            }
        }

        if oracle_price.unsigned_abs() < terminal_price_after.u128() {
            // only allow terminal down when oracle is lower
            if terminal_price_after > terminal_price_before {
                return Err(ContractError::InvalidRepegDirection.into());
            }

            // only push terminal down to top of oracle confidence band
            if oracle_conf_band_top > terminal_price_after {
                return Err(ContractError::InvalidRepegProfitability.into());
            }

            // only push mark down to bottom of oracle confidence band
            if mark_price_after < oracle_conf_band_bottom {
                return Err(ContractError::InvalidRepegProfitability.into());
            }
        }
    }

    // Reduce pnl to quote asset precision and take the absolute value
    if adjustment_cost > 0 {
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_sub(Uint128::from(adjustment_cost.unsigned_abs()))?;

        // Only a portion of the protocol fees are allocated to repegging
        // This checks that the total_fee_minus_distributions does not decrease too much after repeg
        if market.amm.total_fee_minus_distributions
            < market
                .amm
                .total_fee
                .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
                .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?
        {
            return Err(ContractError::InvalidRepegProfitability.into());
        }
    } else {
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(Uint128::from(adjustment_cost.unsigned_abs()))?;
    }

    MARKETS.update(deps.storage, market_index.to_string(), |_m| ->  Result<Market, ContractError>{
        Ok(market)
    })?;

    Ok(adjustment_cost)

}

pub fn adjust_peg_cost(market: &mut Market, new_peg: Uint128) -> Result<i128, ContractError> {
    // Find the net market value before adjusting peg
    let (current_net_market_value, _) =
        _calculate_base_asset_value_and_pnl(market.base_asset_amount.i128(), Uint128::zero(), &market.amm)?;

    market.amm.peg_multiplier = new_peg;

    let (_new_net_market_value, cost) = _calculate_base_asset_value_and_pnl(
        market.base_asset_amount.i128(),
        current_net_market_value,
        &market.amm,
    )?;

    Ok(cost)
}

pub fn try_initialize_market(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    market_index: u64,
    market_name: String,
    amm_base_asset_reserve: Uint128,
    amm_quote_asset_reserve: Uint128,
    amm_periodicity: u64,
    amm_peg_multiplier: Uint128,
    oracle_source: OracleSource,
    margin_ratio_initial: u32,
    margin_ratio_partial: u32,
    margin_ratio_maintenance: u32,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &_info.sender.clone())?;
    let now = env.block.time.seconds();

    let state = STATE.load(deps.storage)?;

    let existing_market = MARKETS.load(deps.storage, market_index.to_string());
    if existing_market.is_ok() {
        return Err(ContractError::MarketIndexAlreadyInitialized {});
    }
    if amm_base_asset_reserve != amm_quote_asset_reserve {
        return Err(ContractError::InvalidInitialPeg.into());
    }

    let init_mark_price = calculate_price(
        amm_quote_asset_reserve,
        amm_base_asset_reserve,
        amm_peg_multiplier,
    )?;

    let a = Amm {
        oracle: state.oracle,
        oracle_source,
        base_asset_reserve: amm_base_asset_reserve,
        quote_asset_reserve: amm_quote_asset_reserve,
        cumulative_repeg_rebate_long: Uint128::zero(),
        cumulative_repeg_rebate_short: Uint128::zero(),
        cumulative_funding_rate_long: Number128::zero(),
        cumulative_funding_rate_short: Number128::zero(),
        last_funding_rate: Number128::zero(),
        last_funding_rate_ts: now,
        funding_period: amm_periodicity,
        last_oracle_price_twap: Number128::zero(),
        last_mark_price_twap: init_mark_price,
        last_mark_price_twap_ts: now,
        sqrt_k: amm_base_asset_reserve,
        peg_multiplier: amm_peg_multiplier,
        total_fee: Uint128::zero(),
        total_fee_minus_distributions: Uint128::zero(),
        total_fee_withdrawn: Uint128::zero(),
        minimum_quote_asset_trade_size: Uint128::from(10000000 as u128),
        last_oracle_price_twap_ts: now,
        last_oracle_price: Number128::zero(),
        minimum_base_asset_trade_size: Uint128::from(10000000 as u128),
    };

    // Verify there's no overflow
    let _k = amm_base_asset_reserve.checked_mul(amm_quote_asset_reserve)?;

    let OraclePriceData {
        // price: oracle_price,
        ..
    } = a.get_oracle_price()?;

    // let last_oracle_price_twap = a.get_oracle_twap()?;

    validate_margin(
        margin_ratio_initial,
        margin_ratio_partial,
        margin_ratio_maintenance,
    )?;
    let market = Market {
        market_name: market_name,
        initialized: true,
        base_asset_amount_long: Number128::zero(),
        base_asset_amount_short: Number128::zero(),
        base_asset_amount: Number128::zero(),
        open_interest: Uint128::zero(),
        margin_ratio_initial, // unit is 20% (+2 decimal places)
        margin_ratio_partial,
        margin_ratio_maintenance,
        amm: a,
    };
    MARKETS.save(deps.storage, market_index.to_string(), &market)?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.markets_length += 1;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_initialize_market"))
}


pub fn try_move_amm_price(
    mut deps: DepsMut,
    base_asset_reserve: Uint128,
    quote_asset_reserve: Uint128,
    market_index: u64,
) -> Result<Response, ContractError> {
    move_price(
        &mut deps,
        market_index,
        base_asset_reserve,
        quote_asset_reserve,
    )?;
    Ok(Response::new().add_attribute("method", "try_move_amm_price"))
}

pub fn try_withdraw_fees(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    amount: u64,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let state = STATE.load(deps.storage)?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    // A portion of fees must always remain in protocol to be used to keep markets optimal
    let max_withdraw = market
        .amm
        .total_fee
        .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
        .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?
        .checked_sub(market.amm.total_fee_withdrawn)?;

    if amount as u128 > max_withdraw.u128() {
        return Err(ContractError::AdminWithdrawTooLarge.into());
    }

    //todo recipient who? is it only admin function
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.collateral_vault.to_string(),
        msg: to_binary(&VaultInterface::Withdraw {
            to_address: info.sender.clone(),
            amount: amount as u128,
        })?,
        funds: vec![],
    });

    market.amm.total_fee_withdrawn = market
        .amm
        .total_fee_withdrawn
        .checked_add(Uint128::from(amount))?;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_withdraw_fees"))
}

pub fn try_withdraw_from_insurance_vault_to_market(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    amount: u64,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let state = STATE.load(deps.storage)?;

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    market.amm.total_fee_minus_distributions = market
        .amm
        .total_fee_minus_distributions
        .checked_add(Uint128::from(amount))?;

    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.insurance_vault.to_string(),
        msg: to_binary(&VaultInterface::Withdraw {
            to_address: state.collateral_vault.clone(),
            amount: amount as u128,
        })?,
        funds: vec![],
    });
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_withdraw_from_insurance_vault_to_market"))
}

pub fn try_repeg_amm_curve(
    mut deps: DepsMut,
    env: Env,
    new_peg_candidate: Uint128,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let market = MARKETS.load(deps.storage, market_index.to_string())?;
    let OraclePriceData {
        price: oracle_price,
        ..
    } = market.amm.get_oracle_price()?;
    let peg_multiplier_before = market.amm.peg_multiplier;
    let base_asset_reserve_before = market.amm.base_asset_reserve;
    let quote_asset_reserve_before = market.amm.quote_asset_reserve;
    let sqrt_k_before = market.amm.sqrt_k;

    // let price_oracle = state.oracle;

    let adjustment_cost =
        repeg(&mut deps, market_index, new_peg_candidate).unwrap();
    let peg_multiplier_after = market.amm.peg_multiplier;
    let base_asset_reserve_after = market.amm.base_asset_reserve;
    let quote_asset_reserve_after = market.amm.quote_asset_reserve;
    let sqrt_k_after = market.amm.sqrt_k;

    let mut len = LENGTH.load(deps.storage)?;
    let curve_history_info_length = len.curve_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
    len.curve_history_length = curve_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    CURVEHISTORY.save(
        deps.storage,
        curve_history_info_length.to_string(),
        &CurveRecord {
            ts: now,
            record_id: curve_history_info_length,
            market_index,
            peg_multiplier_before,
            base_asset_reserve_before,
            quote_asset_reserve_before,
            sqrt_k_before,
            peg_multiplier_after,
            base_asset_reserve_after,
            quote_asset_reserve_after,
            sqrt_k_after,
            base_asset_amount_long: Uint128::from(
                market.base_asset_amount_long.i128().unsigned_abs(),
            ),
            base_asset_amount_short: Uint128::from(
                market.base_asset_amount_short.i128().unsigned_abs(),
            ),
            base_asset_amount: market.base_asset_amount,
            open_interest: market.open_interest,
            total_fee: market.amm.total_fee,
            total_fee_minus_distributions: market.amm.total_fee_minus_distributions,
            adjustment_cost: Number128::new(adjustment_cost),
            oracle_price
        },
    )?;
    Ok(Response::new().add_attribute("method", "try_repeg_amm_curve"))
}

pub fn try_update_amm_oracle_twap(
    deps: DepsMut,
    env: Env,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    // todo get_oracle_twap is not defined yet
    let oracle_twap = market.amm.get_oracle_twap()?;

    if let Some(oracle_twap) = oracle_twap {
        let oracle_mark_gap_before = (market.amm.last_mark_price_twap.u128() as i128)
            .checked_sub(market.amm.last_oracle_price_twap.i128())
            .ok_or_else(|| (ContractError::MathError))?;

        let oracle_mark_gap_after = (market.amm.last_mark_price_twap.u128() as i128)
            .checked_sub(oracle_twap)
            .ok_or_else(|| (ContractError::MathError))?;

        if (oracle_mark_gap_after > 0 && oracle_mark_gap_before < 0)
            || (oracle_mark_gap_after < 0 && oracle_mark_gap_before > 0)
        {
            market.amm.last_oracle_price_twap =
                Number128::new(market.amm.last_mark_price_twap.u128() as i128);
            market.amm.last_oracle_price_twap_ts = now;
        } else if oracle_mark_gap_after.unsigned_abs() <= oracle_mark_gap_before.unsigned_abs() {
            market.amm.last_oracle_price_twap = Number128::new(oracle_twap);
            market.amm.last_oracle_price_twap_ts = now;
        } else {
            return Err(ContractError::OracleMarkSpreadLimit.into());
        }
    } else {
        return Err(ContractError::InvalidOracle.into());
    }

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    Ok(Response::new().add_attribute("method", "try_update_amm_oracle_twap"))
}

pub fn try_reset_amm_oracle_twap(
    deps: DepsMut,
    env: Env,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let oracle_price_data = market.amm.get_oracle_price()?;

    let is_oracle_valid =
        is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;

    if !is_oracle_valid {
        market.amm.last_oracle_price_twap =
            Number128::new(market.amm.last_mark_price_twap.u128() as i128);
        market.amm.last_oracle_price_twap_ts = now;
    }
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new().add_attribute("method", "try_reset_amm_oracle_twap"))
}

pub fn try_update_funding_rate(
    mut deps: DepsMut,
    env: Env,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let funding_paused = STATE.load(deps.storage).unwrap().funding_paused;
    update_funding_rate(
        &mut deps,
        market_index,
        now,
        funding_paused,
        None,
    )?;
    Ok(Response::new().add_attribute("method", "try_update_funding_rate"))
}

pub fn try_update_k(
    mut deps: DepsMut,
    env: Env,
    market_index: u64,
    sqrt_k: Uint128,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    let base_asset_amount_long = Uint128::from(market.base_asset_amount_long.i128().unsigned_abs());
    let base_asset_amount_short =
        Uint128::from(market.base_asset_amount_short.i128().unsigned_abs());
    let base_asset_amount = market.base_asset_amount.i128().clone();
    let open_interest = market.open_interest.clone();

    let price_before = calculate_price(
        market.amm.quote_asset_reserve,
        market.amm.base_asset_reserve,
        market.amm.peg_multiplier,
    )?;

    let peg_multiplier_before = market.amm.peg_multiplier;
    let base_asset_reserve_before = market.amm.base_asset_reserve;
    let quote_asset_reserve_before = market.amm.quote_asset_reserve;
    let sqrt_k_before = market.amm.sqrt_k;

    let adjustment_cost = adjust_k_cost(&mut deps, market_index, sqrt_k)?;

    if adjustment_cost > 0 {
        let max_cost = market
            .amm
            .total_fee_minus_distributions
            .checked_sub(market.amm.total_fee_withdrawn)?;
        if adjustment_cost.unsigned_abs() > max_cost.u128() {
            return Err(ContractError::InvalidUpdateK.into());
        } else {
            market.amm.total_fee_minus_distributions = market
                .amm
                .total_fee_minus_distributions
                .checked_sub(Uint128::from(adjustment_cost.unsigned_abs()))?;
        }
    } else {
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(Uint128::from(adjustment_cost.unsigned_abs()))?;
    }

    let amm = &market.amm;
    let price_after = calculate_price(
        amm.quote_asset_reserve,
        amm.base_asset_reserve,
        amm.peg_multiplier,
    )?;

    let price_change_too_large = (price_before.u128() as i128)
        .checked_sub(price_after.u128() as i128)
        .ok_or_else(|| ContractError::MathError {})?
        .unsigned_abs()
        .gt(&UPDATE_K_ALLOWED_PRICE_CHANGE.u128());

    if price_change_too_large {
        return Err(ContractError::InvalidUpdateK.into());
    }

    let peg_multiplier_after = amm.peg_multiplier;
    let base_asset_reserve_after = amm.base_asset_reserve;
    let quote_asset_reserve_after = amm.quote_asset_reserve;
    let sqrt_k_after = amm.sqrt_k;

    let total_fee = amm.total_fee;
    let total_fee_minus_distributions = amm.total_fee_minus_distributions;
    let mut len = LENGTH.load(deps.storage)?;
    let curve_history_info_length = len.curve_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
    len.curve_history_length = curve_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;

    let OraclePriceData {
        price: oracle_price,
        ..
    } = market.amm.get_oracle_price()?;

    CURVEHISTORY.save(
        deps.storage,
        curve_history_info_length.to_string(),
        &CurveRecord {
            ts: now,
            record_id: curve_history_info_length,
            market_index,
            peg_multiplier_before,
            base_asset_reserve_before,
            quote_asset_reserve_before,
            sqrt_k_before,
            peg_multiplier_after,
            base_asset_reserve_after,
            quote_asset_reserve_after,
            sqrt_k_after,
            base_asset_amount_long,
            base_asset_amount_short,
            base_asset_amount: Number128::new(base_asset_amount),
            open_interest,
            adjustment_cost: Number128::new(adjustment_cost),
            total_fee,
            total_fee_minus_distributions,
            oracle_price
        },
    )?;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_k"))
}

pub fn try_update_margin_ratio(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    margin_ratio_initial: u32,
    margin_ratio_partial: u32,
    margin_ratio_maintenance: u32,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    validate_margin(
        margin_ratio_initial,
        margin_ratio_partial,
        margin_ratio_maintenance,
    )?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> {
            market.margin_ratio_initial = margin_ratio_initial;
            market.margin_ratio_partial = margin_ratio_partial;
            market.margin_ratio_maintenance = margin_ratio_maintenance;
            Ok(market)
        },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_margin_ratio"))
}

pub fn try_update_partial_liquidation_close_percentage(
    deps: DepsMut,
    info: MessageInfo,
    value: Decimal,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.partial_liquidation_close_percentage = value;
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_update_partial_liquidation_close_percentage"))
}

pub fn try_update_partial_liquidation_penalty_percentage(
    deps: DepsMut,
    info: MessageInfo,
    value: Decimal,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.partial_liquidation_penalty_percentage = value;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute(
        "method",
        "try_update_partial_liquidation_penalty_percentage",
    ))
}

pub fn try_update_full_liquidation_penalty_percentage(
    deps: DepsMut,
    info: MessageInfo,
    value: Decimal,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.full_liquidation_penalty_percentage = value;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_full_liquidation_penalty_percentage"))
}

pub fn try_update_partial_liquidation_liquidator_share_denominator(
    deps: DepsMut,
    info: MessageInfo,
    denominator: u64,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.partial_liquidation_liquidator_share_denominator = denominator;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute(
        "method",
        "try_update_partial_liquidation_liquidator_share_denominator",
    ))
}

pub fn try_update_full_liquidation_liquidator_share_denominator(
    deps: DepsMut,
    info: MessageInfo,
    denominator: u64,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.full_liquidation_liquidator_share_denominator = denominator;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute(
        "method",
        "try_update_full_liquidation_liquidator_share_denominator",
    ))
}

pub fn try_update_fee(
    deps: DepsMut,
    info: MessageInfo,
    fee: Decimal,
    first_tier_minimum_balance: Uint128,
    first_tier_discount: Decimal,
    second_tier_minimum_balance: Uint128,
    second_tier_discount: Decimal,
    third_tier_minimum_balance: Uint128,
    third_tier_discount: Decimal,
    fourth_tier_minimum_balance: Uint128,
    fourth_tier_discount: Decimal,
    referrer_reward: Decimal,
    referee_discount: Decimal,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let fee_structure = FeeStructure {
        fee,
        first_tier_minimum_balance,
        second_tier_minimum_balance,
        third_tier_minimum_balance,
        fourth_tier_minimum_balance,
        first_tier_discount,
        second_tier_discount,
        third_tier_discount,
        fourth_tier_discount,
        referrer_reward,
        referee_discount,
    };
    FEESTRUCTURE.update(
        deps.storage,
        |mut _f| -> Result<FeeStructure, ContractError> { Ok(fee_structure) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_fee"))
}

pub fn try_update_order_state_structure(
    deps: DepsMut,
    info: MessageInfo,
    min_order_quote_asset_amount: Uint128,
    reward: Decimal,
    time_based_reward_lower_bound: Uint128,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let order_state = OrderState {
        min_order_quote_asset_amount,
        reward,
        time_based_reward_lower_bound,
    };
    ORDERSTATE.update(deps.storage, |mut _s| -> Result<OrderState, ContractError> {
        Ok(order_state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_order_filler_reward_structure"))
}

pub fn try_update_market_oracle(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    oracle: String,
    oracle_source: OracleSource,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    market.amm.oracle = addr_validate_to_lower(deps.api, &oracle)?;
    market.amm.oracle_source = oracle_source;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_market_oracle"))
}

pub fn try_update_oracle_guard_rails(
    deps: DepsMut,
    info: MessageInfo,
    use_for_liquidations: bool,
    mark_oracle_divergence: Decimal,
    slots_before_stale: i64,
    confidence_interval_max_size: Uint128,
    too_volatile_ratio: i128,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let oracle_gr = OracleGuardRails {
        use_for_liquidations,
        mark_oracle_divergence,
        slots_before_stale,
        confidence_interval_max_size,
        too_volatile_ratio: Number128::new(too_volatile_ratio),
    };
    ORACLEGUARDRAILS.update(
        deps.storage,
        |mut _o| -> Result<OracleGuardRails, ContractError> { Ok(oracle_gr) },
    )?;

    Ok(Response::new().add_attribute("method", "try_update_oracle_guard_rails"))
}

pub fn try_update_max_deposit(
    deps: DepsMut,
    info: MessageInfo,
    max_deposit: Uint128,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.max_deposit = max_deposit;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_max_deposit"))
}

pub fn try_update_exchange_paused(
    deps: DepsMut,
    info: MessageInfo,
    exchange_paused: bool,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.exchange_paused = exchange_paused;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_exchange_paused"))
}

pub fn try_disable_admin_control_prices(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.admin_controls_prices = false;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_disable_admin_control_prices"))
}
pub fn try_update_funding_paused(
    deps: DepsMut,
    info: MessageInfo,
    funding_paused: bool,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.funding_paused = funding_paused;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_funding_paused"))
}

pub fn try_update_market_minimum_quote_asset_trade_size(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    minimum_trade_size: Uint128,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> {
            market.amm.minimum_quote_asset_trade_size = minimum_trade_size;
            Ok(market)
        },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_market_minimum_quote_asset_trade_size"))
}

pub fn try_update_market_minimum_base_asset_trade_size(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    minimum_trade_size: Uint128,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    // let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |m| -> Result<_, ContractError> {
            match m {
                Some(mut mr) => {
                    mr.amm.minimum_base_asset_trade_size = minimum_trade_size;
                    Ok(mr)
                },
                None => {
                    return Err(ContractError::UserMaxDeposit)
                },
            }
        },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_market_minimum_base_asset_trade_size"))
}

pub fn try_update_oracle_address(
    deps: DepsMut,
    info: MessageInfo,
    oracle: String,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let mut state = STATE.load(deps.storage)?;
    state.oracle = addr_validate_to_lower(deps.api, &oracle)?;
    STATE.update(deps.storage, |_state| -> Result<State, ContractError> {
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_oracle_address"))
}

pub fn try_feeding_price(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    price: i128,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    market.amm.last_oracle_price = Number128::new(price);
    market.amm.last_oracle_price_twap = Number128::new(price);
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_oracle_address"))
}

pub fn try_deposit_collateral(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: u64,
    referrer: Option<String>,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();
    let existing_user = USERS.may_load(deps.storage, &user_address)?;
    let now = env.block.time.seconds();
    let mut user: User;
    if existing_user.is_some() {
        // user = existing_user.unwrap();
        user = existing_user.unwrap();
    } else {
        if referrer.is_some() {
            user = User {
                collateral: Uint128::zero(),
                cumulative_deposits: Uint128::zero(),
                total_fee_paid: Uint128::zero(),
                total_token_discount: Uint128::zero(),
                total_referral_reward: Uint128::zero(),
                total_referee_discount: Uint128::zero(),
                referrer: Some(addr_validate_to_lower(deps.api, &referrer.unwrap())?),
            };
        } else {
            user = User {
                collateral: Uint128::zero(),
                cumulative_deposits: Uint128::zero(),
                total_fee_paid: Uint128::zero(),
                total_token_discount: Uint128::zero(),
                total_referral_reward: Uint128::zero(),
                total_referee_discount: Uint128::zero(),
                referrer: None,
            };
        }
    }

    if amount == 0 {
        return Err(ContractError::InsufficientDeposit.into());
    }

    assert_sent_uusd_balance(&info.clone(), amount as u128)?;
    let state = STATE.load(deps.storage)?;

    let collateral_before = user.collateral;
    let cumulative_deposits_before = user.cumulative_deposits;
    user.collateral = user.collateral.checked_add(Uint128::from(amount as u128))?;
    user.cumulative_deposits = user.cumulative_deposits.checked_add(amount.into())?;
    if state.max_deposit.u128() > 0 && user.cumulative_deposits.u128() > state.max_deposit.u128() {
        return Err(ContractError::UserMaxDeposit.into());
    }
    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_m| -> Result<User, ContractError> { Ok(user) },
    )?;

    settle_funding_payment(&mut deps, &user_address, now)?;
    //get and send tokens to collateral vault
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.insurance_vault.to_string(),
        msg: to_binary(&VaultInterface::Deposit {})?,
        funds: coins(amount.into(), "uusd"),
    });
    
    let mut len = LENGTH.load(deps.storage)?;
    let deposit_history_info_length = len.deposit_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
    len.deposit_history_length = deposit_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    DEPOSIT_HISTORY.save(
        deps.storage,
        (user_address.clone(), deposit_history_info_length.to_string()),
        &DepositRecord {
            ts: now,
            record_id: deposit_history_info_length,
            user: user_address.clone(),
            direction: DepositDirection::DEPOSIT,
            collateral_before,
            cumulative_deposits_before,
            amount: amount,
        },
    )?;

    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_deposit_collateral"))
}

pub fn try_withdraw_collateral(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: u64,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();
    let existing_user = USERS.may_load(deps.storage, &user_address)?;
    let now = env.block.time.seconds();
    let mut user;
    if existing_user.is_none() {
        return Err(ContractError::UserDoesNotExist);
    } else {
        user = existing_user.unwrap();
    }
    let collateral_before = user.collateral;
    let cumulative_deposits_before = user.cumulative_deposits;

    settle_funding_payment(&mut deps, &user_address, now)?;
    user = USERS.may_load(deps.storage, &user_address)?.unwrap();

    if (amount as u128) > user.collateral.u128() {
        return Err(ContractError::InsufficientCollateral.into());
    }

    let state = STATE.load(deps.storage)?;
    let collateral_balance = query_balance(&deps.querier, state.collateral_vault.clone())?;
    let insurance_balance = query_balance(&deps.querier, state.insurance_vault.clone())?;
    let (collateral_account_withdrawal, insurance_account_withdrawal) =
        calculate_withdrawal_amounts(
            Uint128::from(amount as u128),
            Uint128::from(collateral_balance),
            Uint128::from(insurance_balance),
        )?;

    // amount_withdrawn can be less than amount if there is an insufficient balance in collateral and insurance vault
    let amount_withdraw =
        collateral_account_withdrawal.checked_add(insurance_account_withdrawal)?;

    user.cumulative_deposits = user
        .cumulative_deposits
        .checked_sub(Uint128::from(amount_withdraw))?;

    user.collateral = user
        .collateral
        .checked_sub(Uint128::from(collateral_account_withdrawal))?
        .checked_sub(Uint128::from(insurance_account_withdrawal))?;

    if !meets_initial_margin_requirement(&mut deps, &info.sender.clone())? {
        return Err(ContractError::InsufficientCollateral.into());
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.collateral_vault.clone().to_string(),
        msg: to_binary(&VaultInterface::Withdraw {
            to_address: info.sender.clone(),
            amount: collateral_account_withdrawal.u128(),
        })?,
        funds: vec![],
    }));

    if insurance_account_withdrawal.gt(&Uint128::zero()) {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.insurance_vault.to_string(),
            msg: to_binary(&VaultInterface::Withdraw {
                to_address: info.sender.clone(),
                amount: insurance_account_withdrawal.u128(),
            })?,
            funds: vec![],
        }));
    }

    let mut len = LENGTH.load(deps.storage)?;
    let deposit_history_info_length = len.deposit_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
    len.deposit_history_length = deposit_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    DEPOSIT_HISTORY.save(
        deps.storage,
        (user_address.clone(), deposit_history_info_length.to_string()),
        &DepositRecord {
            ts: now,
            record_id: deposit_history_info_length,
            user: user_address.clone(),
            direction: DepositDirection::WITHDRAW,
            collateral_before,
            cumulative_deposits_before,
            amount: amount_withdraw.u128() as u64,
        },
    )?;
    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "try_withdraw_collateral"))
}

pub fn try_open_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    direction: PositionDirection,
    quote_asset_amount: Uint128,
    market_index: u64,
    limit_price: Option<Uint128>,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();
    
    let now = env.block.time.seconds();
    let state = STATE.load(deps.storage)?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let fee_structure = FEESTRUCTURE.load(deps.storage)?;

    if quote_asset_amount.is_zero() {
        return Err(ContractError::TradeSizeTooSmall.into());
    }
    settle_funding_payment(&mut deps, &user_address, now)?;

    let position_index = market_index.clone();
    let mark_price_before: Uint128;
    let oracle_mark_spread_pct_before: i128;
    let is_oracle_valid_bool: bool;

    {
        let market = MARKETS.load(deps.storage, market_index.to_string())?;
        mark_price_before = market.amm.mark_price()?;
        let oracle_price_data = market.amm.get_oracle_price()?;
        oracle_mark_spread_pct_before = calculate_oracle_mark_spread_pct(
            &market.amm,
            &oracle_price_data,
            Some(mark_price_before),
        )?;
        is_oracle_valid_bool =
            is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;
        if is_oracle_valid_bool {
            let normalised_oracle_price = normalise_oracle_price(
                &market.amm,
                &oracle_price_data,
                Some(mark_price_before),
            )?;
            update_oracle_price_twap(
                &mut deps,
                market_index,
                now,
                normalised_oracle_price,
            )?;
        }
    }

    let potentially_risk_increasing;
    let base_asset_amount;
    let mut quote_asset_amount = quote_asset_amount;

    {
        let (_potentially_risk_increasing, _, _base_asset_amount, _quote_asset_amount, _) =
            update_position_with_quote_asset_amount(
                &mut deps,
                quote_asset_amount,
                direction,
                &user_address,
                position_index,
                mark_price_before,
                now,
            )?;

        potentially_risk_increasing = _potentially_risk_increasing;
        base_asset_amount = _base_asset_amount;
        quote_asset_amount = _quote_asset_amount;
    }
    let mut user = USERS.load(deps.storage, &user_address)?;
    let mark_price_after: Uint128;
    let oracle_price_after: i128;
    let oracle_mark_spread_pct_after: i128;
    {
        let market = MARKETS.load(deps.storage, market_index.to_string())?;
        mark_price_after = market.amm.mark_price()?;
        let oracle_price_data = market.amm.get_oracle_price()?;
        oracle_mark_spread_pct_after = calculate_oracle_mark_spread_pct(
            &market.amm,
            &oracle_price_data,
            Some(mark_price_after),
        )?;
        oracle_price_after = oracle_price_data.price.i128();
    }

    let meets_initial_margin_requirement =
        meets_initial_margin_requirement(&mut deps, &user_address)?;
    if !meets_initial_margin_requirement && potentially_risk_increasing {
        return Err(ContractError::InsufficientCollateral.into());
    }

    // todo add referrer and discount token
    let referrer = user.referrer.clone();
    let discount_token = Uint128::zero();
    let (user_fee, fee_to_market, token_discount, referrer_reward, referee_discount) =
        calculate_fee_for_trade(
            quote_asset_amount,
            &fee_structure,
            discount_token,
            &referrer,
        )?;

    {
        let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
        market.amm.total_fee = market.amm.total_fee.checked_add(fee_to_market)?;
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(fee_to_market)?;
        MARKETS.update(
            deps.storage,
            market_index.to_string(),
            |_m| -> Result<Market, ContractError> { Ok(market) },
        )?;
    }

    if user.collateral.ge(&user_fee) {
        user.collateral = user.collateral.checked_sub(user_fee)?;
    } else {
        user.collateral = Uint128::zero();
    }

    // Increment the user's total fee variables
    user.total_fee_paid = user.total_fee_paid.checked_add(user_fee)?;
    user.total_token_discount = user.total_token_discount.checked_add(token_discount)?;
    user.total_referee_discount = user.total_referee_discount.checked_add(referee_discount)?;

    // Update the referrer's collateral with their reward
    if referrer.is_some() {
        let mut _referrer = USERS.load(deps.storage, &referrer.clone().unwrap())?;
        _referrer.total_referral_reward = _referrer
            .total_referral_reward
            .checked_add(referrer_reward)?;
        // todo what this signifies
        // referrer.exit(ctx.program_id)?;
        USERS.update(
            deps.storage,
            &referrer.unwrap().clone(),
            |_m| -> Result<User, ContractError> { Ok(_referrer) },
        )?;
    }

    let is_oracle_mark_too_divergent_before = is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_before,
        &oracle_guard_rails,
    )?;
    let is_oracle_mark_too_divergent_after = is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_after,
        &oracle_guard_rails,
    )?;

    if is_oracle_mark_too_divergent_after && !is_oracle_mark_too_divergent_before && is_oracle_valid_bool
    {
        return Err(ContractError::OracleMarkSpreadLimit.into());
    }

    let mut len = LENGTH.load(deps.storage)?;
    let trade_history_info_length = len.trade_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
    len.trade_history_length = trade_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    TRADE_HISTORY.save(
        deps.storage,
        (&user_address, trade_history_info_length.to_string()),
        &TradeRecord {
            ts: now,
            user: user_address.clone(),
            direction,
            base_asset_amount,
            quote_asset_amount,
            mark_price_before,
            mark_price_after,
            fee: user_fee,
            referrer_reward,
            referee_discount,
            token_discount,
            liquidation: false,
            market_index,
            oracle_price: Number128::new(oracle_price_after),
        },
    )?;

    if limit_price.is_some()
        && !limit_price_satisfied(
            limit_price.unwrap(),
            quote_asset_amount,
            base_asset_amount,
            direction,
        )?
    {
        return Err(ContractError::SlippageOutsideLimit.into());
    }

    {
        update_funding_rate(
            &mut deps,
            market_index,
            now,
            state.funding_paused,
            Some(mark_price_before),
        )?;
    }

    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_m| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok(Response::new().add_attribute("method", "try_open_position"))
}

pub fn try_close_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_index: u64,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();
    let now = env.block.time.seconds();
    let state = STATE.load(deps.storage)?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let fee_structure = FEESTRUCTURE.load(deps.storage)?;
    settle_funding_payment(&mut deps, &user_address, now)?;

    let position_index = market_index.clone();
    let market_position = POSITIONS.load(deps.storage, (&user_address.clone(), market_index.to_string()))?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mark_price_before = market.amm.mark_price()?;
    let oracle_price_data = market.amm.get_oracle_price()?;
    let oracle_mark_spread_pct_before = calculate_oracle_mark_spread_pct(
        &market.amm,
        &oracle_price_data,
        Some(mark_price_before),
    )?;
    let direction_to_close =
        direction_to_close_position(market_position.base_asset_amount.i128());

    let (quote_asset_amount, base_asset_amount, _) = close(
        &mut deps,
        &user_address,
        market_index,
        position_index,
        now,
        None,
        Some(mark_price_before),
    )?;

    let mut user = USERS.load(deps.storage, &user_address)?;

    market = MARKETS.load(deps.storage, market_index.to_string())?;
    let base_asset_amount = Uint128::from(base_asset_amount.unsigned_abs());
    let referrer = user.referrer.clone();
    let discount_token = Uint128::zero();

    let (user_fee, fee_to_market, token_discount, referrer_reward, referee_discount) =
        calculate_fee_for_trade(
            quote_asset_amount,
            &fee_structure,
            discount_token,
            &referrer,
        )?;

    market.amm.total_fee = market.amm.total_fee.checked_add(fee_to_market)?;
    market.amm.total_fee_minus_distributions = market
        .amm
        .total_fee_minus_distributions
        .checked_add(fee_to_market)?;

    if user.collateral.gt(&user_fee) {
        user.collateral = user.collateral.checked_sub(user_fee)?;
    } else {
        user.collateral = Uint128::zero();
    }

    user.total_fee_paid = user.total_fee_paid.checked_add(user_fee)?;
    user.total_token_discount = user.total_token_discount.checked_add(token_discount)?;
    user.total_referee_discount = user.total_referee_discount.checked_add(referee_discount)?;

    if referrer.is_some() {
        let mut _referrer = USERS.load(deps.storage, &referrer.clone().unwrap())?;
        _referrer.total_referral_reward = _referrer
            .total_referral_reward
            .checked_add(referrer_reward)?;
        USERS.update(
            deps.storage,
            &referrer.unwrap().clone(),
            |_m| -> Result<User, ContractError> { Ok(_referrer) },
        )?;
    }


    let mark_price_after = market.amm.mark_price()?;


    let oracle_mark_spread_pct_after = calculate_oracle_mark_spread_pct(
        &market.amm,
        &oracle_price_data,
        Some(mark_price_after),
    )?;

    let oracle_price_after = oracle_price_data.price;

    let is_oracle_valid =
        is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;
    
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market.clone()) },
    )?;

    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_m| -> Result<User, ContractError> { Ok(user) },
    )?;
    

    if is_oracle_valid {
        let normalised_oracle_price = normalise_oracle_price(
            &market.amm,
            &oracle_price_data,
            Some(mark_price_before),
        )?;
        update_oracle_price_twap(
            &mut deps,
            market_index,
            now,
            normalised_oracle_price,
        )?;
    }

    let is_oracle_mark_too_divergent_before = is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_before,
        &oracle_guard_rails,
    )?;
    let is_oracle_mark_too_divergent_after = is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_after,
        &oracle_guard_rails,
    )?;

    if (is_oracle_mark_too_divergent_after && !is_oracle_mark_too_divergent_before)
        && is_oracle_valid
    {
        return Err(ContractError::OracleMarkSpreadLimit.into());
    }

    let mut len = LENGTH.load(deps.storage)?;
    let trade_history_info_length = len.trade_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
    len.trade_history_length = trade_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    TRADE_HISTORY.save(
        deps.storage,
        (&user_address , trade_history_info_length.to_string()),
        &TradeRecord {
            ts: now,
            user: user_address.clone(),
            direction: direction_to_close,
            base_asset_amount,
            quote_asset_amount,
            mark_price_before,
            mark_price_after,
            fee: user_fee,
            referrer_reward,
            referee_discount,
            token_discount,
            liquidation: false,
            market_index,
            oracle_price: oracle_price_after,
        },
    )?;

    update_funding_rate(
        &mut deps,
        market_index,
        now,
        state.funding_paused,
        Some(mark_price_before),
    )?;

    Ok(Response::new().add_attribute("method", "try_close_position"))
}

//todo later

pub fn try_liquidate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    market_index: u64,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let user_address = addr_validate_to_lower(deps.api, &user)?;
    let now = env.block.time.seconds();

    settle_funding_payment(&mut deps, &user_address, now)?;

    let mut user = USERS.load(deps.storage, &user_address)?;

    let LiquidationStatus {
        liquidation_type,
        total_collateral,
        adjusted_total_collateral,
        unrealized_pnl,
        base_asset_value,
        market_statuses,
        mut margin_requirement,
        margin_ratio,
    } = calculate_liquidation_status(
        &mut deps,
        &user_address
    )?;

    let res: Response = Response::new().add_attribute("method", "try_liquidate");
    let collateral = user.collateral;
    if liquidation_type == LiquidationType::NONE {
        res.clone()
            .add_attribute("total_collateral {}", total_collateral.to_string());
        res.clone().add_attribute(
            "adjusted_total_collateral {}",
            adjusted_total_collateral.to_string(),
        );
        res.clone()
            .add_attribute("margin_requirement {}", margin_requirement.to_string());
        return Err(ContractError::SufficientCollateral.into());
    }

    let is_dust_position = adjusted_total_collateral <= QUOTE_PRECISION;

    let mut base_asset_value_closed: Uint128 = Uint128::zero();
    let mut liquidation_fee = Uint128::zero();

    let is_full_liquidation = liquidation_type == LiquidationType::FULL || is_dust_position;

    if is_full_liquidation {
        let maximum_liquidation_fee = total_collateral
            .checked_mul(Uint128::from(state.full_liquidation_penalty_percentage.numerator()))?
            .checked_div(Uint128::from(state.full_liquidation_penalty_percentage.denominator()))?;

        for market_status in market_statuses.iter() {
            if market_status.base_asset_value.is_zero() {
                continue;
            }

            let market = MARKETS.load(deps.storage, market_status.market_index.to_string())?;
            let mark_price_before = market_status.mark_price_before;
            let oracle_status = &market_status.oracle_status;

            // if the oracle is invalid and the mark moves too far from twap, dont liquidate
            let oracle_is_valid = oracle_status.is_valid;
            if !oracle_is_valid {
                let mark_twap_divergence =
                    calculate_mark_twap_spread_pct(&market.amm, mark_price_before)?;
                let mark_twap_too_divergent =
                    mark_twap_divergence.unsigned_abs() >= MAX_MARK_TWAP_DIVERGENCE.u128();

                if mark_twap_too_divergent {
                    res.clone().add_attribute(
                        "mark_twap_divergence {} for market {}",
                        mark_twap_divergence.to_string(),
                    );
                    continue;
                }
            }

            let market_position = POSITIONS.load(deps.storage, (&user_address, market_index.to_string()))?;
            // todo initialize position

            let mark_price_before_i128 = mark_price_before.u128() as i128;
            let close_position_slippage = match market_status.close_position_slippage {
                Some(close_position_slippage) => close_position_slippage,
                None => calculate_slippage(
                    market_status.base_asset_value,
                    Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
                    mark_price_before_i128,
                )?,
            };
            let close_position_slippage_pct = calculate_slippage_pct(
                close_position_slippage,
                mark_price_before_i128,
            )?;

            let close_slippage_pct_too_large = close_position_slippage_pct
                > MAX_LIQUIDATION_SLIPPAGE.u128() as i128
                || close_position_slippage_pct < -(MAX_LIQUIDATION_SLIPPAGE.u128() as i128);

            let oracle_mark_divergence_after_close = if !close_slippage_pct_too_large {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    .checked_add(close_position_slippage_pct)
                    .ok_or_else(|| (ContractError::MathError))?
            } else if close_position_slippage_pct > 0 {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_add((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            } else {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_sub((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            };

            let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    
            let oracle_mark_too_divergent_after_close = is_oracle_mark_too_divergent(
                oracle_mark_divergence_after_close,
                &oracle_guard_rails,
            )?;

            // if closing pushes outside the oracle mark threshold, don't liquidate
            if oracle_is_valid && oracle_mark_too_divergent_after_close {
                // but only skip the liquidation if it makes the divergence worse
                if oracle_status.oracle_mark_spread_pct.i128().unsigned_abs()
                    < oracle_mark_divergence_after_close.unsigned_abs()
                {
                    res.clone().add_attribute(
                        "oracle_mark_divergence_after_close ",
                        oracle_mark_divergence_after_close.to_string(),
                    );
                    continue;
                }
            }

            let direction_to_close = direction_to_close_position(
                market_position.base_asset_amount.i128(),
            );

            // just reduce position if position is too big
            let (quote_asset_amount, base_asset_amount) = if close_slippage_pct_too_large {
                let quote_asset_amount = market_status
                    .base_asset_value
                    .checked_mul(MAX_LIQUIDATION_SLIPPAGE)?
                    .checked_div(Uint128::from(close_position_slippage_pct.unsigned_abs()))?;

                let base_asset_amount = reduce(
                    &mut deps,
                    direction_to_close,
                    quote_asset_amount,
                    &user_address,
                    market_index,
                    market_index,
                    now,
                    Some(mark_price_before),
                )?;

                (quote_asset_amount, base_asset_amount)
            } else {
                let (quote_asset_amount, base_asset_amount, _) = close(
                    &mut deps,
                    &user_address,
                    market_index,
                    market_index,
                    now,
                    None,
                    Some(mark_price_before),
                )?;

                (quote_asset_amount, base_asset_amount)
            };

            let base_asset_amount = Uint128::from(base_asset_amount.unsigned_abs());
            base_asset_value_closed = base_asset_value_closed.checked_add(quote_asset_amount)?;
            let mark_price_after = market.amm.mark_price()?;

            let mut len = LENGTH.load(deps.storage)?;
            let trade_history_info_length = len.trade_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
            len.trade_history_length = trade_history_info_length;
            LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
                Ok(len)
            })?;
            TRADE_HISTORY.save(
                deps.storage,
                (&user_address ,trade_history_info_length.to_string()),
                &TradeRecord {
                    ts: now,
                    user: user_address.clone(),
                    direction: direction_to_close,
                    base_asset_amount,
                    quote_asset_amount,
                    mark_price_before,
                    mark_price_after,
                    fee: Uint128::zero(),
                    referrer_reward: Uint128::zero(),
                    referee_discount: Uint128::zero(),
                    token_discount: Uint128::zero(),
                    liquidation: true,
                    market_index,
                    oracle_price: market_status.oracle_status.price_data.price,
                },
            )?;

            margin_requirement = margin_requirement.checked_sub(
                market_status
                    .maintenance_margin_requirement
                    .checked_mul(quote_asset_amount)?
                    .checked_div(market_status.base_asset_value)?,
            )?;

            let market_liquidation_fee = maximum_liquidation_fee
                .checked_mul(quote_asset_amount)?
                .checked_div(base_asset_value)?;

            liquidation_fee = liquidation_fee.checked_add(market_liquidation_fee)?;

            let adjusted_total_collateral_after_fee =
                adjusted_total_collateral.checked_sub(liquidation_fee)?;

            if !is_dust_position && margin_requirement < adjusted_total_collateral_after_fee {
                break;
            }
        }
    } else {
        let maximum_liquidation_fee = total_collateral
            .checked_mul(Uint128::from(state.partial_liquidation_penalty_percentage.numerator()))?
            .checked_div(Uint128::from(state.partial_liquidation_penalty_percentage.denominator()))?;
        let maximum_base_asset_value_closed = base_asset_value
            .checked_mul(Uint128::from(state.partial_liquidation_close_percentage.numerator()))?
            .checked_div(Uint128::from(state.partial_liquidation_close_percentage.denominator()))?;
        for market_status in market_statuses.iter() {
            if market_status.base_asset_value.is_zero() {
                continue;
            }

            let oracle_status = &market_status.oracle_status;
            let market = MARKETS.load(deps.storage, market_index.to_string())?;
            let mark_price_before = market_status.mark_price_before;

            let oracle_is_valid = oracle_status.is_valid;
            if !oracle_is_valid {
                let mark_twap_divergence =
                    calculate_mark_twap_spread_pct(&market.amm, mark_price_before)?;
                let mark_twap_too_divergent =
                    mark_twap_divergence.unsigned_abs() >= MAX_MARK_TWAP_DIVERGENCE.u128();

                if mark_twap_too_divergent {
                    res.clone()
                        .add_attribute("mark_twap_divergence", mark_twap_divergence.to_string());
                    continue;
                }
            }

            let market_position = POSITIONS.load(deps.storage, (&user_address, market_index.to_string()))?;

            let mut quote_asset_amount = market_status
                .base_asset_value
                .checked_mul(Uint128::from(state.partial_liquidation_close_percentage.numerator()))?
                .checked_div(Uint128::from(state.partial_liquidation_close_percentage.denominator()))?;

            let mark_price_before_i128 = mark_price_before.u128() as i128;
            let reduce_position_slippage = match market_status.close_position_slippage {
                Some(close_position_slippage) => close_position_slippage.div(4),
                None => calculate_slippage(
                    market_status.base_asset_value,
                    Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
                    mark_price_before_i128,
                )?
                .div(4),
            };

            let reduce_position_slippage_pct = calculate_slippage_pct(
                reduce_position_slippage,
                mark_price_before_i128,
            )?;

            res.clone().add_attribute(
                "reduce_position_slippage_pct",
                reduce_position_slippage_pct.to_string(),
            );

            let reduce_slippage_pct_too_large = reduce_position_slippage_pct
                > (MAX_LIQUIDATION_SLIPPAGE.u128() as i128)
                || reduce_position_slippage_pct < -(MAX_LIQUIDATION_SLIPPAGE.u128() as i128);

            let oracle_mark_divergence_after_reduce = if !reduce_slippage_pct_too_large {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    .checked_add(reduce_position_slippage_pct)
                    .ok_or_else(|| (ContractError::MathError))?
            } else if reduce_position_slippage_pct > 0 {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_add((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            } else {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_sub((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            };

            let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
            let oracle_mark_too_divergent_after_reduce =
                is_oracle_mark_too_divergent(
                    oracle_mark_divergence_after_reduce,
                    &oracle_guard_rails,
                )?;

            // if reducing pushes outside the oracle mark threshold, don't liquidate
            if oracle_is_valid && oracle_mark_too_divergent_after_reduce {
                // but only skip the liquidation if it makes the divergence worse
                if oracle_status.oracle_mark_spread_pct.i128().unsigned_abs()
                    < oracle_mark_divergence_after_reduce.unsigned_abs()
                {
                    res.clone().add_attribute(
                        "oracle_mark_spread_pct_after_reduce",
                        oracle_mark_divergence_after_reduce.to_string(),
                    );
                    return Err(ContractError::OracleMarkSpreadLimit.into());
                }
            }

            if reduce_slippage_pct_too_large {
                quote_asset_amount = quote_asset_amount
                    .checked_mul(MAX_LIQUIDATION_SLIPPAGE)?
                    .checked_div(Uint128::from(reduce_position_slippage_pct.unsigned_abs()))?;
            }

            base_asset_value_closed = base_asset_value_closed.checked_add(quote_asset_amount)?;

            let direction_to_reduce = direction_to_close_position(
                market_position.base_asset_amount.i128(),
            );

            let base_asset_amount = reduce(
                &mut deps,
                direction_to_reduce,
                quote_asset_amount,
                &user_address,
                market_index,
                market_index,
                now,
                Some(mark_price_before),
            )?
            .unsigned_abs();

            let mark_price_after = market.amm.mark_price()?;

            let mut len = LENGTH.load(deps.storage)?;
            let trade_history_info_length = len.trade_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
            len.trade_history_length = trade_history_info_length;
            LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
                Ok(len)
            })?;        
            TRADE_HISTORY.save(
                deps.storage,
                (&user_address, trade_history_info_length.to_string()),
                &TradeRecord {
                    ts: now,
                    user: user_address.clone(),
                    direction: direction_to_reduce,
                    base_asset_amount: Uint128::from(base_asset_amount),
                    quote_asset_amount,
                    mark_price_before,
                    mark_price_after,
                    fee: Uint128::zero(),
                    referrer_reward: Uint128::zero(),
                    referee_discount: Uint128::zero(),
                    token_discount: Uint128::zero(),
                    liquidation: true,
                    market_index,
                    oracle_price: market_status.oracle_status.price_data.price,
                },
            )?;

            margin_requirement = margin_requirement.checked_sub(
                market_status
                    .partial_margin_requirement
                    .checked_mul(quote_asset_amount)?
                    .checked_div(market_status.base_asset_value)?,
            )?;

            let market_liquidation_fee = maximum_liquidation_fee
                .checked_mul(quote_asset_amount)?
                .checked_div(maximum_base_asset_value_closed)?;

            liquidation_fee = liquidation_fee.checked_add(market_liquidation_fee)?;

            let adjusted_total_collateral_after_fee =
                adjusted_total_collateral.checked_sub(liquidation_fee)?;

            if margin_requirement < adjusted_total_collateral_after_fee {
                break;
            }
        }
    }
    if base_asset_value_closed.is_zero() {
        return Err(ContractError::NoPositionsLiquidatable);
    }

    let balance_collateral = query_balance(&deps.querier, state.collateral_vault.clone())?;

    let balance_insurance = query_balance(&deps.querier, state.insurance_vault.clone())?;

    let (withdrawal_amount, _) = calculate_withdrawal_amounts(
        liquidation_fee,
        Uint128::from(balance_collateral),
        Uint128::from(balance_insurance),
    )?;

    user = USERS.load(deps.storage, &user_address)?;
    user.collateral = user.collateral.checked_sub(liquidation_fee)?;
    USERS.update(deps.storage, &user_address, |_u| -> Result<User, ContractError> {
        Ok(user)
    })?;

    let fee_to_liquidator = if is_full_liquidation {
        withdrawal_amount.checked_div(Uint128::from(
            state.full_liquidation_liquidator_share_denominator,
        ))?
    } else {
        withdrawal_amount.checked_div(Uint128::from(
            state.partial_liquidation_liquidator_share_denominator,
        ))?
    };

    let fee_to_insurance_fund = withdrawal_amount.checked_sub(fee_to_liquidator)?;

    if fee_to_liquidator.gt(&Uint128::zero()) {
        let mut liquidator = USERS.load(deps.storage, &info.sender.clone())?;
        liquidator.collateral = liquidator
            .collateral
            .checked_add(Uint128::from(fee_to_liquidator))?;

        USERS.update(
            deps.storage,
            &info.sender.clone(),
            |_m| -> Result<User, ContractError> { Ok(liquidator) },
        )?;
    }
    let mut messages: Vec<CosmosMsg> = vec![];
    if fee_to_insurance_fund.gt(&Uint128::zero()) {
        let message = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.collateral_vault.to_string(),
            msg: to_binary(&VaultInterface::Withdraw {
                to_address: state.insurance_vault.clone(),
                amount: fee_to_insurance_fund.u128(),
            })?,
            funds: vec![],
        });
        messages.push(message);
    }

    let mut len = LENGTH.load(deps.storage)?;
    let liquidation_history_info_length = len.liquidation_history_length.checked_add(1).ok_or_else(|| (ContractError::MathError))?;
    len.liquidation_history_length = liquidation_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    LIQUIDATION_HISTORY.save(
        deps.storage,
        (user_address.clone(), liquidation_history_info_length.to_string()),
        &LiquidationRecord {
            ts: now,
            record_id: liquidation_history_info_length,
            user: user_address,
            partial: !is_full_liquidation,
            base_asset_value,
            base_asset_value_closed,
            liquidation_fee,
            liquidator: info.sender.clone(),
            total_collateral,
            collateral,
            unrealized_pnl: Number128::new(unrealized_pnl),
            margin_ratio,
            fee_to_liquidator: fee_to_liquidator.u128() as u64,
            fee_to_insurance_fund: fee_to_insurance_fund.u128() as u64,
        },
    )?;
    Ok(res.add_messages(messages))
}

pub fn try_settle_funding_payment(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let user_address = info.sender;

    settle_funding_payment(&mut deps, &user_address, now)?;
    Ok(Response::new().add_attribute("method", "try_settle_funding_payment"))
}

pub fn get_user(deps: Deps, user_address: String) -> Result<UserResponse, ContractError> {
    let user = USERS.load(
        deps.storage,
        &addr_validate_to_lower(deps.api, &user_address)?,
    )?;
    let referrer: String;
    if user.referrer.is_none() {
        referrer = "".to_string();
    } else {
        referrer = user.referrer.unwrap().into();
    }
    let ur = UserResponse {
        collateral: user.collateral,
        cumulative_deposits: user.cumulative_deposits,
        total_fee_paid: user.total_fee_paid,
        total_token_discount: user.total_token_discount,
        total_referral_reward: user.total_referral_reward,
        total_referee_discount: user.total_token_discount,
        referrer,
    };
    Ok(ur)
}

pub fn get_user_position(
    deps: Deps,
    user_address: String,
    index: u64,
) -> Result<UserPositionResponse, ContractError> {
    let position = POSITIONS.load(
        deps.storage,
        (&addr_validate_to_lower(deps.api, &user_address)?, index.to_string()),
    )?;
    let upr = UserPositionResponse {
        base_asset_amount: position.base_asset_amount,
        quote_asset_amount: position.quote_asset_amount,
        last_cumulative_funding_rate: position.last_cumulative_funding_rate,
        last_cumulative_repeg_rebate: position.last_cumulative_repeg_rebate,
        last_funding_rate_ts: position.last_funding_rate_ts
    };
    Ok(upr)
}

pub fn get_admin(deps: Deps) -> Result<AdminResponse, ContractError> {
    let admin = AdminResponse {
        admin: ADMIN.query_admin(deps).unwrap().admin.unwrap(),
    };
    Ok(admin)
}

pub fn is_exchange_paused(deps: Deps) -> Result<IsExchangePausedResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let ex_paused = IsExchangePausedResponse {
        exchange_paused: state.exchange_paused,
    };
    Ok(ex_paused)
}

pub fn is_funding_paused(deps: Deps) -> Result<IsFundingPausedResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let funding_paused = IsFundingPausedResponse {
        funding_paused: state.funding_paused,
    };
    Ok(funding_paused)
}

pub fn admin_controls_prices(deps: Deps) -> Result<AdminControlsPricesResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let admin_control = AdminControlsPricesResponse {
        admin_controls_prices: state.admin_controls_prices,
    };
    Ok(admin_control)
}
pub fn get_vaults_address(deps: Deps) -> Result<VaultsResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let vaults = VaultsResponse {
        collateral_vault: state.collateral_vault.to_string(),
        insurance_vault: state.insurance_vault.to_string(),
    };
    Ok(vaults)
}

pub fn get_oracle_address(deps: Deps) -> Result<OracleResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let oracle = OracleResponse {
        oracle: state.oracle.to_string(),
    };
    Ok(oracle)
}

pub fn get_margin_ratios(deps: Deps) -> Result<MarginRatioResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let margin_ratio = MarginRatioResponse {
        margin_ratio_initial: state.margin_ratio_initial,
        margin_ratio_partial: state.margin_ratio_partial,
        margin_ratio_maintenance: state.margin_ratio_maintenance,
    };
    Ok(margin_ratio)
}

pub fn get_partial_liquidation_close_percentage(
    deps: Deps,
) -> Result<PartialLiquidationClosePercentageResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let partial_liq_close_perc = PartialLiquidationClosePercentageResponse {
        value: state.partial_liquidation_close_percentage,
    };
    Ok(partial_liq_close_perc)
}

pub fn get_partial_liquidation_penalty_percentage(
    deps: Deps,
) -> Result<PartialLiquidationPenaltyPercentageResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let partial_liq_penalty_perc = PartialLiquidationPenaltyPercentageResponse {
        value: state.partial_liquidation_penalty_percentage,
    };
    Ok(partial_liq_penalty_perc)
}

pub fn get_full_liquidation_penalty_percentage(
    deps: Deps,
) -> Result<FullLiquidationPenaltyPercentageResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let full_liq_penalty_perc = FullLiquidationPenaltyPercentageResponse {
        value: state.full_liquidation_penalty_percentage,
    };
    Ok(full_liq_penalty_perc)
}

pub fn get_partial_liquidator_share_percentage(
    deps: Deps,
) -> Result<PartialLiquidatorSharePercentageResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let partial_liquidator_share_perc = PartialLiquidatorSharePercentageResponse {
        denominator: state.partial_liquidation_liquidator_share_denominator,
    };
    Ok(partial_liquidator_share_perc)
}

pub fn get_full_liquidator_share_percentage(
    deps: Deps,
) -> Result<FullLiquidatorSharePercentageResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let full_liquidator_share_perc = FullLiquidatorSharePercentageResponse {
        denominator: state.full_liquidation_liquidator_share_denominator,
    };
    Ok(full_liquidator_share_perc)
}
pub fn get_max_deposit_limit(deps: Deps) -> Result<MaxDepositLimitResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let max_deposit = MaxDepositLimitResponse {
        max_deposit: state.max_deposit,
    };
    Ok(max_deposit)
}

pub fn get_market_length(deps: Deps) -> Result<MarketLengthResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    Ok(MarketLengthResponse {
        length: state.markets_length,
    })
}

pub fn get_oracle_guard_rails(deps: Deps) -> Result<OracleGuardRailsResponse, ContractError> {
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let ogr = OracleGuardRailsResponse {
        use_for_liquidations: oracle_guard_rails.use_for_liquidations,
        mark_oracle_divergence: oracle_guard_rails.mark_oracle_divergence,
        slots_before_stale: Number128::new(oracle_guard_rails.slots_before_stale as i128),
        confidence_interval_max_size: oracle_guard_rails.confidence_interval_max_size,
        too_volatile_ratio: oracle_guard_rails.too_volatile_ratio,
    };
    Ok(ogr)
}

pub fn get_order_state(deps: Deps) -> Result<OrderStateResponse, ContractError> {
    let orderstate = ORDERSTATE.load(deps.storage)?;
    let os = OrderStateResponse {
        min_order_quote_asset_amount: orderstate.min_order_quote_asset_amount,
        reward: orderstate.reward,
        time_based_reward_lower_bound: orderstate.time_based_reward_lower_bound,
    };
    Ok(os)
}

pub fn get_fee_structure(deps: Deps) -> Result<FeeStructureResponse, ContractError> {
    let fs = FEESTRUCTURE.load(deps.storage)?;
    let res = FeeStructureResponse {
        fee: fs.fee,
        first_tier_minimum_balance: fs.first_tier_minimum_balance,
        first_tier_discount: fs.first_tier_discount,
        second_tier_minimum_balance: fs.second_tier_minimum_balance,
        second_tier_discount: fs.second_tier_discount,
        third_tier_minimum_balance: fs.third_tier_minimum_balance,
        third_tier_discount: fs.third_tier_discount,
        fourth_tier_minimum_balance: fs.fourth_tier_minimum_balance,
        fourth_tier_discount: fs.fourth_tier_discount,
        referrer_reward: fs.referrer_reward,
        referee_discount: fs.referee_discount,
    };
    Ok(res)
}

pub fn get_length(deps: Deps) -> Result<LengthResponse, ContractError> {
    let len = LENGTH.load(deps.storage)?;
    let length = LengthResponse {
        curve_history_length: len.curve_history_length,
        deposit_history_length: len.deposit_history_length,
        funding_payment_history_length: len.funding_payment_history_length,
        funding_rate_history_length: len.funding_rate_history_length,
        liquidation_history_length: len.liquidation_history_length,
        order_history_length: len.order_history_length,
        trade_history_length: len.trade_history_length,
    };
    Ok(length)
}

pub fn get_curve_history(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<CurveHistoryResponse>, ContractError> {
    let chl = LENGTH.load(deps.storage)?.curve_history_length;
    let mut curves: Vec<CurveHistoryResponse> = vec![];
    if chl > 0 {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after
            .map(|start| start.joined_key())
            .map(Bound::Exclusive);

        curves = CURVEHISTORY
            .range(deps.storage, start, None, Order::Descending)
            .filter_map(|curve_record| {
                curve_record.ok().map(|curve| CurveHistoryResponse {
                    ts: curve.1.ts,
                    record_id: curve.1.record_id,
                    market_index: curve.1.market_index,
                    peg_multiplier_before: curve.1.peg_multiplier_before,
                    base_asset_reserve_before: curve.1.base_asset_reserve_before,
                    quote_asset_reserve_before: curve.1.quote_asset_reserve_before,
                    sqrt_k_before: curve.1.sqrt_k_before,
                    peg_multiplier_after: curve.1.peg_multiplier_after,
                    base_asset_reserve_after: curve.1.base_asset_reserve_after,
                    quote_asset_reserve_after: curve.1.quote_asset_reserve_after,
                    sqrt_k_after: curve.1.sqrt_k_after,
                    base_asset_amount_long: curve.1.base_asset_amount_long,
                    base_asset_amount_short: curve.1.base_asset_amount_short,
                    base_asset_amount: curve.1.base_asset_amount,
                    open_interest: curve.1.open_interest,
                    total_fee: curve.1.total_fee,
                    total_fee_minus_distributions: curve.1.total_fee_minus_distributions,
                    adjustment_cost: curve.1.adjustment_cost,
                    oracle_price: curve.1.oracle_price,
                })
            })
            .take(limit)
            .collect();
    }
    Ok(curves)
}

pub fn get_deposit_history(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<DepositHistoryResponse>, ContractError> {
    let user_addr = addr_validate_to_lower(deps.api, &user_address.to_string())?;
    let mut deposit_history: Vec<DepositHistoryResponse> = vec![];
    let user_cumulative_deposit = (USERS.load(deps.storage, &user_addr)?).cumulative_deposits;
    if user_cumulative_deposit.u128() > 0 {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after
            .map(|start| start.joined_key())
            .map(Bound::Exclusive);
        deposit_history = DEPOSIT_HISTORY
            .prefix(user_addr)
            .range(deps.storage, start, None, Order::Descending)
            .filter_map(|records| {
                records.ok().map(|record| DepositHistoryResponse {
                    ts: record.1.ts,
                    record_id: record.1.record_id,
                    user: record.1.user.to_string(),
                    direction: record.1.direction,
                    collateral_before: record.1.collateral_before,
                    cumulative_deposits_before: record.1.cumulative_deposits_before,
                    amount: record.1.amount,
                })
            })
            .take(limit)
            .collect();
    }
    Ok(deposit_history)
}

pub fn get_funding_payment_history(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<FundingPaymentHistoryResponse>, ContractError> {
    let user_addr = addr_validate_to_lower(deps.api, user_address.as_str())?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let funding_payment_history = FUNDING_PAYMENT_HISTORY
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|funding_payments| {
            funding_payments
                .ok()
                .map(|fp| FundingPaymentHistoryResponse {
                    ts: fp.1.ts,
                    record_id: fp.1.record_id,
                    user: fp.1.user.to_string(),
                    market_index: fp.1.market_index,
                    funding_payment: fp.1.funding_payment,
                    base_asset_amount: fp.1.base_asset_amount,
                    user_last_cumulative_funding: fp.1.user_last_cumulative_funding,
                    user_last_funding_rate_ts: fp.1.user_last_funding_rate_ts,
                    amm_cumulative_funding_long: fp.1.amm_cumulative_funding_long,
                    amm_cumulative_funding_short: fp.1.amm_cumulative_funding_short,
                })
        })
        .take(limit)
        .collect();
    Ok(funding_payment_history)
}

pub fn get_funding_rate_history(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<FundingRateHistoryResponse>, ContractError> {
    let mut fr_history: Vec<FundingRateHistoryResponse> = vec![];
    let length = LENGTH.load(deps.storage)?;
    if length.funding_rate_history_length > 0 {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after
            .map(|start| start.joined_key())
            .map(Bound::Exclusive);
        fr_history = FUNDING_RATE_HISTORY
            .range(deps.storage, start, None, Order::Descending)
            .filter_map(|fr_records| {
                fr_records
                    .ok()
                    .map(|funding_record| FundingRateHistoryResponse {
                        ts: funding_record.1.ts,
                        record_id: funding_record.1.record_id,
                        market_index: funding_record.1.market_index,
                        funding_rate: funding_record.1.funding_rate,
                        cumulative_funding_rate_long: funding_record.1.cumulative_funding_rate_long,
                        cumulative_funding_rate_short: funding_record
                            .1
                            .cumulative_funding_rate_short,
                        oracle_price_twap: funding_record.1.oracle_price_twap,
                        mark_price_twap: funding_record.1.mark_price_twap,
                    })
            })
            .take(limit)
            .collect();
    }
    Ok(fr_history)
}

pub fn get_liquidation_history(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<LiquidationHistoryResponse>, ContractError> {
    let user_addr = addr_validate_to_lower(deps.api, &user_address)?;

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let liq_history = LIQUIDATION_HISTORY
        .prefix(user_addr)
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|records| {
            records.ok().map(|record| LiquidationHistoryResponse {
                ts: record.1.ts,
                record_id: record.1.record_id,
                user: record.1.user.to_string(),
                partial: record.1.partial,
                base_asset_value: record.1.base_asset_value,
                base_asset_value_closed: record.1.base_asset_value_closed,
                liquidation_fee: record.1.liquidation_fee,
                fee_to_liquidator: record.1.fee_to_liquidator,
                fee_to_insurance_fund: record.1.fee_to_insurance_fund,
                liquidator: record.1.liquidator.to_string(),
                total_collateral: record.1.total_collateral,
                collateral: record.1.collateral,
                unrealized_pnl: record.1.unrealized_pnl,
                margin_ratio: record.1.margin_ratio,
            })
        })
        .take(limit)
        .collect();
    Ok(liq_history)
}

pub fn get_trade_history(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<TradeHistoryResponse>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let trade_history = TRADE_HISTORY
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|records| {
            records.ok().map(|record| TradeHistoryResponse {
                ts: record.1.ts,
                user: record.1.user.to_string(),
                direction: record.1.direction,
                base_asset_amount: record.1.base_asset_amount,
                quote_asset_amount: record.1.quote_asset_amount,
                mark_price_before: record.1.mark_price_before,
                mark_price_after: record.1.mark_price_after,
                fee: record.1.fee,
                referrer_reward: record.1.referrer_reward,
                referee_discount: record.1.referee_discount,
                token_discount: record.1.token_discount,
                liquidation: record.1.liquidation,
                market_index: record.1.market_index,
                oracle_price: record.1.oracle_price,
            })
        })
        .take(limit)
        .collect();
    Ok(trade_history)
}

pub fn get_market_info(deps: Deps, market_index: u64) -> Result<MarketInfoResponse, ContractError> {
    let market = MARKETS.load(deps.storage, market_index.to_string())?;
    let market_info = MarketInfoResponse {
        market_name: market.market_name,
        initialized: market.initialized,
        base_asset_amount_long: market.base_asset_amount_long,
        base_asset_amount_short: market.base_asset_amount_short,
        base_asset_amount: market.base_asset_amount,
        open_interest: market.open_interest,
        oracle: market.amm.oracle.into(),
        oracle_source: market.amm.oracle_source,
        base_asset_reserve: market.amm.base_asset_reserve,
        quote_asset_reserve: market.amm.quote_asset_reserve,
        cumulative_repeg_rebate_long: market.amm.cumulative_repeg_rebate_long,
        cumulative_repeg_rebate_short: market.amm.cumulative_repeg_rebate_short,
        cumulative_funding_rate_long: market.amm.cumulative_funding_rate_long,
        cumulative_funding_rate_short: market.amm.cumulative_funding_rate_short,
        last_funding_rate: market.amm.last_funding_rate,
        last_funding_rate_ts: market.amm.last_funding_rate_ts,
        funding_period: market.amm.funding_period,
        last_oracle_price_twap: market.amm.last_oracle_price_twap,
        last_mark_price_twap: market.amm.last_mark_price_twap,
        last_mark_price_twap_ts: market.amm.last_mark_price_twap_ts,
        sqrt_k: market.amm.sqrt_k,
        peg_multiplier: market.amm.peg_multiplier,
        total_fee: market.amm.total_fee,
        total_fee_minus_distributions: market.amm.total_fee_minus_distributions,
        total_fee_withdrawn: market.amm.total_fee_withdrawn,
        minimum_trade_size: Uint128::from(100000000 as u64),
        last_oracle_price_twap_ts: market.amm.last_oracle_price_twap_ts,
        last_oracle_price: market.amm.last_oracle_price,
        minimum_base_asset_trade_size: market.amm.minimum_base_asset_trade_size,
        minimum_quote_asset_trade_size: market.amm.minimum_quote_asset_trade_size
    };
    Ok(market_info)
}

pub fn try_calculate_liquidation_status(
    deps: &Deps,
    user_addr: &Addr,
    oracle_guard_rails: &OracleGuardRails,
) -> Result<LiquidationStatus, ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;

    let mut partial_margin_requirement: Uint128 = Uint128::zero();
    let mut maintenance_margin_requirement: Uint128 = Uint128::zero();
    let mut base_asset_value: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;
    let mut adjusted_unrealized_pnl: i128 = 0;
    let mut market_statuses: Vec<MarketStatus> = Vec::new();

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }

                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;
                let (amm_position_base_asset_value, amm_position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;

                base_asset_value = base_asset_value.checked_add(amm_position_base_asset_value)?;
                unrealized_pnl = unrealized_pnl
                    .checked_add(amm_position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;

                // Block the liquidation if the oracle is invalid or the oracle and mark are too divergent
                let mark_price_before = market.amm.mark_price()?;

                let oracle_status =
                    get_oracle_status(&market.amm, oracle_guard_rails, Some(mark_price_before))?;

                let market_partial_margin_requirement: Uint128;
                let market_maintenance_margin_requirement: Uint128;
                let mut close_position_slippage = None;
                if oracle_status.is_valid
                    && use_oracle_price_for_margin_calculation(
                        oracle_status.oracle_mark_spread_pct.i128(),
                        &oracle_guard_rails,
                    )?
                {
                    let exit_slippage = calculate_slippage(
                        amm_position_base_asset_value,
                        Uint128::from(m.base_asset_amount.i128().unsigned_abs()),
                        mark_price_before.u128() as i128,
                    )?;
                    close_position_slippage = Some(exit_slippage);

                    let oracle_exit_price = oracle_status
                        .price_data
                        .price
                        .i128()
                        .checked_add(exit_slippage)
                        .ok_or_else(|| (ContractError::HelpersError))?;

                    let (oracle_position_base_asset_value, oracle_position_unrealized_pnl) =
                        calculate_base_asset_value_and_pnl_with_oracle_price(
                            &m,
                            oracle_exit_price,
                        )?;

                    let oracle_provides_better_pnl =
                        oracle_position_unrealized_pnl > amm_position_unrealized_pnl;
                    if oracle_provides_better_pnl {
                        adjusted_unrealized_pnl = adjusted_unrealized_pnl
                            .checked_add(oracle_position_unrealized_pnl)
                            .ok_or_else(|| (ContractError::HelpersError))?;

                        market_partial_margin_requirement = (oracle_position_base_asset_value)
                            .checked_mul(market.margin_ratio_partial.into())?;

                        partial_margin_requirement = partial_margin_requirement
                            .checked_add(market_partial_margin_requirement)?;

                        market_maintenance_margin_requirement = oracle_position_base_asset_value
                            .checked_mul(market.margin_ratio_maintenance.into())?;

                        maintenance_margin_requirement = maintenance_margin_requirement
                            .checked_add(market_maintenance_margin_requirement)?;
                    } else {
                        adjusted_unrealized_pnl = adjusted_unrealized_pnl
                            .checked_add(amm_position_unrealized_pnl)
                            .ok_or_else(|| (ContractError::HelpersError))?;

                        market_partial_margin_requirement = (amm_position_base_asset_value)
                            .checked_mul(market.margin_ratio_partial.into())?;

                        partial_margin_requirement = partial_margin_requirement
                            .checked_add(market_partial_margin_requirement)?;

                        market_maintenance_margin_requirement = amm_position_base_asset_value
                            .checked_mul(market.margin_ratio_maintenance.into())?;

                        maintenance_margin_requirement = maintenance_margin_requirement
                            .checked_add(market_maintenance_margin_requirement)?;
                    }
                } else {
                    adjusted_unrealized_pnl = adjusted_unrealized_pnl
                        .checked_add(amm_position_unrealized_pnl)
                        .ok_or_else(|| (ContractError::HelpersError))?;

                    market_partial_margin_requirement = (amm_position_base_asset_value)
                        .checked_mul(market.margin_ratio_partial.into())?;

                    partial_margin_requirement =
                        partial_margin_requirement.checked_add(market_partial_margin_requirement)?;

                    market_maintenance_margin_requirement = amm_position_base_asset_value
                        .checked_mul(market.margin_ratio_maintenance.into())?;

                    maintenance_margin_requirement = maintenance_margin_requirement
                        .checked_add(market_maintenance_margin_requirement)?;
                }

                market_statuses.push(MarketStatus {
                    market_index: n,
                    partial_margin_requirement: market_partial_margin_requirement
                        .checked_div(MARGIN_PRECISION)?,
                    maintenance_margin_requirement: market_maintenance_margin_requirement
                        .checked_div(MARGIN_PRECISION)?,
                    base_asset_value: amm_position_base_asset_value,
                    mark_price_before,
                    oracle_status,
                    close_position_slippage,
                });
            }
            Err(_) => continue,
        }
    }

    partial_margin_requirement = partial_margin_requirement.checked_div(MARGIN_PRECISION)?;

    maintenance_margin_requirement =
        maintenance_margin_requirement.checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;
    let adjusted_total_collateral =
        calculate_updated_collateral(user.collateral, adjusted_unrealized_pnl)?;

    let requires_partial_liquidation = adjusted_total_collateral < partial_margin_requirement;
    let requires_full_liquidation = adjusted_total_collateral < maintenance_margin_requirement;

    let liquidation_type = if requires_full_liquidation {
        LiquidationType::FULL
    } else if requires_partial_liquidation {
        LiquidationType::PARTIAL
    } else {
        LiquidationType::NONE
    };

    let margin_requirement = match liquidation_type {
        LiquidationType::FULL => maintenance_margin_requirement,
        LiquidationType::PARTIAL => partial_margin_requirement,
        LiquidationType::NONE => partial_margin_requirement,
    };

    // Sort the market statuses such that we close the markets with biggest margin requirements first
    if liquidation_type == LiquidationType::FULL {
        market_statuses.sort_by(|a, b| {
            b.maintenance_margin_requirement
                .cmp(&a.maintenance_margin_requirement)
        });
    } else if liquidation_type == LiquidationType::PARTIAL {
        market_statuses.sort_by(|a, b| {
            b.partial_margin_requirement
                .cmp(&a.partial_margin_requirement)
        });
    }

    let margin_ratio = if base_asset_value.is_zero() {
        Uint128::MAX
    } else {
        total_collateral
            .checked_mul(MARGIN_PRECISION)?
            .checked_div(base_asset_value)?
    };

    Ok(LiquidationStatus {
        liquidation_type,
        margin_requirement,
        total_collateral,
        unrealized_pnl,
        adjusted_total_collateral,
        base_asset_value,
        market_statuses,
        margin_ratio,
    })
}

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:clearing-house";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    //TODO:: adding condition to check the initialization, if it's done already
    let fs = FeeStructure {
        fee: Decimal::from_ratio(DEFAULT_FEE_NUMERATOR, DEFAULT_FEE_DENOMINATOR),
        first_tier_minimum_balance: DEFAULT_DISCOUNT_TOKEN_FIRST_TIER_MINIMUM_BALANCE,
        first_tier_discount: Decimal::from_ratio(
            DEFAULT_DISCOUNT_TOKEN_FIRST_TIER_DISCOUNT_NUMERATOR,
            DEFAULT_DISCOUNT_TOKEN_FIRST_TIER_DISCOUNT_DENOMINATOR,
        ),
        second_tier_minimum_balance: DEFAULT_DISCOUNT_TOKEN_SECOND_TIER_MINIMUM_BALANCE,
        second_tier_discount: Decimal::from_ratio(
            DEFAULT_DISCOUNT_TOKEN_SECOND_TIER_DISCOUNT_DENOMINATOR,
            DEFAULT_DISCOUNT_TOKEN_SECOND_TIER_DISCOUNT_DENOMINATOR,
        ),
        third_tier_minimum_balance: DEFAULT_DISCOUNT_TOKEN_THIRD_TIER_MINIMUM_BALANCE,
        third_tier_discount: Decimal::from_ratio(
            DEFAULT_DISCOUNT_TOKEN_THIRD_TIER_DISCOUNT_DENOMINATOR,
            DEFAULT_DISCOUNT_TOKEN_THIRD_TIER_DISCOUNT_DENOMINATOR,
        ),
        fourth_tier_minimum_balance: DEFAULT_DISCOUNT_TOKEN_FOURTH_TIER_MINIMUM_BALANCE,
        fourth_tier_discount: Decimal::from_ratio(
            DEFAULT_DISCOUNT_TOKEN_FOURTH_TIER_DISCOUNT_DENOMINATOR,
            DEFAULT_DISCOUNT_TOKEN_FOURTH_TIER_DISCOUNT_DENOMINATOR,
        ),
        referrer_reward: Decimal::from_ratio(
            DEFAULT_REFERRER_REWARD_NUMERATOR,
            DEFAULT_REFERRER_REWARD_DENOMINATOR,
        ),
        referee_discount: Decimal::from_ratio(
            DEFAULT_REFEREE_DISCOUNT_NUMERATOR,
            DEFAULT_REFEREE_DISCOUNT_DENOMINATOR,
        ),
    };

    let oracle_gr = OracleGuardRails {
        use_for_liquidations: true,
        mark_oracle_divergence: Decimal::percent(10),
        slots_before_stale: 1000,
        confidence_interval_max_size: Uint128::from(4u64),
        too_volatile_ratio: Number128::new(5),
    };

    let orderstate = OrderState {
        min_order_quote_asset_amount: Uint128::zero(),
        reward: Decimal::zero(),
        time_based_reward_lower_bound: Uint128::zero(), // minimum filler reward for time-based reward
    };
    let state = State {
        exchange_paused: false,
        funding_paused: false,
        admin_controls_prices: true,
        collateral_vault: addr_validate_to_lower(deps.api, &msg.collateral_vault).unwrap(),
        insurance_vault: addr_validate_to_lower(deps.api, &msg.insurance_vault).unwrap(),
        oracle: addr_validate_to_lower(deps.api, &msg.oracle)?,
        margin_ratio_initial: Uint128::from(2000u128),
        margin_ratio_maintenance: Uint128::from(500u128),
        margin_ratio_partial: Uint128::from(625u128),
        partial_liquidation_close_percentage: Decimal::percent(25),
        partial_liquidation_penalty_percentage: Decimal::percent(25),
        full_liquidation_penalty_percentage: Decimal::one(),
        full_liquidation_liquidator_share_denominator: 2000u64,
        max_deposit: Uint128::zero(),
        markets_length: 0u64,
        partial_liquidation_liquidator_share_denominator: 1u64,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADMIN.set(deps.branch(), Some(info.sender.clone()))?;
    STATE.save(deps.storage, &state)?;
    // STATE.load(deps.storage)?;

    FEESTRUCTURE.save(deps.storage, &fs)?;
    ORACLEGUARDRAILS.save(deps.storage, &oracle_gr)?;
    ORDERSTATE.save(deps.storage, &orderstate)?;

    LENGTH.save(deps.storage, &Length{
        curve_history_length: 0,
        deposit_history_length: 0,
        funding_payment_history_length: 0,
        funding_rate_history_length: 0,
        liquidation_history_length: 0,
        order_history_length: 0,
        trade_history_length: 0,
    })?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender.clone()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InitializeMarket {
            market_index,
            market_name,
            amm_base_asset_reserve,
            amm_quote_asset_reserve,
            amm_periodicity,
            amm_peg_multiplier,
            oracle_source,
            margin_ratio_initial,
            margin_ratio_partial,
            margin_ratio_maintenance,
        } => try_initialize_market(
            deps,
            _env,
            info,
            market_index,
            market_name,
            amm_base_asset_reserve,
            amm_quote_asset_reserve,
            amm_periodicity,
            amm_peg_multiplier,
            oracle_source,
            margin_ratio_initial,
            margin_ratio_partial,
            margin_ratio_maintenance,
        ),
        ExecuteMsg::DepositCollateral { amount, referrer } => {
            try_deposit_collateral(deps, _env, info, amount, referrer)
        }
        ExecuteMsg::WithdrawCollateral { amount } => {
            try_withdraw_collateral(deps, _env, info, amount)
        }
        ExecuteMsg::OpenPosition {
            direction,
            quote_asset_amount,
            market_index,
            limit_price,
        } => try_open_position(
            deps,
            _env,
            info,
            direction,
            quote_asset_amount,
            market_index,
            limit_price,
        ),
        ExecuteMsg::ClosePosition { market_index } => {
            try_close_position(deps, _env, info, market_index)
        }
        ExecuteMsg::Liquidate { user, market_index } => {
            try_liquidate(deps, _env, info, user, market_index)
        }
        ExecuteMsg::MoveAMMPrice {
            base_asset_reserve,
            quote_asset_reserve,
            market_index,
        } => try_move_amm_price(deps, base_asset_reserve, quote_asset_reserve, market_index),
        ExecuteMsg::WithdrawFees {
            market_index,
            amount,
        } => try_withdraw_fees(deps, info, market_index, amount),
        ExecuteMsg::WithdrawFromInsuranceVaultToMarket {
            market_index,
            amount,
        } => try_withdraw_from_insurance_vault_to_market(deps, info, market_index, amount),
        ExecuteMsg::RepegAMMCurve {
            new_peg_candidate,
            market_index,
        } => try_repeg_amm_curve(deps, _env, new_peg_candidate, market_index),
        ExecuteMsg::UpdateAMMOracleTwap { market_index } => {
            try_update_amm_oracle_twap(deps, _env, market_index)
        }
        ExecuteMsg::ResetAMMOracleTwap { market_index } => {
            try_reset_amm_oracle_twap(deps, _env, market_index)
        }
        ExecuteMsg::SettleFundingPayment {} => try_settle_funding_payment(deps, _env, info),
        ExecuteMsg::UpdateFundingRate { market_index } => {
            try_update_funding_rate(deps, _env, market_index)
        }
        ExecuteMsg::UpdateK {
            market_index,
            sqrt_k,
        } => try_update_k(deps, _env, market_index, sqrt_k),
        ExecuteMsg::UpdateMarginRatio {
            market_index,
            margin_ratio_initial,
            margin_ratio_partial,
            margin_ratio_maintenance,
        } => try_update_margin_ratio(
            deps,
            info,
            market_index,
            margin_ratio_initial,
            margin_ratio_partial,
            margin_ratio_maintenance,
        ),
        ExecuteMsg::UpdatePartialLiquidationClosePercentage { value } => {
            try_update_partial_liquidation_close_percentage(deps, info, value)
        }
        ExecuteMsg::UpdatePartialLiquidationPenaltyPercentage { value } => {
            try_update_partial_liquidation_penalty_percentage(deps, info, value)
        }
        ExecuteMsg::UpdateFullLiquidationPenaltyPercentage { value } => {
            try_update_full_liquidation_penalty_percentage(deps, info, value)
        }
        ExecuteMsg::UpdatePartialLiquidationLiquidatorShareDenominator { denominator } => {
            try_update_partial_liquidation_liquidator_share_denominator(deps, info, denominator)
        }
        ExecuteMsg::UpdateFullLiquidationLiquidatorShareDenominator { denominator } => {
            try_update_full_liquidation_liquidator_share_denominator(deps, info, denominator)
        }
        ExecuteMsg::UpdateFee {
            fee_: fee,
            first_tier_minimum_balance,
            first_tier_discount,
            second_tier_minimum_balance,
            second_tier_discount,
            third_tier_minimum_balance,
            third_tier_discount,
            fourth_tier_minimum_balance,
            fourth_tier_discount,
            referrer_reward,
            referee_discount,
        } => try_update_fee(
            deps,
            info,
            fee,
            first_tier_minimum_balance,
            first_tier_discount,
            second_tier_minimum_balance,
            second_tier_discount,
            third_tier_minimum_balance,
            third_tier_discount,
            fourth_tier_minimum_balance,
            fourth_tier_discount,
            referrer_reward,
            referee_discount,
        ),
        ExecuteMsg::UpdateOraceGuardRails {
            use_for_liquidations,
            mark_oracle_divergence,
            slots_before_stale,
            confidence_interval_max_size,
            too_volatile_ratio,
        } => try_update_oracle_guard_rails(
            deps,
            info,
            use_for_liquidations,
            mark_oracle_divergence,
            slots_before_stale,
            confidence_interval_max_size,
            too_volatile_ratio,
        ),
        ExecuteMsg::UpdateAdmin { admin } => {
            let addr = Some(deps.api.addr_validate(&admin)?);
            Ok(ADMIN.execute_update_admin(deps, info, addr)?)
        }
        ExecuteMsg::UpdateMaxDeposit { max_deposit } => {
            try_update_max_deposit(deps, info, max_deposit)
        }
        ExecuteMsg::UpdateExchangePaused { exchange_paused } => {
            try_update_exchange_paused(deps, info, exchange_paused)
        }
        ExecuteMsg::DisableAdminControlsPrices {} => try_disable_admin_control_prices(deps, info),
        ExecuteMsg::UpdateFundingPaused { funding_paused } => {
            try_update_funding_paused(deps, info, funding_paused)
        }
        ExecuteMsg::UpdateMarketMinimumQuoteAssetTradeSize {
            market_index,
            minimum_trade_size,
        } => try_update_market_minimum_quote_asset_trade_size(
            deps,
            info,
            market_index,
            minimum_trade_size,
        ),
        ExecuteMsg::UpdateMarketMinimumBaseAssetTradeSize {
            market_index,
            minimum_trade_size,
        } => try_update_market_minimum_base_asset_trade_size(
            deps,
            info,
            market_index,
            minimum_trade_size,
        ),
        ExecuteMsg::UpdateOrderState {
            min_order_quote_asset_amount,
            reward,
            time_based_reward_lower_bound,
        } => try_update_order_state_structure(
            deps,
            info,
            min_order_quote_asset_amount,
            reward,
            time_based_reward_lower_bound,
        ),
        ExecuteMsg::UpdateMarketOracle {
            market_index,
            oracle,
            oracle_source,
        } => try_update_market_oracle(deps, info, market_index, oracle, oracle_source),
        ExecuteMsg::UpdateOracleAddress { oracle } => try_update_oracle_address(deps, info, oracle),
        ExecuteMsg::OracleFeeder {
            market_index,
            price,
        } => try_feeding_price(deps, info, market_index, price),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetUser { user_address } => Ok(to_binary(&get_user(deps, user_address)?)?),
        QueryMsg::GetUserMarketPosition {
            user_address,
            index,
        } => Ok(to_binary(&get_user_position(deps, user_address, index)?)?),
        QueryMsg::GetAdmin {} => Ok(to_binary(&get_admin(deps)?)?),
        QueryMsg::IsExchangePaused {} => Ok(to_binary(&is_exchange_paused(deps)?)?),
        QueryMsg::IsFundingPaused {} => Ok(to_binary(&is_funding_paused(deps)?)?),
        QueryMsg::AdminControlsPrices {} => Ok(to_binary(&admin_controls_prices(deps)?)?),
        QueryMsg::GetVaults {} => Ok(to_binary(&get_vaults_address(deps)?)?),
        QueryMsg::GetMarginRatio {} => Ok(to_binary(&get_margin_ratios(deps)?)?),
        QueryMsg::GetPartialLiquidationClosePercentage {} => {
            Ok(to_binary(&get_partial_liquidation_close_percentage(deps)?)?)
        }
        QueryMsg::GetPartialLiquidationPenaltyPercentage {} => Ok(to_binary(
            &get_partial_liquidation_penalty_percentage(deps)?,
        )?),
        QueryMsg::GetFullLiquidationPenaltyPercentage {} => {
            Ok(to_binary(&get_full_liquidation_penalty_percentage(deps)?)?)
        }
        QueryMsg::GetPartialLiquidatorSharePercentage {} => {
            Ok(to_binary(&get_partial_liquidator_share_percentage(deps)?)?)
        }
        QueryMsg::GetFullLiquidatorSharePercentage {} => {
            Ok(to_binary(&get_full_liquidator_share_percentage(deps)?)?)
        }
        QueryMsg::GetMaxDepositLimit {} => Ok(to_binary(&get_max_deposit_limit(deps)?)?),
        QueryMsg::GetOracle {} => Ok(to_binary(&get_oracle_address(deps)?)?),
        QueryMsg::GetMarketLength {} => Ok(to_binary(&get_market_length2(deps)?)?),
        QueryMsg::GetOracleGuardRails {} => Ok(to_binary(&get_oracle_guard_rails(deps)?)?),
        QueryMsg::GetOrderState {} => Ok(to_binary(&get_order_state(deps)?)?),
        QueryMsg::GetFeeStructure {} => Ok(to_binary(&get_fee_structure(deps)?)?),
        QueryMsg::GetCurveHistory { start_after, limit } => {
            Ok(to_binary(&get_curve_history(deps, start_after, limit)?)?)
        }
        QueryMsg::GetDepositHistory {
            user_address,
            start_after,
            limit,
        } => Ok(to_binary(&get_deposit_history(
            deps,
            user_address,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetFundingPaymentHistory {
            user_address,
            start_after,
            limit,
        } => Ok(to_binary(&get_funding_payment_history(
            deps,
            user_address,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetFundingRateHistory { start_after, limit } => Ok(to_binary(
            &get_funding_rate_history(deps, start_after, limit)?,
        )?),
        QueryMsg::GetLiquidationHistory {
            user_address,
            start_after,
            limit,
        } => Ok(to_binary(&get_liquidation_history(
            deps,
            user_address,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetTradeHistory { start_after, limit } => {
            Ok(to_binary(&get_trade_history(deps, start_after, limit)?)?)
        }
        QueryMsg::GetMarketInfo { market_index } => {
            Ok(to_binary(&get_market_info(deps, market_index)?)?)
        }
        QueryMsg::GetLength { } => {
            Ok(to_binary(&get_length(deps)?)?)
        }
    }
}

fn get_market_length2(deps: Deps) -> Result<MarketLengthResponse, ContractError> {
    let k = STATE.load(deps.storage)?;
    Ok(MarketLengthResponse {
        length: k.markets_length,
    })
}
