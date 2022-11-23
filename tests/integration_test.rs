use astroport::factory::InstantiateMsg as AstroportFactoryInstantiateMsg;
use astroport_liquidity_helper::{helpers::LiquidityHelper, msg::InstantiateMsg};
use cosmwasm_std::{to_binary, Addr, Coin, Uint128};
use cw_dex::osmosis::OsmosisPool;
use cw_it::app::App;
use cw_multi_test::Wasm;
use astroport::generator::InstantiateMsg as GeneratorInstantiateMsg;
use testcontainers::clients::Cli;
pub const ASTROPORT_LIQUIDITY_HELPER_WASM_FILE: &str = "artifacts/astroport_liquidity_helper.wasm";
pub const ASTROPORT_FACTORY_WASM_FILE: &str = "artifacts/astroport_factory-aarch64.wasm";
pub const ASTROPORT_GENERATOR_WASM_FILE: &str = "artifacts/astroport_generator-aarch64.wasm";
pub const CW1_WASM_FILE: &str = "artifacts/cw1_whitelist.wasm";
pub const CW20_WASM_FILE: &str = "artifacts/cw20_base.wasm";
const TEST_CONFIG_PATH: &str = "tests/configs/terra.yaml";

/// Merges a list of list of coins into a single list of coins, adding amounts
/// when denoms are the same.
fn merge_coins(coins: &[&[Coin]]) -> Vec<Coin> {
    let mut merged: Vec<Coin> = vec![];
    for coin_list in coins {
        for coin in *coin_list {
            let mut found = false;
            merged.iter_mut().for_each(|c| {
                if c.denom == coin.denom {
                    c.amount += coin.amount;
                    found = true;
                }
            });
            if !found {
                merged.push(coin.clone());
            }
        }
    }
    merged
}

#[test]
/// Runs all tests again the Osmosis bindings
pub fn test_with_osmosis_bindings() {
    let docker: Cli = Cli::default();
    let app = App::new(TEST_CONFIG_PATH, &docker);

    let accs = app
        .init_accounts(
            &[
                Coin::new(1_000_000_000_000, "uatom"),
                Coin::new(1_000_000_000_000, "uosmo"),
            ],
            2,
        )
        .unwrap();

    test_balancing_provide_liquidity(&app, accs);
}

/// Instantiates the liquidity helper contract
pub fn setup_osmosis_liquidity_provider_tests<R>(
    app: &R,
    accs: &[SigningAccount],
) -> LiquidityHelper
where
    R: for<'a> Runner<'a>,
{
    let wasm = Wasm::new(app);
    let admin = &accs[0];

    // Load compiled wasm bytecode
    let astroport_factory_wasm_byte_code = std::fs::read(ASTROPORT_FACTORY_WASM_FILE).unwrap();
    let astroport_factory_code_id = wasm
        .store_code(&astroport_factory_wasm_byte_code, None, admin)
        .unwrap()
        .data
        .code_id;

    let astroport_generator_wasm_byte_code = std::fs::read(ASTROPORT_GENERATOR_WASM_FILE).unwrap();
    let astroport_generator_code_id = wasm
        .store_code(&astroport_generator_wasm_byte_code, None, admin)
        .unwrap()
        .data
        .code_id;

    let astroport_generator = wasm
        .instantiate(
            astroport_generator_code_id,
            &GeneratorInstantiateMsg {
                owner: admin,
                whitelist_code_id: todo!(),
                factory: todo!(),
                generator_controller: todo!(),
                voting_escrow_delegation: todo!(),
                voting_escrow: todo!(),
                guardian: todo!(),
                astro_token: todo!(),
                tokens_per_block: todo!(),
                start_block: todo!(),
                vesting_contract: todo!(),
            },
            Some(&admin.address()),    // contract admin used for migration
            Some("Astroport Factory"), // contract label
            &[],                       // funds
            admin,                     // signer
        )
        .unwrap()
        .data
        .address;

    // Instantiate the contract
    let astroport_factory = wasm
        .instantiate(
            astroport_factory_code_id,
            &AstroportFactoryInstantiateMsg {
                pair_configs: vec![],
                token_code_id: todo!(),
                fee_address: None,
                generator_address: todo!(),
                owner: admin,
                whitelist_code_id: todo!(),
            },
            Some(&admin.address()),    // contract admin used for migration
            Some("Astroport Factory"), // contract label
            &[],                       // funds
            admin,                     // signer
        )
        .unwrap()
        .data
        .address;

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
            astroport_factory_code_id,
            &InstantiateMsg { astroport_factory },
            Some(&admin.address()), // contract admin used for migration
            Some("Astroport Liquidity Helper"), // contract label
            &[],                    // funds
            admin,                  // signer
        )
        .unwrap()
        .data
        .address;

    // let liquidity_helper = LiquidityHelperBase(contract_addr).check(api).unwrap(); // TODO this errors with "human address too long". Why?
    let liquidity_helper = LiquidityHelper::new(Addr::unchecked(astroport_liquidity_helper));

    liquidity_helper
}

/// Tests the BalancingProvideLiquidity message
pub fn test_balancing_provide_liquidity<R>(app: &R, accs: Vec<SigningAccount>)
where
    R: for<'a> Runner<'a>,
{
    let liquidity_helper = setup_osmosis_liquidity_provider_tests(app, &accs);
    let gamm = Gamm::new(app);

    // Create 1:1 pool
    let pool_liquidity = vec![Coin::new(1_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_id = gamm
        .create_basic_pool(&pool_liquidity, &accs[0])
        .unwrap()
        .data
        .pool_id;
    let pool = OsmosisPool::new(pool_id);

    // Balancing Provide liquidity
    println!("Balancing provide liquidity");
    let coins = vec![Coin::new(100_000, "uatom"), Coin::new(100_000, "uosmo")];
    let msg = liquidity_helper
        .balancing_provide_liquidity(
            coins.clone().into(),
            Uint128::one(),
            to_binary(&pool).unwrap(),
        )
        .unwrap();
    let _res = execute_cosmos_msg::<_, MsgExecuteContractResponse>(app, &msg, &accs[1]).unwrap();

    // Check pool liquidity after adding
    let initial_pool_liquidity = vec![Coin::new(1_000_000, "uatom"), Coin::new(1_000_000, "uosmo")];
    let pool_liquidity = gamm.query_pool_reserves(pool_id).unwrap();
    assert_eq!(
        pool_liquidity,
        merge_coins(&[&initial_pool_liquidity, &coins])
    );
}
