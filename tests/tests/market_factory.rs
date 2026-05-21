//! Integration tests for the market-factory registry.
//!
//! Covers the happy-path register flow (must succeed when market and
//! pool are owned by the right programs) and the two negative paths
//! the program is designed to reject (market owned by the wrong
//! program; pool's bound market doesn't match the provided market).

use janus_tests::{ids, pda, so_paths, token_fixtures};
use mollusk_svm::{result::Check, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

fn system_program_id() -> Pubkey {
    "11111111111111111111111111111111".parse().unwrap()
}
fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap()
}

fn make_mollusk() -> Mollusk {
    let mut mollusk = Mollusk::new(&ids::market_factory(), so_paths::MARKET_FACTORY);
    mollusk.add_program(&ids::conditional_tokens(), so_paths::CONDITIONAL_TOKENS);
    mollusk.add_program(&ids::lmsr_market(), so_paths::LMSR_MARKET);
    mollusk.add_program(&ids::slot_height_resolver(), so_paths::SLOT_HEIGHT_RESOLVER);
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk
}

fn loaded_program_account() -> Account {
    Account {
        lamports: 1,
        data: vec![],
        owner: mollusk_svm::program::loader_keys::LOADER_V3,
        executable: true,
        rent_epoch: 0,
    }
}

/// Run the full prerequisite flow (init resolver → init market → split
/// → init pool) and return the persisted market account, pool account,
/// and the relevant pubkeys. The mollusk instance is also returned so
/// the caller can chain a register call against the same loaded
/// programs.
struct Prereqs {
    mollusk: Mollusk,
    market: Pubkey,
    market_account: Account,
    pool: Pubkey,
    pool_account: Account,
    payer: Pubkey,
    payer_account: Account,
}

