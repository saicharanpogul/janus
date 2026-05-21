//! End-to-end integration tests for the conditional-tokens program.
//!
//! Each test wires up Mollusk with the conditional-tokens program, the
//! slot-height-resolver program (so a market can bind to a real resolver
//! state), and the SPL Token program. It then runs the actual instruction
//! sequence — Initialize resolver → InitializeMarket → Split → Merge —
//! and asserts on the resulting account balances.

use janus_tests::{ids, pda, so_paths, token_fixtures};
use mollusk_svm::{result::Check, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use spl_token_interface::state::Account as SplTokenAccount;

fn system_program_id() -> Pubkey {
    "11111111111111111111111111111111".parse().unwrap()
}

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap()
}

/// Spin up a Mollusk preloaded with the three programs we need to drive
/// a conditional-tokens end-to-end flow.
fn make_mollusk() -> Mollusk {
    let mut mollusk = Mollusk::new(&ids::conditional_tokens(), so_paths::CONDITIONAL_TOKENS);
    mollusk.add_program(&ids::slot_height_resolver(), so_paths::SLOT_HEIGHT_RESOLVER);
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk
}

/// Parse the `amount` field out of an SPL `TokenAccount`. The SPL layout
/// stores `amount` at offset 64 (after `mint: Pubkey, owner: Pubkey`).
fn token_balance(acct: &Account) -> u64 {
    use solana_program_pack::Pack;
    SplTokenAccount::unpack(&acct.data).expect("valid SPL token account").amount
}

