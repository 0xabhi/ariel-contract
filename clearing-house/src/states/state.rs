use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::Item;

use crate::package::types::{FeeStructure, OracleGuardRails};

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