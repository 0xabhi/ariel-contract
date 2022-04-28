use crate::helpers::amm::use_oracle_price_for_margin_calculation;
use crate::helpers::position::{calculate_updated_collateral, calculate_slippage};
use crate::states::constants::{
    AMM_TO_QUOTE_PRECISION_RATIO, DEFAULT_LIMIT, MARGIN_PRECISION, MARK_PRICE_PRECISION, MAX_LIMIT,
};
use crate::helpers::oracle::get_oracle_status;
use crate::helpers::position::{
    calculate_base_asset_value_and_pnl, calculate_base_asset_value_and_pnl_with_oracle_price,
    direction_to_close_position,
};
use crate::ContractError;
// use crate::helpers::casting::cast_to_i64;
use crate::states::history::*;
use crate::states::market::{LiquidationStatus, LiquidationType, MarketStatus, MARKETS};
use crate::states::state::{ADMIN, STATE, ORACLEGUARDRAILS, ORDERSTATE, FEESTRUCTURE, LENGTH};
use crate::states::user::{POSITIONS, USERS};

use crate::package::helper::addr_validate_to_lower;

use crate::package::number::Number128;
use crate::package::response::*;

use crate::package::types::{OracleGuardRails, PositionDirection};
use cosmwasm_std::{Addr, Deps, Order, Uint128};
use cw_storage_plus::{Bound, PrimaryKey};

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
    // let length = MarketLengthResponse {
    //     length: state.markets_length,
    // };
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

// get list in response
// pub fn get_active_positions(
//     deps: Deps,
//     user_address: String,
//     start_after: Option<String>,
//     limit: Option<u32>,
// ) -> Result<Vec<PositionResponse>, ContractError> {
//     let user_addr = addr_validate_to_lower(deps.api, user_address.as_str())?;
    
//     let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
//     let start = start_after
//         .map(|start| start.joined_key())
//         .map(Bound::Exclusive);

//     let active_positions : Vec<UserPositionResponse> = POSITIONS
//         .prefix(&user_addr)
//         .range(deps.storage, start, None, Order::Ascending)
//         .filter_map(|positions| {
//             positions.ok().map(|position| UserPositionResponse {
//                 base_asset_amount: position.1.base_asset_amount,
//                 quote_asset_amount: position.1.quote_asset_amount,
//                 last_cumulative_funding_rate: position.1.last_cumulative_funding_rate,
//                 last_cumulative_repeg_rebate: position.1.last_cumulative_repeg_rebate,
//                 last_funding_rate_ts: position.1.last_funding_rate_ts
//             })
//         })
//         .take(limit)
//         .collect();
//     // }
        
//     let mut positions: Vec<PositionResponse> = vec![];
//     for position in active_positions.clone() {
//         if position.base_asset_amount.i128().unsigned_abs() == 0{
//             continue;
//         }
//         let mut direction = direction_to_close_position(position.base_asset_amount.i128());
//         if direction == PositionDirection::Long {
//             direction = PositionDirection::Short;
//         }
//         else{
//             direction = PositionDirection::Long;
//         }
//         let entry_price: Uint128 = (position
//             .quote_asset_amount
//             .checked_mul(MARK_PRICE_PRECISION * AMM_TO_QUOTE_PRECISION_RATIO))?
//         .checked_div(Uint128::from(
//             position.base_asset_amount.i128().unsigned_abs(),
//         ))?;

//         let entry_notional = position.quote_asset_amount;
//         let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
//         let liq_status =
//             calculate_liquidation_status(&deps, &user_addr, &oracle_guard_rails).unwrap();
//         let pr = PositionResponse {
//             direction,
//             initial_size: Uint128::from(position.base_asset_amount.i128().unsigned_abs()),
//             entry_notional: Number128::new(entry_notional.u128() as i128),
//             entry_price,
//             pnl: Number128::new(liq_status.unrealized_pnl),
//             base_asset_amount: position.base_asset_amount,
//             quote_asset_amount: position.quote_asset_amount,
//             last_cumulative_funding_rate: position.last_cumulative_funding_rate,
//             last_cumulative_repeg_rebate: position.last_cumulative_repeg_rebate,
//             last_funding_rate_ts: position.last_funding_rate_ts
//         };
//         positions.push(pr);
//     }

//     Ok(positions)
// }

pub fn calculate_liquidation_status(
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
