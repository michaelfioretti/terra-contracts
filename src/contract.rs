#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{OwnerResponse, ScoreResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE, SCORES};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:example-terra-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {        
        owner: info.sender.clone()
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateScore {user, score} => try_update_score(deps, info, user, score)
    }
}

pub fn try_update_score(deps: DepsMut, info: MessageInfo, user: Addr, score: u32) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    let current_score = SCORES.may_load(deps.storage, user.to_string())?.unwrap_or_default();

    if current_score == 0 {
        SCORES.save(deps.storage, user.to_string(), &score);
    } else {
        SCORES.update(deps.storage, user.to_string(), |score: Option<u32>| -> StdResult<_> { 
            Ok(score.unwrap_or_default())
        })?;
    }
    
    Ok(Response::new().add_attribute("method", "try_update_score"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
        QueryMsg::GetScore { user } => to_binary(&query_score(deps, user)?)
    }
}

fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(OwnerResponse { owner: state.owner })
}

fn query_score(deps: Deps, user: String) -> StdResult<ScoreResponse>  {
    let score = SCORES.may_load(deps.storage, user)?.unwrap_or_default();
    Ok(ScoreResponse{ score })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    fn get_score<T: Into<String>>(deps: Deps, address: T) -> u32 {
        query_score(deps, address.into()).unwrap().score
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        // let msg = InstantiateMsg { count: 17 };
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Query and validate the owner was set correctly
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOwner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("creator", value.owner);
    }
    
    #[test]
    // Sets a specific user's score
    fn set_user_score() {
        let mut deps = mock_dependencies_with_balance(&coins(10, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Set a user's score
        let info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::UpdateScore { user: info.sender.clone(), score: 1120 };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(get_score(deps.as_ref(), "creator"), 1120);

        // Attempting to set a user's score with someone other than the owner will fail
        let info = mock_info("someone_new", &coins(2, "token"));
        let msg = ExecuteMsg::UpdateScore { user: info.sender.clone(), score: 500 };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    // Get token balances of users
    fn get_token_balances_of_users() {
        let mut deps = mock_dependencies_with_balance(&coins(10, "token"));

        let msg = InstantiateMsg {};
        let instantiate_info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), instantiate_info, msg).unwrap();

        // Set creator
        let creator_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::UpdateScore { user: creator_info.sender.clone(), score: 123 };
        let _res = execute(deps.as_mut(), mock_env(), creator_info, msg).unwrap();

        // Set someone else
        let creator_info = mock_info("creator", &coins(2, "token"));
        let new_human = mock_info("new_human", &coins(10, "token"));
        let msg = ExecuteMsg::UpdateScore { user: new_human.sender.clone(), score: 456 };
        let _res = execute(deps.as_mut(), mock_env(), creator_info, msg).unwrap();
        
        // Fetch creator
        let creator_info = mock_info("creator", &coins(10, "token"));
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetScore {user: creator_info.sender.to_string()}).unwrap();
        let value: ScoreResponse = from_binary(&res).unwrap();
        println!("{}", value.score);
        assert_eq!(123, value.score);

        // Fetch new human
        let new_human = mock_info("new_human", &coins(10, "token"));
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetScore {user: new_human.sender.to_string()}).unwrap();
        let value: ScoreResponse = from_binary(&res).unwrap();
        assert_eq!(456, value.score);
    }

    #[test]
    // Get the owner of the contract
    fn get_owner() {
        let mut deps = mock_dependencies_with_balance(&coins(10, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Fetch here
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOwner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("creator", value.owner);
    }
}
