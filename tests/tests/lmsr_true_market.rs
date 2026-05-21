//! End-to-end integration test for the true-LMSR market program.
//!
//! Exercises the full lifecycle on Mollusk:
//!   1. InitializePool — pool PDA + collateral vault PDA created; subsidy
//!      `>= b·ln(2)` pulled from the creator.
//!   2. Buy(YES, delta) — collateral charged, YES tokens minted to user,
//!      `q_yes` cached on the pool.
//!   3. Sell(YES, delta) — YES burned, collateral paid out, `q_yes`
//!      decremented.
//!
//! Also prints the compute-unit cost of each instruction; the Buy CU is
//! the headline number for the CU-budget audit (target < 50K).

use janus_tests::{ids, pda, so_paths, token_fixtures};
use mollusk_svm::{result::Check, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_program_pack::Pack;
use spl_token_interface::state::Account as SplTokenAccount;

fn system_program_id() -> Pubkey {
    "11111111111111111111111111111111".parse().unwrap()
}
fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap()
}

fn make_mollusk() -> Mollusk {
    let mut mollusk = Mollusk::new(&ids::lmsr_true_market(), so_paths::LMSR_TRUE_MARKET);
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk
}

fn token_balance(acct: &Account) -> u64 {
    SplTokenAccount::unpack(&acct.data).expect("valid SPL token account").amount
}

