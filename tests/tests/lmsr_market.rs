//! End-to-end integration test for the lmsr-market program.
//!
//! Builds a full market (initialise resolver, initialise conditional-tokens
//! market, split collateral), then initialises an LMSR pool and runs a
//! Swap to verify the CPMM curve and slippage protection work against
//! the real token program.

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
    let mut mollusk = Mollusk::new(&ids::lmsr_market(), so_paths::LMSR_MARKET);
    mollusk.add_program(&ids::conditional_tokens(), so_paths::CONDITIONAL_TOKENS);
    mollusk.add_program(&ids::slot_height_resolver(), so_paths::SLOT_HEIGHT_RESOLVER);
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk
}

fn token_balance(acct: &Account) -> u64 {
    SplTokenAccount::unpack(&acct.data).expect("valid SPL token account").amount
}

/// Stub program account for a loaded SBF program.
fn loaded_program_account() -> Account {
    Account {
        lamports: 1,
        data: vec![],
        owner: mollusk_svm::program::loader_keys::LOADER_V3,
        executable: true,
        rent_epoch: 0,
    }
}

#[test]
fn initialize_pool_and_swap() {
    let mut mollusk = make_mollusk();
    mollusk.sysvars.clock.slot = 100;

    // ---- Common test fixtures ----
    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let deadline_slot: u64 = 1_000;
    let resolver_seed_key = Pubkey::new_unique();

    // ---- Step 1: init resolver ----
    let (resolver_state, resolver_state_bump) = pda::slot_resolver_state(&resolver_seed_key);
    let mut init_resolver_data = Vec::with_capacity(49);
    init_resolver_data.push(1u8);
    init_resolver_data.push(1u8);
    init_resolver_data.push(resolver_state_bump);
    init_resolver_data.extend_from_slice(&[0u8; 6]);
    init_resolver_data.extend_from_slice(&deadline_slot.to_le_bytes());
    init_resolver_data.extend_from_slice(resolver_seed_key.as_ref());
    let init_resolver_ix = Instruction {
        program_id: ids::slot_height_resolver(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(resolver_state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: init_resolver_data,
    };
    let result = mollusk.process_and_validate_instruction(
        &init_resolver_ix,
        &[
            (payer, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (resolver_state, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let payer_account = result.get_account(&payer).unwrap().clone();
    let resolver_state_account = result.get_account(&resolver_state).unwrap().clone();

    // ---- Step 2: init market ----
    let (market, market_bump) = pda::market(&collateral_mint, &resolver_state, deadline_slot);
    let (yes_mint, yes_bump) = pda::yes_mint(&market);
    let (no_mint, no_bump) = pda::no_mint(&market);
    let (vault, vault_bump) = pda::vault(&market);

    let mut init_market_data = Vec::with_capacity(17);
    init_market_data.push(0u8);
    init_market_data.extend_from_slice(&deadline_slot.to_le_bytes());
    init_market_data.push(market_bump);
    init_market_data.push(yes_bump);
    init_market_data.push(no_bump);
    init_market_data.push(vault_bump);
    init_market_data.extend_from_slice(&[0u8; 4]);
    let init_market_ix = Instruction {
        program_id: ids::conditional_tokens(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(collateral_mint, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(ids::slot_height_resolver(), false),
            AccountMeta::new_readonly(resolver_state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: init_market_data,
    };

    let user_collateral = Pubkey::new_unique();
    let initial_collateral = 10_000_000u64;
    let user_collateral_account =
        token_fixtures::token_account(&collateral_mint, &authority, initial_collateral);
    let collateral_mint_account = token_fixtures::mint_account(&payer, 6, 0);

    let result = mollusk.process_and_validate_instruction(
        &init_market_ix,
        &[
            (payer, payer_account.clone()),
            (market, Account::default()),
            (collateral_mint, collateral_mint_account.clone()),
            (yes_mint, Account::default()),
            (no_mint, Account::default()),
            (vault, Account::default()),
            (ids::slot_height_resolver(), loaded_program_account()),
            (resolver_state, resolver_state_account.clone()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let payer_account = result.get_account(&payer).unwrap().clone();
    let market_account = result.get_account(&market).unwrap().clone();
    let yes_mint_account = result.get_account(&yes_mint).unwrap().clone();
    let no_mint_account = result.get_account(&no_mint).unwrap().clone();
    let vault_account = result.get_account(&vault).unwrap().clone();

    // ---- Step 3: split 2_000_000 collateral so the creator has YES + NO to seed the pool ----
    let creator_yes = Pubkey::new_unique();
    let creator_no = Pubkey::new_unique();
    let creator_yes_account = token_fixtures::token_account(&yes_mint, &authority, 0);
    let creator_no_account = token_fixtures::token_account(&no_mint, &authority, 0);

    let split_amount: u64 = 2_000_000;
    let mut split_data = Vec::with_capacity(9);
    split_data.push(1u8);
    split_data.extend_from_slice(&split_amount.to_le_bytes());
    let split_ix = Instruction {
        program_id: ids::conditional_tokens(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(creator_yes, false),
            AccountMeta::new(creator_no, false),
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: split_data,
    };
    let result = mollusk.process_and_validate_instruction(
        &split_ix,
        &[
            (authority, Account::new(1_000_000_000u64, 0, &system_program_id())),
            (market, market_account.clone()),
            (user_collateral, user_collateral_account),
            (vault, vault_account),
            (yes_mint, yes_mint_account),
            (no_mint, no_mint_account),
            (creator_yes, creator_yes_account),
            (creator_no, creator_no_account),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );
    let creator_yes_account = result.get_account(&creator_yes).unwrap().clone();
    let creator_no_account = result.get_account(&creator_no).unwrap().clone();
    let yes_mint_account = result.get_account(&yes_mint).unwrap().clone();
    let no_mint_account = result.get_account(&no_mint).unwrap().clone();

    assert_eq!(token_balance(&creator_yes_account), split_amount);
    assert_eq!(token_balance(&creator_no_account), split_amount);

    // ---- Step 4: InitializePool — seed reserves with 1_000_000 each, 100 bps fee ----
    let (pool, pool_bump) = pda::pool(&market);
    let (yes_vault, yes_vault_bump) = pda::pool_yes_vault(&pool);
    let (no_vault, no_vault_bump) = pda::pool_no_vault(&pool);

    let subsidy_yes: u64 = 1_000_000;
    let subsidy_no: u64 = 1_000_000;
    let fee_bps: u16 = 100;

    let mut init_pool_data = Vec::with_capacity(25);
    init_pool_data.push(0u8); // InitializePool tag
    init_pool_data.extend_from_slice(&subsidy_yes.to_le_bytes());
    init_pool_data.extend_from_slice(&subsidy_no.to_le_bytes());
    init_pool_data.extend_from_slice(&fee_bps.to_le_bytes());
    init_pool_data.push(pool_bump);
    init_pool_data.push(yes_vault_bump);
    init_pool_data.push(no_vault_bump);
    init_pool_data.extend_from_slice(&[0u8; 3]);

    let init_pool_ix = Instruction {
        program_id: ids::lmsr_market(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new_readonly(yes_mint, false),
            AccountMeta::new_readonly(no_mint, false),
            AccountMeta::new(yes_vault, false),
            AccountMeta::new(no_vault, false),
            AccountMeta::new(creator_yes, false),
            AccountMeta::new(creator_no, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: init_pool_data,
    };

    let result = mollusk.process_and_validate_instruction(
        &init_pool_ix,
        &[
            (authority, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (pool, Account::default()),
            (market, market_account.clone()),
            (yes_mint, yes_mint_account.clone()),
            (no_mint, no_mint_account.clone()),
            (yes_vault, Account::default()),
            (no_vault, Account::default()),
            (creator_yes, creator_yes_account),
            (creator_no, creator_no_account),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    let pool_account = result.get_account(&pool).unwrap().clone();
    let yes_vault_account = result.get_account(&yes_vault).unwrap().clone();
    let no_vault_account = result.get_account(&no_vault).unwrap().clone();
    let creator_yes_account = result.get_account(&creator_yes).unwrap().clone();
    let creator_no_account = result.get_account(&creator_no).unwrap().clone();

    assert_eq!(pool_account.owner, ids::lmsr_market(), "pool owned by lmsr-market");
    assert_eq!(pool_account.data.len(), 224, "Pool struct is 224 bytes");
    assert_eq!(
        token_balance(&yes_vault_account),
        subsidy_yes,
        "pool's YES vault holds the YES subsidy"
    );
    assert_eq!(token_balance(&no_vault_account), subsidy_no);
    // Creator's outcome tokens debited by the subsidy.
    assert_eq!(token_balance(&creator_yes_account), split_amount - subsidy_yes);
    assert_eq!(token_balance(&creator_no_account), split_amount - subsidy_no);

    // ---- Step 5: Swap YES → NO with the trader using their remaining YES ----
    let trader = authority; // reuse for simplicity
    let amount_in: u64 = 100_000;
    // CPMM: amount_in_after_fee = 100_000 * (10_000 - 100) / 10_000 = 99_000
    // amount_out = 1_000_000 * 99_000 / (1_000_000 + 99_000) = 90,081 (rounded down)
    let min_amount_out: u64 = 90_000;

    let mut swap_data = Vec::with_capacity(25);
    swap_data.push(1u8); // Swap tag
    swap_data.extend_from_slice(&amount_in.to_le_bytes());
    swap_data.extend_from_slice(&min_amount_out.to_le_bytes());
    swap_data.push(0u8); // direction = YES in, NO out
    swap_data.extend_from_slice(&[0u8; 7]);

    let swap_ix = Instruction {
        program_id: ids::lmsr_market(),
        accounts: vec![
            AccountMeta::new_readonly(trader, true),
            AccountMeta::new(pool, false),
            AccountMeta::new(yes_vault, false),
            AccountMeta::new(no_vault, false),
            AccountMeta::new(creator_yes, false), // trader's YES (in)
            AccountMeta::new(creator_no, false),  // trader's NO (out)
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: swap_data,
    };

    let result = mollusk.process_and_validate_instruction(
        &swap_ix,
        &[
            (trader, Account::new(1_000_000u64, 0, &system_program_id())),
            (pool, pool_account),
            (yes_vault, yes_vault_account),
            (no_vault, no_vault_account),
            (creator_yes, creator_yes_account.clone()),
            (creator_no, creator_no_account.clone()),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );

    let final_yes_vault = result.get_account(&yes_vault).unwrap();
    let final_no_vault = result.get_account(&no_vault).unwrap();
    let final_trader_yes = result.get_account(&creator_yes).unwrap();
    let final_trader_no = result.get_account(&creator_no).unwrap();

    // YES vault grew by amount_in; NO vault shrank by amount_out.
    assert_eq!(
        token_balance(final_yes_vault),
        subsidy_yes + amount_in,
        "YES vault gained amount_in"
    );
    let amount_out = subsidy_no - token_balance(final_no_vault);
    assert!(amount_out >= min_amount_out, "slippage check satisfied");
    assert!(amount_out < amount_in, "amount_out < amount_in for first trade (fees + curve)");

    // Trader's YES debited, NO credited.
    assert_eq!(
        token_balance(final_trader_yes),
        token_balance(&creator_yes_account) - amount_in
    );
    assert_eq!(
        token_balance(final_trader_no),
        token_balance(&creator_no_account) + amount_out
    );

    let _ = payer_account;
}

#[test]
fn swap_rejects_when_slippage_exceeded() {
    // Same setup as above, but request an unreachable min_amount_out
    // and assert the instruction errors instead of succeeding.
    let mut mollusk = make_mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let deadline_slot: u64 = 1_000;
    let resolver_seed_key = Pubkey::new_unique();
    let (resolver_state, resolver_state_bump) = pda::slot_resolver_state(&resolver_seed_key);

    // Init resolver
    let mut init_resolver_data = Vec::with_capacity(49);
    init_resolver_data.push(1u8);
    init_resolver_data.push(1u8);
    init_resolver_data.push(resolver_state_bump);
    init_resolver_data.extend_from_slice(&[0u8; 6]);
    init_resolver_data.extend_from_slice(&deadline_slot.to_le_bytes());
    init_resolver_data.extend_from_slice(resolver_seed_key.as_ref());
    let init_resolver_ix = Instruction {
        program_id: ids::slot_height_resolver(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(resolver_state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: init_resolver_data,
    };
    let result = mollusk.process_and_validate_instruction(
        &init_resolver_ix,
        &[
            (payer, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (resolver_state, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let payer_account = result.get_account(&payer).unwrap().clone();
    let resolver_state_account = result.get_account(&resolver_state).unwrap().clone();

    // Init market
    let (market, market_bump) = pda::market(&collateral_mint, &resolver_state, deadline_slot);
    let (yes_mint, yes_bump) = pda::yes_mint(&market);
    let (no_mint, no_bump) = pda::no_mint(&market);
    let (vault, vault_bump) = pda::vault(&market);
    let mut init_market_data = Vec::with_capacity(17);
    init_market_data.push(0u8);
    init_market_data.extend_from_slice(&deadline_slot.to_le_bytes());
    init_market_data.push(market_bump);
    init_market_data.push(yes_bump);
    init_market_data.push(no_bump);
    init_market_data.push(vault_bump);
    init_market_data.extend_from_slice(&[0u8; 4]);
    let init_market_ix = Instruction {
        program_id: ids::conditional_tokens(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(collateral_mint, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(ids::slot_height_resolver(), false),
            AccountMeta::new_readonly(resolver_state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: init_market_data,
    };

    let user_collateral = Pubkey::new_unique();
    let user_collateral_account =
        token_fixtures::token_account(&collateral_mint, &authority, 10_000_000);
    let collateral_mint_account = token_fixtures::mint_account(&payer, 6, 0);

    let result = mollusk.process_and_validate_instruction(
        &init_market_ix,
        &[
            (payer, payer_account.clone()),
            (market, Account::default()),
            (collateral_mint, collateral_mint_account),
            (yes_mint, Account::default()),
            (no_mint, Account::default()),
            (vault, Account::default()),
            (ids::slot_height_resolver(), loaded_program_account()),
            (resolver_state, resolver_state_account),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    let market_account = result.get_account(&market).unwrap().clone();
    let yes_mint_account = result.get_account(&yes_mint).unwrap().clone();
    let no_mint_account = result.get_account(&no_mint).unwrap().clone();
    let vault_account = result.get_account(&vault).unwrap().clone();

    // Split + InitializePool (compressed; reusing test setup pattern)
    let creator_yes = Pubkey::new_unique();
    let creator_no = Pubkey::new_unique();
    let creator_yes_account = token_fixtures::token_account(&yes_mint, &authority, 0);
    let creator_no_account = token_fixtures::token_account(&no_mint, &authority, 0);

    let mut split_data = Vec::with_capacity(9);
    split_data.push(1u8);
    split_data.extend_from_slice(&2_000_000u64.to_le_bytes());
    let split_ix = Instruction {
        program_id: ids::conditional_tokens(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(creator_yes, false),
            AccountMeta::new(creator_no, false),
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: split_data,
    };
    let result = mollusk.process_and_validate_instruction(
        &split_ix,
        &[
            (authority, Account::new(1_000_000_000u64, 0, &system_program_id())),
            (market, market_account.clone()),
            (user_collateral, user_collateral_account),
            (vault, vault_account),
            (yes_mint, yes_mint_account),
            (no_mint, no_mint_account),
            (creator_yes, creator_yes_account),
            (creator_no, creator_no_account),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );
    let creator_yes_account = result.get_account(&creator_yes).unwrap().clone();
    let creator_no_account = result.get_account(&creator_no).unwrap().clone();
    let yes_mint_account = result.get_account(&yes_mint).unwrap().clone();
    let no_mint_account = result.get_account(&no_mint).unwrap().clone();

    let (pool, pool_bump) = pda::pool(&market);
    let (yes_vault, yes_vault_bump) = pda::pool_yes_vault(&pool);
    let (no_vault, no_vault_bump) = pda::pool_no_vault(&pool);
    let mut init_pool_data = Vec::with_capacity(25);
    init_pool_data.push(0u8);
    init_pool_data.extend_from_slice(&1_000_000u64.to_le_bytes());
    init_pool_data.extend_from_slice(&1_000_000u64.to_le_bytes());
    init_pool_data.extend_from_slice(&100u16.to_le_bytes());
    init_pool_data.push(pool_bump);
    init_pool_data.push(yes_vault_bump);
    init_pool_data.push(no_vault_bump);
    init_pool_data.extend_from_slice(&[0u8; 3]);
    let init_pool_ix = Instruction {
        program_id: ids::lmsr_market(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new_readonly(yes_mint, false),
            AccountMeta::new_readonly(no_mint, false),
            AccountMeta::new(yes_vault, false),
            AccountMeta::new(no_vault, false),
            AccountMeta::new(creator_yes, false),
            AccountMeta::new(creator_no, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: init_pool_data,
    };
    let result = mollusk.process_and_validate_instruction(
        &init_pool_ix,
        &[
            (authority, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (pool, Account::default()),
            (market, market_account),
            (yes_mint, yes_mint_account),
            (no_mint, no_mint_account),
            (yes_vault, Account::default()),
            (no_vault, Account::default()),
            (creator_yes, creator_yes_account),
            (creator_no, creator_no_account),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let pool_account = result.get_account(&pool).unwrap().clone();
    let yes_vault_account = result.get_account(&yes_vault).unwrap().clone();
    let no_vault_account = result.get_account(&no_vault).unwrap().clone();
    let creator_yes_account = result.get_account(&creator_yes).unwrap().clone();
    let creator_no_account = result.get_account(&creator_no).unwrap().clone();

    // ---- Swap with unrealistically high min_amount_out → should error
    // with SlippageExceeded (custom error 8 from LmsrError). ----
    let mut swap_data = Vec::with_capacity(25);
    swap_data.push(1u8);
    swap_data.extend_from_slice(&100_000u64.to_le_bytes());
    swap_data.extend_from_slice(&500_000u64.to_le_bytes()); // unreachable
    swap_data.push(0u8);
    swap_data.extend_from_slice(&[0u8; 7]);

    let swap_ix = Instruction {
        program_id: ids::lmsr_market(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(pool, false),
            AccountMeta::new(yes_vault, false),
            AccountMeta::new(no_vault, false),
            AccountMeta::new(creator_yes, false),
            AccountMeta::new(creator_no, false),
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: swap_data,
    };

    let result = mollusk.process_instruction(
        &swap_ix,
        &[
            (authority, Account::new(1_000_000u64, 0, &system_program_id())),
            (pool, pool_account),
            (yes_vault, yes_vault_account),
            (no_vault, no_vault_account),
            (creator_yes, creator_yes_account),
            (creator_no, creator_no_account),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
    );
    assert!(
        !matches!(result.program_result, mollusk_svm::result::ProgramResult::Success),
        "swap with unreachable min_amount_out should error"
    );
    let _ = payer_account;
}
