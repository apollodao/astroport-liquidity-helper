use std::fmt::{ Display, Formatter, Result };
use astroport_core::factory::PairType;

use cosmwasm_schema::{ cw_serde, QueryResponses };
use cw_asset::AssetList;
use schemars::JsonSchema;
use serde::{ Deserialize, Serialize };

use cosmwasm_std::{
    to_binary,
    Addr,
    Binary,
    CosmosMsg,
    StdResult,
    Uint128,
    WasmMsg,
    QuerierWrapper,
    Decimal,
};

use crate::msg::ExecuteMsg;

/// This structure describes the available query messages for the factory contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum FactoryQueryMsg {
    /// FeeInfo returns fee parameters for a specific pair. The response is returned using a [`FeeInfoResponse`] structure
    #[returns(FeeInfoResponse)]
    FeeInfo {
        /// The pair type for which we return fee information. Pair type is a [`PairType`] struct
        pair_type: PairType,
    },
}

// /// This enum describes available pair types.
// /// ## Available pool types
// /// ```
// /// # use astroport::factory::PairType::{Custom, Stable, Xyk};
// /// Xyk {};
// /// Stable {};
// /// Custom(String::from("Custom"));
// /// ```
// #[cw_serde]
// pub enum PairType {
//     /// XYK pair type
//     Xyk {},
//     /// Stable pair type
//     Stable {},
//     /// Custom pair type
//     Custom(String),
// }

// /// Returns a raw encoded string representing the name of each pool type
// impl Display for PairType {
//     fn fmt(&self, fmt: &mut Formatter) -> Result {
//         match self {
//             PairType::Xyk {} => fmt.write_str("xyk"),
//             PairType::Stable {} => fmt.write_str("stable"),
//             PairType::Custom(pair_type) => fmt.write_str(format!("custom-{}", pair_type).as_str()),
//         }
//     }
// }

/// This structure holds parameters that describe the fee structure for a pool.
pub struct FeeInfo {
    /// The fee address
    pub fee_address: Option<Addr>,
    /// The total amount of fees charged per swap
    pub total_fee_rate: Decimal,
    /// The amount of fees sent to the Maker contract
    pub maker_fee_rate: Decimal,
}

/// A custom struct for each query response that returns an object of type [`FeeInfoResponse`].
#[cw_serde]
pub struct FeeInfoResponse {
    /// Contract address to send governance fees to
    pub fee_address: Option<Addr>,
    /// Total amount of fees (in bps) charged on a swap
    pub total_fee_bps: u16,
    /// Amount of fees (in bps) sent to the Maker contract
    pub maker_fee_bps: u16,
}

/// Returns the fee information for a specific pair type.
///
/// * **pair_type** pair type we query information for.
pub fn query_fee_info(
    querier: &QuerierWrapper,
    factory_contract: impl Into<String>,
    pair_type: PairType
) -> StdResult<FeeInfo> {
    let res: FeeInfoResponse = querier.query_wasm_smart(
        factory_contract,
        &(FactoryQueryMsg::FeeInfo { pair_type })
    )?;

    Ok(FeeInfo {
        fee_address: res.fee_address,
        total_fee_rate: Decimal::from_ratio(res.total_fee_bps, 10000u16),
        maker_fee_rate: Decimal::from_ratio(res.maker_fee_bps, 10000u16),
    })
}

/// LiquidityHelper is a wrapper around Addr that provides a lot of helpers
/// for working with this contract. It can be imported by other contracts
/// who wish to call this contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct LiquidityHelper(pub Addr);

impl LiquidityHelper {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(
            (WasmMsg::Execute {
                contract_addr: self.addr().into(),
                msg,
                funds: vec![],
            }).into()
        )
    }

    pub fn balancing_provide_liquidity(
        &self,
        assets: AssetList,
        min_out: Uint128,
        pool: Binary
    ) -> StdResult<CosmosMsg> {
        self.call(ExecuteMsg::BalancingProvideLiquidity {
            assets: assets.into(),
            min_out,
            pool,
        })
    }
}