#[test]
fn full_lifecycle_initialize_split_merge() {
    let mut mollusk = make_mollusk();
    mollusk.sysvars.clock.slot = 100;

    // ---- Test parameters ----
    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let deadline_slot: u64 = 1_000;
    let resolver_seed_key = Pubkey::new_unique();

    // ---- Step 1: initialize the slot-height resolver state ----
    let (resolver_state, resolver_state_bump) = pda::slot_resolver_state(&resolver_seed_key);
    let init_resolver_data = {
        let mut d = Vec::with_capacity(49);
        d.push(1u8); // INSTRUCTION_INITIALIZE
        d.push(1u8); // outcome = Yes
        d.push(resolver_state_bump);
        d.extend_from_slice(&[0u8; 6]);
        d.extend_from_slice(&deadline_slot.to_le_bytes());
        d.extend_from_slice(resolver_seed_key.as_ref());
        d
    };
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

    let payer_account = result.get_account(&payer).expect("payer").clone();
    let resolver_state_account = result.get_account(&resolver_state).expect("state").clone();

    // ---- Step 2: prepare collateral mint + user collateral account ----
    let collateral_mint_account = token_fixtures::mint_account(&payer, 6, 0);
    let user_collateral = Pubkey::new_unique();
    let initial_user_balance: u64 = 5_000_000; // 5 USDC at 6 decimals
    let user_collateral_account =
        token_fixtures::token_account(&collateral_mint, &user, initial_user_balance);

    // ---- Step 3: derive market + outcome mint + vault PDAs ----
    let (market, market_bump) = pda::market(&collateral_mint, &resolver_state, deadline_slot);
    let (yes_mint, yes_bump) = pda::yes_mint(&market);
    let (no_mint, no_bump) = pda::no_mint(&market);
    let (vault, vault_bump) = pda::vault(&market);

    let init_market_data = {
        let mut d = Vec::with_capacity(17);
        d.push(0u8); // InitializeMarket
        d.extend_from_slice(&deadline_slot.to_le_bytes());
        d.push(market_bump);
        d.push(yes_bump);
        d.push(no_bump);
        d.push(vault_bump);
        d.extend_from_slice(&[0u8; 4]);
        d
    };
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

    // The resolver_program slot is just the program's executable account;
    // mollusk has it loaded so we can pass a placeholder.
    let resolver_program_account = Account {
        lamports: 1,
        data: vec![],
        owner: mollusk_svm::program::loader_keys::LOADER_V3,
        executable: true,
        rent_epoch: 0,
    };

    let result = mollusk.process_and_validate_instruction(
        &init_market_ix,
        &[
            (payer, payer_account.clone()),
            (market, Account::default()),
            (collateral_mint, collateral_mint_account.clone()),
            (yes_mint, Account::default()),
            (no_mint, Account::default()),
            (vault, Account::default()),
            (ids::slot_height_resolver(), resolver_program_account),
            (resolver_state, resolver_state_account.clone()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    // Capture post-init state so the next instruction can build on it.
    let payer_account = result.get_account(&payer).unwrap().clone();
    let market_account = result.get_account(&market).unwrap().clone();
    let yes_mint_account = result.get_account(&yes_mint).unwrap().clone();
    let no_mint_account = result.get_account(&no_mint).unwrap().clone();
    let vault_account = result.get_account(&vault).unwrap().clone();
    let collateral_mint_account = result
        .get_account(&collateral_mint)
        .unwrap_or(&collateral_mint_account)
        .clone();

    // Sanity-check the market was created with our program as owner and
    // the expected fixed size.
    assert_eq!(market_account.owner, ids::conditional_tokens());
    assert_eq!(market_account.data.len(), 248);
    assert_eq!(market_account.data[1], 0, "status should be Active=0");

    // ---- Step 4: pre-create user_yes and user_no token accounts ----
    // The split instruction expects these to already exist as valid SPL
    // token accounts. In production the SDK creates them as ATAs
    // alongside the split; here we just inject pre-initialised accounts.
    let user_yes = Pubkey::new_unique();
    let user_no = Pubkey::new_unique();
    let user_yes_account = token_fixtures::token_account(&yes_mint, &user, 0);
    let user_no_account = token_fixtures::token_account(&no_mint, &user, 0);

    // ---- Step 5: Split — deposit 1_000_000 collateral, mint matching YES + NO ----
    let split_amount: u64 = 1_000_000;
    let split_data = {
        let mut d = Vec::with_capacity(9);
        d.push(1u8); // Split tag
        d.extend_from_slice(&split_amount.to_le_bytes());
        d
    };
    let split_ix = Instruction {
        program_id: ids::conditional_tokens(),
        accounts: vec![
            AccountMeta::new_readonly(user, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(user_yes, false),
            AccountMeta::new(user_no, false),
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: split_data,
    };

    let result = mollusk.process_and_validate_instruction(
        &split_ix,
        &[
            (user, Account::new(1_000_000_000u64, 0, &system_program_id())),
            (market, market_account.clone()),
            (user_collateral, user_collateral_account.clone()),
            (vault, vault_account.clone()),
            (yes_mint, yes_mint_account.clone()),
            (no_mint, no_mint_account.clone()),
            (user_yes, user_yes_account),
            (user_no, user_no_account),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );

    // Verify post-split balances.
    let post_user_collateral = result.get_account(&user_collateral).unwrap();
    let post_vault = result.get_account(&vault).unwrap();
    let post_user_yes = result.get_account(&user_yes).unwrap();
    let post_user_no = result.get_account(&user_no).unwrap();

    assert_eq!(
        token_balance(post_user_collateral),
        initial_user_balance - split_amount,
        "user collateral should be debited by split_amount"
    );
    assert_eq!(
        token_balance(post_vault),
        split_amount,
        "vault should hold split_amount of collateral"
    );
    assert_eq!(token_balance(post_user_yes), split_amount, "user gets YES = amount");
    assert_eq!(token_balance(post_user_no), split_amount, "user gets NO = amount");

    let post_yes_mint = result.get_account(&yes_mint).unwrap().clone();
    let post_no_mint = result.get_account(&no_mint).unwrap().clone();

    // ---- Step 6: Merge — burn matching YES + NO, recover collateral ----
    let merge_amount: u64 = 400_000;
    let merge_data = {
        let mut d = Vec::with_capacity(9);
        d.push(2u8); // Merge tag
        d.extend_from_slice(&merge_amount.to_le_bytes());
        d
    };
    let merge_ix = Instruction {
        program_id: ids::conditional_tokens(),
        accounts: vec![
            AccountMeta::new_readonly(user, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(user_yes, false),
            AccountMeta::new(user_no, false),
            AccountMeta::new_readonly(token_program_id(), false),
        ],
        data: merge_data,
    };

    let result = mollusk.process_and_validate_instruction(
        &merge_ix,
        &[
            (user, Account::new(1_000_000_000u64, 0, &system_program_id())),
            (market, market_account),
            (user_collateral, post_user_collateral.clone()),
            (vault, post_vault.clone()),
            (yes_mint, post_yes_mint),
            (no_mint, post_no_mint),
            (user_yes, post_user_yes.clone()),
            (user_no, post_user_no.clone()),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );

    let final_user_collateral = result.get_account(&user_collateral).unwrap();
    let final_vault = result.get_account(&vault).unwrap();
    let final_user_yes = result.get_account(&user_yes).unwrap();
    let final_user_no = result.get_account(&user_no).unwrap();

    // After splitting 1_000_000 then merging 400_000:
    //   user_collateral = initial - 1_000_000 + 400_000
    //   vault           = 1_000_000 - 400_000 = 600_000
    //   user_yes / no   = 1_000_000 - 400_000 = 600_000
    assert_eq!(
        token_balance(final_user_collateral),
        initial_user_balance - split_amount + merge_amount,
    );
    assert_eq!(token_balance(final_vault), split_amount - merge_amount);
    assert_eq!(token_balance(final_user_yes), split_amount - merge_amount);
    assert_eq!(token_balance(final_user_no), split_amount - merge_amount);

    let _ = payer_account; // suppress unused warning (kept for tx chaining)
}
