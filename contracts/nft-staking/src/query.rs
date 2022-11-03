#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Env, StdResult, Deps};
use crate::ContractError;
use crate::handler::{compute_rewards, staker_tokenid_key, query_rewards_token_balance};
use crate::msg::{QueryMsg, ConfigResponse, StartTimeResponse, TotalRewardsPoolResponse, StakerHistoryResponse, TokenInfosResponse, RewardsScheduleResponse, EstimateRewardsResponse, NextClaimResponse, WithdrawRewardsPoolResponse, DisableResponse};
use crate::state::{CONFIG_STATE, REWARDS_SCHEDULE, START_TIMESTAMP, DISABLE, TOTAL_REWARDS_POOL, Snapshot, STAKER_HISTORIES, TokenInfo, TOKEN_INFOS, Claim, NEXT_CLAIMS, NextClaim};

const SUCCESS: &str = "success";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps,
            env: Env,
            msg: QueryMsg
        ) -> StdResult<Binary> {
        match msg {
            QueryMsg::GetConfig {} => to_binary(&get_config(deps)?),
            QueryMsg::GetRewardsSchedule {} => to_binary(&get_rewards_schedule(deps)?),
            QueryMsg::StartTime {} => to_binary(&start_time(deps)?),
            QueryMsg::Disable {} => to_binary(&disable(deps)?),
            QueryMsg::TotalRewardsPool {} => to_binary(&total_rewards_pool(deps)?),
            QueryMsg::WithdrawRewardsPoolAmount {} => to_binary(&withdraw_rewards_pool_amount(deps, env)?),
            QueryMsg::StakerHistory { staker, token_id } => to_binary(&staker_history(deps, staker, token_id)?),
            QueryMsg::TokenInfo { token_id } => to_binary(&token_infos(deps, token_id)?),
            QueryMsg::EstimateRewards { max_period, staker, token_id } => to_binary(&estimate_rewards(deps, env, max_period, token_id, staker)?),
            QueryMsg::NextClaim { staker, token_id } => to_binary(&next_claims(deps, staker, token_id)?),
        }
}

fn get_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config_state = CONFIG_STATE.load(deps.storage)?;
    Ok(ConfigResponse { 
        owner: config_state.owner.to_string(), 
        cycle_length_in_seconds: config_state.cycle_length_in_seconds,
        period_length_in_cycles: config_state.period_length_in_cycles,
        white_listed_nft_contract: config_state.white_listed_nft_contract.to_string(),
        rewards_token_contract: config_state.rewards_token_contract.to_string(),
    })
}

fn get_rewards_schedule(
    deps: Deps
) -> StdResult<RewardsScheduleResponse> {
    let res: RewardsScheduleResponse;
    let rewards_schedule = REWARDS_SCHEDULE.may_load(deps.storage)?;

    if !rewards_schedule.is_none() {
        res = RewardsScheduleResponse {
            rewards_per_cycle: rewards_schedule.unwrap(),
            res_msg: SUCCESS.to_string()
        };
    } else {
        res = RewardsScheduleResponse {
            rewards_per_cycle: 0,
            res_msg: ContractError::NoneRewardsSchedule {}.to_string()
        };
    }

    Ok(res)
}

fn start_time(
    deps: Deps,
) -> StdResult<StartTimeResponse> {
    let start_bool: bool;
    let start_time: u64;
    let res_msg: String;

    let start_timestamp = START_TIMESTAMP.may_load(deps.storage)?;
    if start_timestamp.is_none() {
        start_bool = false;
        start_time = 0;
        res_msg = ContractError::NotStarted {}.to_string()
    } else {
        start_bool = true;
        start_time = start_timestamp.unwrap();
        res_msg = SUCCESS.to_string();
    }

    let res = StartTimeResponse {
        start: start_bool,
        start_time: start_time,
        res_msg: res_msg
    };

    Ok(res)
}

fn disable(
    deps: Deps,
) -> StdResult<DisableResponse> {
    let disable = DISABLE.load(deps.storage)?;
    let res = DisableResponse {
        disable: disable,
        res_msg: SUCCESS.to_string(),
    };

    Ok(res)
}

fn total_rewards_pool (
    deps: Deps,
) -> StdResult<TotalRewardsPoolResponse> {
    let pool: u128;
    let res_msg: String;

    let total_rewards_pool = TOTAL_REWARDS_POOL.may_load(deps.storage)?;
    if !total_rewards_pool.is_none() {
        pool = total_rewards_pool.unwrap();
        res_msg = SUCCESS.to_string();
    } else {
        pool = 0;
        res_msg = ContractError::EmptyRewardsPool {}.to_string();
    }

    let res = TotalRewardsPoolResponse {
        total_rewards_pool: pool,
        res_msg: res_msg
    };

    Ok(res)
}

