use std::env;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg, ConfigResponse, PriceResponse, InfoResponse};
use crate::state::{Config, CONFIG, ASSETS, Price, FEEDERS};
// use terra_cosmwasm::{ TerraQuerier, ExchangeRatesResponse };

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = Config {
        admin: info.sender.clone(),
        base_denom: "uusd".to_string(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RegisterAsset { asset, price_feeder } => try_register_asset(deps, info,env, asset, price_feeder),
        ExecuteMsg::RevokeAsset { asset } => try_revoke_asset(deps, info, asset),
        ExecuteMsg::FeedPrice { asset, price } => try_feed_price(deps, info, env, asset, price),
    }
}

pub fn try_register_asset(deps: DepsMut, info: MessageInfo, env: Env, asset: String, price_feeder: Addr) -> Result<Response, ContractError> {

    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    ASSETS.save(deps.storage, asset.clone().into(), &Price{
        price: 0,
        last_updated : env.block.time.seconds(),
    })?;

    FEEDERS.save(deps.storage, asset.clone().into(), &price_feeder)?;

    Ok(Response::new()
        .add_attribute("method", "register_asset")
        .add_attribute("asset", asset.clone())
        .add_attribute("feeder", price_feeder))
    
}

pub fn try_revoke_asset(deps: DepsMut, info: MessageInfo, asset: String) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    ASSETS.remove(deps.storage, asset.clone().into());
    FEEDERS.remove(deps.storage, asset);

    Ok(Response::new().add_attribute("method", "try_increment"))
}

pub fn try_feed_price(deps: DepsMut, info: MessageInfo, env: Env, asset: String, price : u128) -> Result<Response, ContractError> {
    let feeder = FEEDERS.load(deps.storage, asset.clone())?;
    if info.sender != feeder {
        return Err(ContractError::Unauthorized {});
    }

    ASSETS.update(deps.storage, asset, |_a| -> Result<Price, ContractError>{
        Ok(Price {
            price: price,
            last_updated: env.block.time.seconds(),
        })
    })?;

    Ok(Response::new().add_attribute("method", "feed_price"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Price {asset} => to_binary(&query_price(deps, asset)?),
        // QueryMsg::PriceLuna {} => to_binary(&query_price_luna(deps)?),
        QueryMsg::AssetInfo {asset} => to_binary(&query_asset(deps, asset)?),
        
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse{
        owner: state.admin,
        base_denom: state.base_denom,
    })
}

fn query_price(deps: Deps, asset:String) -> StdResult<PriceResponse> {
    let price = ASSETS.load(deps.storage, asset.clone())?;
    Ok(PriceResponse{
        asset: asset,
        price: price.price,
        last_updated: price.last_updated,
    })
}

// fn query_price_luna(deps: Deps) -> StdResult<PriceResponse> {
//     let querier = TerraQuerier::new(&deps.querier);
//     let exchange_rates: ExchangeRatesResponse = querier.query_exchange_rates("uusd", vec!["uluna", "ukrw"])?;
//     Ok(PriceResponse{
//         asset: "uluna",
//         price: exchange_rates.exchange_rates[0].exchange_rate,
//         last_updated: 0,
//     })
// }

fn query_asset(deps: Deps, asset:String) -> StdResult<InfoResponse> {
    let price = ASSETS.load(deps.storage, asset.clone())?;
    let feeder = FEEDERS.load(deps.storage, asset.clone())?;
    Ok(InfoResponse{
        asset: asset,
        feeder: feeder,
        price: price.price,
        last_updated: price.last_updated,
    })
}
