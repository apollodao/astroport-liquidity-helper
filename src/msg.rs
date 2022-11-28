use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Addr, Binary, CosmosMsg, Env, StdResult, Uint128, WasmMsg};
use cw_asset::{AssetList, AssetListUnchecked};
use cw_dex::astroport::AstroportPool;

#[cw_serde]
pub struct InstantiateMsg {
    pub astroport_factory: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    BalancingProvideLiquidity {
        assets: AssetListUnchecked,
        min_out: Uint128,
        pool: Binary,
        recipient: Option<String>,
    },
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum CallbackMsg {
    ProvideLiquidity {
        assets: AssetList,
        min_out: Uint128,
        pool: AstroportPool,
        recipient: Addr,
    },
    ReturnLpTokens {
        pool: AstroportPool,
        balance_before: Uint128,
        recipient: Addr,
    },
}

impl CallbackMsg {
    pub fn into_cosmos_msg(&self, env: &Env) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    AstroportFactory {},
}

#[cw_serde]
pub enum MigrateMsg {}