#[test]
fn init_buy_sell_lifecycle() {
    let mollusk = make_mollusk();

    let payer = Pubkey::new_unique();
    let authority = payer; // same signer keeps the test simpler

    // Resolver program/state are stored on the pool but not invoked during
    // init/buy/sell — so any pubkey will do for these slots.
    let resolver_program = Pubkey::new_unique();
    let resolver_state = Pubkey::new_unique();

    let collateral_mint = Pubkey::new_unique();
    let collateral_mint_account = token_fixtures::mint_account(&payer, 6, 0);

    // ---- Derive pool + collateral vault PDAs.
    let (pool, pool_bump) = pda::true_pool(&resolver_state);
    let (collateral_vault, vault_bump) = pda::true_collateral_vault(&pool);

    // Pool is the mint authority for YES + NO.
    let yes_mint = Pubkey::new_unique();
    let no_mint = Pubkey::new_unique();
    let yes_mint_account = token_fixtures::mint_account(&pool, 6, 0);
    let no_mint_account = token_fixtures::mint_account(&pool, 6, 0);

    // User's collateral account, funded with enough to pay all buy costs.
    let user_collateral = Pubkey::new_unique();
    let initial_balance = 1_000_000u64;
    let user_collateral_account =
        token_fixtures::token_account(&collateral_mint, &authority, initial_balance);

    // ---- Step 1: InitializePool ----
    // b = 1000 → ceil(b · ln(2)) = ceil(693.147) = 694.
    let b: u64 = 1_000;
    let initial_subsidy: u64 = 1_000; // > 694
    let mut init_data = Vec::with_capacity(25);
    init_data.push(0u8); // InitializePool tag
    init_data.extend_from_slice(&b.to_le_bytes());
    init_data.extend_from_slice(&initial_subsidy.to_le_bytes());
    init_data.push(pool_bump);
    init_data.push(vault_bump);
    init_data.extend_from_slice(&[0u8; 6]);

    let init_ix = Instruction {
        program_id: ids::lmsr_true_market(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(resolver_program, false),
            AccountMeta::new_readonly(resolver_state, false),
            AccountMeta::new_readonly(collateral_mint, false),
            AccountMeta::new(collateral_vault, false),
            AccountMeta::new_readonly(yes_mint, false),
            AccountMeta::new_readonly(no_mint, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: init_data,
    };

    let result = mollusk.process_and_validate_instruction(
        &init_ix,
        &[
            (payer, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (pool, Account::default()),
            (resolver_program, Account::new(0, 0, &system_program_id())),
            (resolver_state, Account::new(0, 0, &system_program_id())),
            (collateral_mint, collateral_mint_account.clone()),
            (collateral_vault, Account::default()),
            (yes_mint, yes_mint_account.clone()),
            (no_mint, no_mint_account.clone()),
            (user_collateral, user_collateral_account),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    println!("InitializePool CU: {}", result.compute_units_consumed);

    let pool_account = result.get_account(&pool).unwrap().clone();
    let collateral_vault_account = result.get_account(&collateral_vault).unwrap().clone();
    let user_collateral_account_post = result.get_account(&user_collateral).unwrap().clone();

    assert_eq!(pool_account.owner, ids::lmsr_true_market());
    assert_eq!(pool_account.data.len(), 264);
    assert_eq!(token_balance(&collateral_vault_account), initial_subsidy);
    assert_eq!(
        token_balance(&user_collateral_account_post),
        initial_balance - initial_subsidy,
    );

    // ---- Step 2: Buy(YES, delta=10) ----
    // At q_yes = q_no = 0, the LMSR price is exactly 0.5. So buying 10
    // YES at b=1000 should cost ≈ 10 × 0.5 = 5 collateral (the curve
    // adjusts slightly upward as q_yes grows, but for tiny delta vs b
    // the linear approximation is excellent).
    let user_yes = Pubkey::new_unique();
    let user_yes_account = token_fixtures::token_account(&yes_mint, &authority, 0);

    let delta: u64 = 10;
    let max_collateral_in: u64 = 50; // generous slippage budget
    let mut buy_data = Vec::with_capacity(25);
    buy_data.push(1u8); // Buy tag
    buy_data.extend_from_slice(&delta.to_le_bytes());
    buy_data.extend_from_slice(&max_collateral_in.to_le_bytes());
    buy_data.push(0u8); // side = YES
    buy_data.extend_from_slice(&[0u8; 7]);

    let buy_ix = Instruction {
        program_id: ids::lmsr_true_market(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(pool, false),
            AccountMeta::new(collateral_vault, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(user_yes, false),
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: buy_data,
    };

    let result = mollusk.process_and_validate_instruction(
        &buy_ix,
        &[
            (authority, Account::new(1_000_000u64, 0, &system_program_id())),
            (pool, pool_account.clone()),
            (collateral_vault, collateral_vault_account.clone()),
            (user_collateral, user_collateral_account_post.clone()),
            (yes_mint, yes_mint_account.clone()),
            (user_yes, user_yes_account),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );
    println!("Buy(YES, 10) CU: {}", result.compute_units_consumed);
    assert!(
        result.compute_units_consumed < 200_000,
        "Buy exceeded 200K CU budget: {}",
        result.compute_units_consumed
    );

    let user_yes_post = result.get_account(&user_yes).unwrap().clone();
    let collateral_vault_after_buy = result.get_account(&collateral_vault).unwrap().clone();
    let user_collateral_after_buy = result.get_account(&user_collateral).unwrap().clone();
    let pool_after_buy = result.get_account(&pool).unwrap().clone();

    let yes_balance = token_balance(&user_yes_post);
    assert_eq!(yes_balance, delta, "user received exactly delta YES");

    let buy_cost =
        token_balance(&user_collateral_account_post) - token_balance(&user_collateral_after_buy);
    println!("Buy(YES, 10) cost (collateral): {}", buy_cost);
    // Cost must be in [3, 10]: lower bound ~ 0.3 (in case math compresses
    // the price), upper bound = max_collateral_in.
    assert!(
        (3..=max_collateral_in).contains(&buy_cost),
        "buy cost {} outside reasonable bounds",
        buy_cost
    );
    assert_eq!(
        token_balance(&collateral_vault_after_buy),
        initial_subsidy + buy_cost,
        "vault credited",
    );

    // Verify pool's q_yes was incremented.
    let q_yes_after = u64::from_le_bytes(pool_after_buy.data[240..248].try_into().unwrap());
    assert_eq!(q_yes_after, delta, "pool.q_yes incremented");

    // ---- Step 3: Sell(YES, delta=10) — round-trip should refund roughly the buy cost ----
    let mut sell_data = Vec::with_capacity(25);
    sell_data.push(2u8); // Sell tag
    sell_data.extend_from_slice(&delta.to_le_bytes());
    sell_data.extend_from_slice(&0u64.to_le_bytes()); // min_collateral_out = 0 (no slippage check)
    sell_data.push(0u8); // side = YES
    sell_data.extend_from_slice(&[0u8; 7]);

    let sell_ix = Instruction {
        program_id: ids::lmsr_true_market(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(pool, false),
            AccountMeta::new(collateral_vault, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(user_yes, false),
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: sell_data,
    };

    let result = mollusk.process_and_validate_instruction(
        &sell_ix,
        &[
            (authority, Account::new(1_000_000u64, 0, &system_program_id())),
            (pool, pool_after_buy),
            (collateral_vault, collateral_vault_after_buy),
            (user_collateral, user_collateral_after_buy.clone()),
            (yes_mint, yes_mint_account.clone()),
            (user_yes, user_yes_post),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );
    println!("Sell(YES, 10) CU: {}", result.compute_units_consumed);

    let user_yes_after_sell = result.get_account(&user_yes).unwrap();
    let user_collateral_after_sell = result.get_account(&user_collateral).unwrap();
    let pool_after_sell = result.get_account(&pool).unwrap();

    assert_eq!(token_balance(user_yes_after_sell), 0, "YES burned");

    // Sell payout should be very close to the buy cost (round-trip).
    // It may be slightly lower because of round-up on buy and round-down
    // (default rounding) on sell — both favor the pool.
    let payout = token_balance(user_collateral_after_sell)
        - token_balance(&user_collateral_after_buy);
    println!("Sell(YES, 10) payout: {}", payout);
    assert!(payout <= buy_cost, "round-trip favors pool (payout ≤ buy cost)");
    assert!(buy_cost - payout <= 2, "round-trip loss ≤ 2 collateral units");

    // q_yes returns to 0.
    let q_yes_final = u64::from_le_bytes(pool_after_sell.data[240..248].try_into().unwrap());
    assert_eq!(q_yes_final, 0, "pool.q_yes returned to 0");
}

#[test]
fn init_rejects_subsidy_below_b_ln2() {
    let mollusk = make_mollusk();

    let payer = Pubkey::new_unique();
    let resolver_program = Pubkey::new_unique();
    let resolver_state = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let collateral_mint_account = token_fixtures::mint_account(&payer, 6, 0);

    let (pool, pool_bump) = pda::true_pool(&resolver_state);
    let (collateral_vault, vault_bump) = pda::true_collateral_vault(&pool);
    let yes_mint = Pubkey::new_unique();
    let no_mint = Pubkey::new_unique();
    let yes_mint_account = token_fixtures::mint_account(&pool, 6, 0);
    let no_mint_account = token_fixtures::mint_account(&pool, 6, 0);
    let user_collateral = Pubkey::new_unique();
    let user_collateral_account = token_fixtures::token_account(&collateral_mint, &payer, 100_000);

    let b: u64 = 1000;
    let bad_subsidy: u64 = 500; // below ceil(b·ln(2)) = 694

    let mut data = Vec::with_capacity(25);
    data.push(0u8);
    data.extend_from_slice(&b.to_le_bytes());
    data.extend_from_slice(&bad_subsidy.to_le_bytes());
    data.push(pool_bump);
    data.push(vault_bump);
    data.extend_from_slice(&[0u8; 6]);

    let ix = Instruction {
        program_id: ids::lmsr_true_market(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(resolver_program, false),
            AccountMeta::new_readonly(resolver_state, false),
            AccountMeta::new_readonly(collateral_mint, false),
            AccountMeta::new(collateral_vault, false),
            AccountMeta::new_readonly(yes_mint, false),
            AccountMeta::new_readonly(no_mint, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data,
    };

    let result = mollusk.process_instruction(
        &ix,
        &[
            (payer, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (pool, Account::default()),
            (resolver_program, Account::new(0, 0, &system_program_id())),
            (resolver_state, Account::new(0, 0, &system_program_id())),
            (collateral_mint, collateral_mint_account),
            (collateral_vault, Account::default()),
            (yes_mint, yes_mint_account),
            (no_mint, no_mint_account),
            (user_collateral, user_collateral_account),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
    );
    assert!(
        !matches!(
            result.program_result,
            mollusk_svm::result::ProgramResult::Success
        ),
        "init must fail with subsidy below b·ln(2)"
    );
}
