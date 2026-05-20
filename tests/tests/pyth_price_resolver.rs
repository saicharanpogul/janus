//! Integration tests for the pyth-price-resolver program.
//!
//! Uses synthetic Pyth `PriceUpdateV2`-shaped accounts to drive the
//! resolver without depending on a live feed.

use janus_tests::{ids, so_paths};
use mollusk_svm::{result::Check, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

fn system_program_id() -> Pubkey {
    "11111111111111111111111111111111".parse().unwrap()
}

fn mollusk() -> Mollusk {
    Mollusk::new(&ids::pyth_price_resolver(), so_paths::PYTH_PRICE_RESOLVER)
}

fn pyth_resolver_state_pda(seed_key: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"pyth-resolver", seed_key.as_ref()],
        &ids::pyth_price_resolver(),
    )
}

/// Synthesise a Pyth `PriceUpdateV2`-shaped account.
///
/// Byte layout (must match the offsets the resolver reads at):
///   - [73..81]  price (i64 LE)
///   - [89..93]  exponent (i32 LE)
/// All other bytes are zero. Total length 133+ bytes.
fn pyth_feed_account(owner: Pubkey, price: i64, expo: i32) -> Account {
    let mut data = vec![0u8; 200];
    data[73..81].copy_from_slice(&price.to_le_bytes());
    data[89..93].copy_from_slice(&expo.to_le_bytes());
    Account {
        lamports: 1_000_000,
        data,
        owner,
        executable: false,
        rent_epoch: 0,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_initialize_ix(
    payer: Pubkey,
    authority: Pubkey,
    seed_key: Pubkey,
    price_feed: Pubkey,
    earliest_slot: u64,
    threshold_price: i64,
    threshold_expo: i32,
    comparison: u8,
) -> (Instruction, Pubkey) {
    let (state, bump) = pyth_resolver_state_pda(&seed_key);

    let mut data = Vec::with_capacity(1 + 96);
    data.push(1u8); // INSTRUCTION_INITIALIZE
    data.push(bump);
    data.push(comparison);
    data.extend_from_slice(&[0u8; 6]); // padding
    data.extend_from_slice(price_feed.as_ref());
    data.extend_from_slice(&earliest_slot.to_le_bytes());
    data.extend_from_slice(&threshold_price.to_le_bytes());
    data.extend_from_slice(&threshold_expo.to_le_bytes());
    data.extend_from_slice(&[0u8; 4]); // padding
    data.extend_from_slice(seed_key.as_ref());

    let ix = Instruction {
        program_id: ids::pyth_price_resolver(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data,
    };
    (ix, state)
}

fn build_resolve_ix(state: Pubkey, feed: Pubkey) -> Instruction {
    Instruction {
        program_id: ids::pyth_price_resolver(),
        accounts: vec![
            AccountMeta::new_readonly(state, false),
            AccountMeta::new_readonly(feed, false),
        ],
        data: vec![0u8],
    }
}

#[test]
fn resolve_gte_yes_when_price_meets_threshold() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let feed_owner = Pubkey::new_unique();
    let feed = Pubkey::new_unique();

    // Threshold: price >= 300_00000000 with expo -8 (i.e. $300 if expo -8)
    let threshold_price = 300_00000000_i64;
    let threshold_expo = -8_i32;

    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        /* earliest_slot = */ 500,
        threshold_price,
        threshold_expo,
        /* comparison = GreaterThanOrEqual */ 0,
    );

    let init = mollusk.process_and_validate_instruction(
        &init_ix,
        &[
            (payer, Account::new(1_000_000_000_000, 0, &system_program_id())),
            (state_pda, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let state_account = init.get_account(&state_pda).expect("state").clone();

    // Bump slot past earliest_slot.
    mollusk.sysvars.clock.slot = 600;

    // Feed reports $350 with matching exponent → YES.
    let feed_account = pyth_feed_account(feed_owner, 350_00000000_i64, threshold_expo);
    let result = mollusk.process_and_validate_instruction(
        &build_resolve_ix(state_pda, feed),
        &[
            (state_pda, state_account),
            (feed, feed_account),
        ],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[1u8]); // Yes
}

#[test]
fn resolve_gte_no_when_price_below_threshold() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let feed_owner = Pubkey::new_unique();
    let feed = Pubkey::new_unique();

    let threshold_price = 300_00000000_i64;
    let threshold_expo = -8_i32;

    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        500,
        threshold_price,
        threshold_expo,
        0, // GTE
    );

    let init = mollusk.process_and_validate_instruction(
        &init_ix,
        &[
            (payer, Account::new(1_000_000_000_000, 0, &system_program_id())),
            (state_pda, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let state_account = init.get_account(&state_pda).expect("state").clone();

    mollusk.sysvars.clock.slot = 600;

    // Feed reports $250 → NO.
    let feed_account = pyth_feed_account(feed_owner, 250_00000000_i64, threshold_expo);
    let result = mollusk.process_and_validate_instruction(
        &build_resolve_ix(state_pda, feed),
        &[
            (state_pda, state_account),
            (feed, feed_account),
        ],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[2u8]); // No
}

#[test]
fn resolve_pre_earliest_slot_returns_unresolved() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let feed_owner = Pubkey::new_unique();
    let feed = Pubkey::new_unique();

    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        /* earliest_slot = */ 500,
        300_00000000,
        -8,
        0,
    );
    let init = mollusk.process_and_validate_instruction(
        &init_ix,
        &[
            (payer, Account::new(1_000_000_000_000, 0, &system_program_id())),
            (state_pda, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let state_account = init.get_account(&state_pda).expect("state").clone();

    // Stay at slot 100 < earliest 500.
    let feed_account = pyth_feed_account(feed_owner, 350_00000000, -8);
    let result = mollusk.process_and_validate_instruction(
        &build_resolve_ix(state_pda, feed),
        &[
            (state_pda, state_account),
            (feed, feed_account),
        ],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[0u8]); // Unresolved
}

#[test]
fn resolve_with_mismatched_exponent_returns_invalid() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 600;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let feed_owner = Pubkey::new_unique();
    let feed = Pubkey::new_unique();

    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        500,
        300_00000000,
        /* threshold_expo = */ -8,
        0,
    );
    let init = mollusk.process_and_validate_instruction(
        &init_ix,
        &[
            (payer, Account::new(1_000_000_000_000, 0, &system_program_id())),
            (state_pda, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let state_account = init.get_account(&state_pda).expect("state").clone();

    // Feed reports with a DIFFERENT exponent — resolver should return Invalid.
    let feed_account = pyth_feed_account(feed_owner, 350_000, /* expo = */ -6);
    let result = mollusk.process_and_validate_instruction(
        &build_resolve_ix(state_pda, feed),
        &[
            (state_pda, state_account),
            (feed, feed_account),
        ],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[3u8]); // Invalid
}