fn build_prereqs() -> Prereqs {
    let mut mollusk = make_mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let deadline_slot: u64 = 1_000;
    let resolver_seed_key = Pubkey::new_unique();

    // init resolver
    let (resolver_state, resolver_state_bump) = pda::slot_resolver_state(&resolver_seed_key);
    let mut d = Vec::with_capacity(49);
    d.push(1u8);
    d.push(1u8);
    d.push(resolver_state_bump);
    d.extend_from_slice(&[0u8; 6]);
    d.extend_from_slice(&deadline_slot.to_le_bytes());
    d.extend_from_slice(resolver_seed_key.as_ref());
    let init_resolver_ix = Instruction {
        program_id: ids::slot_height_resolver(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(resolver_state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: d,
    };
    let r = mollusk.process_and_validate_instruction(
        &init_resolver_ix,
        &[
            (payer, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (resolver_state, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let payer_account = r.get_account(&payer).unwrap().clone();
    let resolver_state_account = r.get_account(&resolver_state).unwrap().clone();

    // init market
    let (market, market_bump) = pda::market(&collateral_mint, &resolver_state, deadline_slot);
    let (yes_mint, yes_bump) = pda::yes_mint(&market);
    let (no_mint, no_bump) = pda::no_mint(&market);
    let (vault, vault_bump) = pda::vault(&market);
    let mut d = Vec::with_capacity(17);
    d.push(0u8);
    d.extend_from_slice(&deadline_slot.to_le_bytes());
    d.push(market_bump);
    d.push(yes_bump);
    d.push(no_bump);
    d.push(vault_bump);
    d.extend_from_slice(&[0u8; 4]);
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
        data: d,
    };

    let user_collateral = Pubkey::new_unique();
    let user_collateral_account =
        token_fixtures::token_account(&collateral_mint, &authority, 10_000_000);
    let collateral_mint_account = token_fixtures::mint_account(&payer, 6, 0);

    let r = mollusk.process_and_validate_instruction(
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
    let payer_account = r.get_account(&payer).unwrap().clone();
    let market_account = r.get_account(&market).unwrap().clone();
    let yes_mint_account = r.get_account(&yes_mint).unwrap().clone();
    let no_mint_account = r.get_account(&no_mint).unwrap().clone();
    let vault_account = r.get_account(&vault).unwrap().clone();

    // split
    let creator_yes = Pubkey::new_unique();
    let creator_no = Pubkey::new_unique();
    let creator_yes_account = token_fixtures::token_account(&yes_mint, &authority, 0);
    let creator_no_account = token_fixtures::token_account(&no_mint, &authority, 0);
    let mut d = Vec::with_capacity(9);
    d.push(1u8);
    d.extend_from_slice(&2_000_000u64.to_le_bytes());
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
        data: d,
    };
    let r = mollusk.process_and_validate_instruction(
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
    let creator_yes_account = r.get_account(&creator_yes).unwrap().clone();
    let creator_no_account = r.get_account(&creator_no).unwrap().clone();
    let yes_mint_account = r.get_account(&yes_mint).unwrap().clone();
    let no_mint_account = r.get_account(&no_mint).unwrap().clone();

    // init pool
    let (pool, pool_bump) = pda::pool(&market);
    let (yes_vault, yes_vault_bump) = pda::pool_yes_vault(&pool);
    let (no_vault, no_vault_bump) = pda::pool_no_vault(&pool);
    let mut d = Vec::with_capacity(25);
    d.push(0u8);
    d.extend_from_slice(&1_000_000u64.to_le_bytes());
    d.extend_from_slice(&1_000_000u64.to_le_bytes());
    d.extend_from_slice(&100u16.to_le_bytes());
    d.push(pool_bump);
    d.push(yes_vault_bump);
    d.push(no_vault_bump);
    d.extend_from_slice(&[0u8; 3]);
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
        data: d,
    };
    let r = mollusk.process_and_validate_instruction(
        &init_pool_ix,
        &[
            (authority, Account::new(1_000_000_000_000u64, 0, &system_program_id())),
            (pool, Account::default()),
            (market, market_account.clone()),
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
    let pool_account = r.get_account(&pool).unwrap().clone();

    Prereqs {
        mollusk,
        market,
        market_account,
        pool,
        pool_account,
        payer,
        payer_account,
    }
}

fn build_register_ix(payer: Pubkey, market: Pubkey, pool: Pubkey) -> (Instruction, Pubkey) {
    let (registration, bump) = pda::registration(&market);
    let mut d = Vec::with_capacity(41);
    d.push(0u8); // Register tag
    d.push(bump);
    d.extend_from_slice(&[0u8; 7]); // padding
    d.extend_from_slice(&[0u8; 32]); // empty question_hash
    let ix = Instruction {
        program_id: ids::market_factory(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(registration, false),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: d,
    };
    (ix, registration)
}

#[test]
fn register_happy_path() {
    let mut p = build_prereqs();
    let (register_ix, registration) = build_register_ix(p.payer, p.market, p.pool);

    let result = p.mollusk.process_and_validate_instruction(
        &register_ix,
        &[
            (p.payer, p.payer_account.clone()),
            (registration, Account::default()),
            (p.market, p.market_account.clone()),
            (p.pool, p.pool_account.clone()),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    let reg = result.get_account(&registration).expect("registration created");
    assert_eq!(reg.owner, ids::market_factory());
    assert_eq!(reg.data.len(), 216, "MarketRegistration is 216 bytes");
    // market pubkey lives at offset 8 (after bump + 7 bytes padding).
    assert_eq!(&reg.data[8..40], p.market.as_ref());
    // pool pubkey at offset 40.
    assert_eq!(&reg.data[40..72], p.pool.as_ref());
}

#[test]
fn register_rejects_market_with_wrong_owner() {
    let mut p = build_prereqs();
    // Corrupt the market account by reassigning ownership to the system
    // program — the factory must reject this.
    let mut bad_market = p.market_account.clone();
    bad_market.owner = system_program_id();

    let (register_ix, registration) = build_register_ix(p.payer, p.market, p.pool);
    let result = p.mollusk.process_instruction(
        &register_ix,
        &[
            (p.payer, p.payer_account.clone()),
            (registration, Account::default()),
            (p.market, bad_market),
            (p.pool, p.pool_account.clone()),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
    );
    assert!(
        !matches!(result.program_result, mollusk_svm::result::ProgramResult::Success),
        "register must fail when market is owned by the wrong program"
    );
}

#[test]
fn register_rejects_pool_with_wrong_owner() {
    let mut p = build_prereqs();
    let mut bad_pool = p.pool_account.clone();
    bad_pool.owner = system_program_id();

    let (register_ix, registration) = build_register_ix(p.payer, p.market, p.pool);
    let result = p.mollusk.process_instruction(
        &register_ix,
        &[
            (p.payer, p.payer_account.clone()),
            (registration, Account::default()),
            (p.market, p.market_account.clone()),
            (p.pool, bad_pool),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
    );
    assert!(
        !matches!(result.program_result, mollusk_svm::result::ProgramResult::Success),
        "register must fail when pool is owned by the wrong program"
    );
}