fn withdraw_rewards_pool_amount (
    deps: Deps,
    env: Env,
) -> StdResult<WithdrawRewardsPoolResponse> {
    let address = env.contract.address.to_string();
    let withdraw_rewards_pool_amount: u128;
    let res_msg: String;

    let config = get_config(deps)?;
    let balance_response = query_rewards_token_balance(deps, address, config.rewards_token_contract);
    match balance_response {
        Ok(t) => {
            withdraw_rewards_pool_amount = t.balance.u128();
            res_msg = SUCCESS.to_string();
        },
        Err(e) => {
            withdraw_rewards_pool_amount = 0;
            res_msg = e.to_string();
        }
    }

    let res = WithdrawRewardsPoolResponse {
        withdraw_rewards_pool_amount: withdraw_rewards_pool_amount,
        res_msg: res_msg,
    };

    Ok(res)
}

fn next_claims(
    deps: Deps,
    staker: String,
    token_id: String,
) -> StdResult<NextClaimResponse> {
    let staker_tokenid_key = staker_tokenid_key(staker, token_id);
    let res_msg: String;

    let next_claim: NextClaim;
    let next_claims = NEXT_CLAIMS.may_load(deps.storage, staker_tokenid_key)?;
    if next_claims.is_none() {
        next_claim = NextClaim {
            period: 0,
            staker_snapshot_index: 0,
        };
        res_msg = ContractError::EmptyNextClaim {}.to_string()
    } else {
        next_claim = next_claims.unwrap();
        res_msg = SUCCESS.to_string();
    }

    let res = NextClaimResponse {
        next_claim: next_claim,
        res_msg: res_msg,
    };

    Ok(res)
}

fn staker_history (
    deps: Deps,
    staker: String,
    token_id: String,
) -> StdResult<StakerHistoryResponse> {
    let history: Vec<Snapshot>;
    let res_msg: String;

    let staker_tokenid_key = staker_tokenid_key(staker, token_id);
    let staker_history = STAKER_HISTORIES.may_load(deps.storage, staker_tokenid_key.clone())?;

    if !staker_history.is_none() {
        history = staker_history.unwrap();
        res_msg = SUCCESS.to_string();

    } else {
        history = vec![];
        res_msg = ContractError::HaveNotHistory {}.to_string();
    }

    let res = StakerHistoryResponse {
        staker_tokenid_key: staker_tokenid_key,
        staker_history: history,
        res_msg: res_msg,
    };

    Ok(res)
}

fn token_infos (
    deps: Deps,
    token_id: String,
) -> StdResult<TokenInfosResponse> {
    let token_info: TokenInfo;
    let res_msg: String;

    let token_infos = TOKEN_INFOS.may_load(deps.storage, token_id.clone())?;

    if !token_infos.is_none() {
        if token_infos.clone().unwrap().is_staked {
            token_info = token_infos.unwrap();
            res_msg = SUCCESS.to_string();
        } else {
            token_info = token_infos.unwrap();
            res_msg = ContractError::UnstakedTokenId {}.to_string();
        }
    } else {
        token_info = TokenInfo {
            owner: "".to_string(),
            is_staked: false,
            deposit_cycle: 0,
            withdraw_cycle: 0
        };
        res_msg = ContractError::InvalidTokenId {}.to_string()
    }

    let res = TokenInfosResponse {
        token_id: token_id,
        token_info: token_info,
        res_msg: res_msg,
    };

    Ok(res)
}

pub fn estimate_rewards(
    deps: Deps,
    env: Env,
    max_period: u64,
    token_id: String,
    staker: String,
) -> StdResult<EstimateRewardsResponse> {
    let return_claim = Claim {
        periods: 0,
        start_period: 0,
        amount: 0,
    };

    let staker_tokenid_key = staker_tokenid_key(staker.clone(), token_id.clone());
    let next_claim = NEXT_CLAIMS.may_load(deps.storage, staker_tokenid_key.clone())?;
    if next_claim.is_none() {
        return Ok(EstimateRewardsResponse { 
            req_staker_tokenid_key: staker_tokenid_key, 
            claim: return_claim,
            res_msg: ContractError::InvalidClaim {}.to_string()
        })
    }

    let start_timestamp = START_TIMESTAMP.may_load(deps.storage)?;
    if start_timestamp.is_none() {
        return Ok(EstimateRewardsResponse { 
            req_staker_tokenid_key: staker_tokenid_key, 
            claim: return_claim,
            res_msg: ContractError::NotStarted {}.to_string()
        })
    }

    let disable = DISABLE.load(deps.storage)?;
    if disable == true {
        return Ok(EstimateRewardsResponse { 
            req_staker_tokenid_key: staker_tokenid_key, 
            claim: return_claim,
            res_msg: ContractError::Disabled {}.to_string() 
        })
    }

    let config = CONFIG_STATE.load(deps.storage)?;
    let now = env.block.time.seconds();

    let claim: Claim;
    let compute_rewards = compute_rewards(deps, staker_tokenid_key.clone(), max_period, now, start_timestamp.unwrap(), config.clone());
    match compute_rewards {
        Ok(t) => {
            claim = t.0;

        },
        Err(e) => {
            return Ok(EstimateRewardsResponse { 
                req_staker_tokenid_key: staker_tokenid_key, 
                claim: return_claim,
                res_msg: e.to_string()
            })
        }
    }

    Ok(EstimateRewardsResponse { 
        req_staker_tokenid_key: staker_tokenid_key, 
        claim: claim,
        res_msg: SUCCESS.to_string()
    })
}