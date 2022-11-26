use std::str::FromStr;

use astroport::asset::{Asset as AstroAsset, AssetInfo as AstroAssetInfo};
use astroport::factory::PairType;
use astroport::pair::{ExecuteMsg as PairExecuteMsg, QueryMsg as PairQueryMsg};
use astroport_liquidity_helper::{helpers::LiquidityHelper, msg::InstantiateMsg};
use cosmwasm_std::{to_binary, Addr, Coin, Decimal, Uint128};
use cw20::{AllowanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_asset::{Asset, AssetInfo, AssetList};
use cw_dex::astroport::msg::PoolResponse;
use cw_dex::astroport::AstroportPool;
use cw_it::astroport::{create_astroport_pair, instantiate_astroport};
use cw_it::Cli;
use cw_it::{app::App as RpcRunner, astroport::AstroportContracts};
use osmosis_testing::{
    cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContractResponse, Account, Module, Runner,
    SigningAccount, Wasm,
};

const TEST_CONFIG_PATH: &str = "tests/configs/terra.yaml";
pub const ASTROPORT_LIQUIDITY_HELPER_WASM_FILE: &str = "artifacts/astroport_liquidity_helper.wasm";

#[test]
/// Runs all tests against LocalTerra
pub fn test_with_localterra() {
    // let _ = env_logger::builder().is_test(true).try_init();
    let docker: Cli = Cli::default();
    let app = RpcRunner::new(TEST_CONFIG_PATH, &docker);

    let accs = app
        .test_config
        .import_all_accounts()
        .into_values()
        .collect::<Vec<_>>();

    // Instantiate Astroport contracts
    let astroport_contracts = instantiate_astroport(&app, &accs[0]);
    println!("astroport_contracts: {:?}", astroport_contracts);

    // Test basic liquidity helper functionality
    test_balancing_provide_liquidity(&app, accs, astroport_contracts);
}

/// Instantiates the liquidity helper contract
pub fn setup_astroport_liquidity_provider_tests<R>(
    app: &R,
    accs: &[SigningAccount],
    astroport_contracts: &AstroportContracts,
) -> LiquidityHelper
where
    R: for<'a> Runner<'a>,
{
    let wasm = Wasm::new(app);
    let admin = &accs[0];

    // Load compiled wasm bytecode
    let astroport_liquidity_helper_wasm_byte_code =
        std::fs::read(ASTROPORT_LIQUIDITY_HELPER_WASM_FILE).unwrap();
    let astroport_liquidity_helper_code_id = wasm
        .store_code(&astroport_liquidity_helper_wasm_byte_code, None, admin)
        .unwrap()
        .data
        .code_id;

    // Instantiate the contract
    let astroport_liquidity_helper = wasm
        .instantiate(
            astroport_liquidity_helper_code_id,
            &InstantiateMsg {
                astroport_factory: astroport_contracts.factory.address.clone(),
            },
            Some(&admin.address()), // contract admin used for migration
            Some("Astroport Liquidity Helper"), // contract label
            &[],                    // funds
            admin,                  // signer
        )
        .unwrap()
        .data
        .address;

    let liquidity_helper = LiquidityHelper::new(Addr::unchecked(astroport_liquidity_helper));

    liquidity_helper
}

/// Tests the BalancingProvideLiquidity message
pub fn test_balancing_provide_liquidity<R>(
    app: &R,
    accs: Vec<SigningAccount>,
    astroport_contracts: AstroportContracts,
) where
    R: for<'a> Runner<'a>,
{
    let liquidity_helper =
        setup_astroport_liquidity_provider_tests(app, &accs, &astroport_contracts);
    let wasm = Wasm::new(app);
    let admin = &accs[0];
    let astro_token = astroport_contracts.astro_token.address.clone();

    // Create 1:1 XYK pool
    let asset_infos: [AstroAssetInfo; 2] = [
        AstroAssetInfo::NativeToken {
            denom: "uluna".into(),
        },
        AstroAssetInfo::Token {
            contract_addr: Addr::unchecked(&astro_token),
        },
    ];
    let (uluna_astro_pair_addr, uluna_astro_lp_token) = create_astroport_pair(
        app,
        &astroport_contracts.factory.address,
        PairType::Xyk {},
        asset_infos.clone(),
        None,
        admin,
    );
    let pool = AstroportPool {
        lp_token_addr: Addr::unchecked(uluna_astro_lp_token.clone()),
        pair_addr: Addr::unchecked(uluna_astro_pair_addr.clone()),
        pair_type: cw_dex::astroport::msg::PairType::Xyk {},
    };

    // Increase allowance of astro token for Pair contract
    let increase_allowance_msg = Cw20ExecuteMsg::IncreaseAllowance {
        spender: uluna_astro_pair_addr.clone(),
        amount: Uint128::from(1000000000u128),
        expires: None,
    };
    let _res = wasm
        .execute(&astro_token, &increase_allowance_msg, &vec![], admin)
        .unwrap();

    // Query allowance
    let allowance_res: AllowanceResponse = wasm
        .query(
            &astro_token,
            &Cw20QueryMsg::Allowance {
                owner: admin.address().to_string(),
                spender: uluna_astro_pair_addr.clone(),
            },
        )
        .unwrap();
    assert_eq!(allowance_res.allowance, Uint128::from(1000000000u128));

    // Provide liquidity normal to have some liquidity in pool
    let provide_liq_msg = PairExecuteMsg::ProvideLiquidity {
        assets: [
            AstroAsset {
                amount: Uint128::from(1000000000u128),
                info: AstroAssetInfo::NativeToken {
                    denom: "uluna".into(),
                },
            },
            AstroAsset {
                amount: Uint128::from(1000000000u128),
                info: AstroAssetInfo::Token {
                    contract_addr: Addr::unchecked(&astro_token),
                },
            },
        ],
        slippage_tolerance: Some(Decimal::from_str("0.02").unwrap()),
        auto_stake: Some(false),
        receiver: None,
    };
    let _res = wasm.execute(
        &uluna_astro_pair_addr,
        &provide_liq_msg,
        &vec![Coin {
            amount: Uint128::from(1000000000u128),
            denom: "uluna".into(),
        }],
        admin,
    );

    // Balancing Provide liquidity
    println!("Balancing provide liquidity");
    let mut assets: AssetList = vec![Coin::new(100_000, "uluna")].into();
    assets
        .add(&Asset::new(
            AssetInfo::Cw20(Addr::unchecked(&astro_token)),
            Uint128::from(100_000u128),
        ))
        .unwrap();
    let msgs = liquidity_helper
        .balancing_provide_liquidity(assets, Uint128::one(), to_binary(&pool).unwrap())
        .unwrap();
    let _res = app
        .execute_cosmos_msgs::<MsgExecuteContractResponse>(&msgs, admin)
        .unwrap();

    // Check pool liquidity after adding
    let mut initial_pool_liquidity = AssetList::new();
    initial_pool_liquidity
        .add(&Asset::native("uluna", Uint128::from(1000000000u128)))
        .unwrap()
        .add(&Asset::new(
            AssetInfo::Cw20(Addr::unchecked(&astro_token)),
            Uint128::from(1000000000u128),
        ))
        .unwrap();
    let expected_liquidity_after_add = initial_pool_liquidity
        .add(&Asset::native("uluna", Uint128::from(100_000u128)))
        .unwrap()
        .add(&Asset::new(
            AssetInfo::Cw20(Addr::unchecked(&astro_token)),
            Uint128::from(100_000u128),
        ))
        .unwrap();
    let pool_liquidity: PoolResponse = wasm
        .query(&uluna_astro_pair_addr, &PairQueryMsg::Pool {})
        .unwrap();
    let pool_liquidity: AssetList = pool_liquidity.assets.to_vec().into();
    assert_eq!(&pool_liquidity, expected_liquidity_after_add);
}
