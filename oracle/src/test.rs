#[cfg(test)]
mod tests {
    use crate::contract::{instantiate, query, execute};
    use crate::msg::{InstantiateMsg, QueryMsg, ConfigResponse, ExecuteMsg};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg { };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("creator", value.owner);
    }

    // #[test]
    // fn feed_and_read() {
    //     let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    //     let msg = InstantiateMsg { };
    //     let info = mock_info("creator", &coins(2, "token"));
    //     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // beneficiary can release it
    //     let info = mock_info("feeder", &coins(2, "token"));
    //     let msg = ExecuteMsg::FeedPrice { asset: "uluna".to_string(), price: 1000000000 };
    //     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // should increase counter by 1
    //     // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     // let value: CountResponse = from_binary(&res).unwrap();
    //     // assert_eq!(18, value.count);
    // }

}
