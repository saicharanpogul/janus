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
///   - [0..8]    Anchor discriminator for PriceUpdateV2
///   - [41..73]  feed_id (32 bytes)
///   - [73..81]  price (i64 LE)
///   - [89..93]  exponent (i32 LE)
///   - [125..133] posted_slot (u64 LE)
/// All other bytes are zero. Total length 133+ bytes.
const PRICE_UPDATE_V2_DISCRIMINATOR: [u8; 8] = [34, 241, 35, 99, 157, 126, 244, 205];

fn pyth_feed_account(
    owner: Pubkey,
    feed_id: [u8; 32],
    price: i64,
    expo: i32,
    posted_slot: u64,
) -> Account {
    let mut data = vec![0u8; 200];
    data[0..8].copy_from_slice(&PRICE_UPDATE_V2_DISCRIMINATOR);
    data[41..73].copy_from_slice(&feed_id);
    data[73..81].copy_from_slice(&price.to_le_bytes());
    data[89..93].copy_from_slice(&expo.to_le_bytes());
    data[125..133].copy_from_slice(&posted_slot.to_le_bytes());
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
    feed_id: [u8; 32],
    earliest_slot: u64,
    max_staleness_slots: u64,
    threshold_price: i64,
    threshold_expo: i32,
    comparison: u8,
) -> (Instruction, Pubkey) {
    let (state, bump) = pyth_resolver_state_pda(&seed_key);

    let mut data = Vec::with_capacity(136);
    data.push(bump);                                       // [0]
    data.push(comparison);                                  // [1]
    data.extend_from_slice(&[0u8; 6]);                      // [2..8]
    data.extend_from_slice(price_feed.as_ref());            // [8..40]
    data.extend_from_slice(&feed_id);                       // [40..72]
    data.extend_from_slice(&earliest_slot.to_le_bytes());   // [72..80]
    data.extend_from_slice(&max_staleness_slots.to_le_bytes()); // [80..88]
    data.extend_from_slice(&threshold_price.to_le_bytes()); // [88..96]
    data.extend_from_slice(&threshold_expo.to_le_bytes());  // [96..100]
    data.extend_from_slice(&[0u8; 4]);                      // [100..104]
    data.extend_from_slice(seed_key.as_ref());              // [104..136]
    assert_eq!(data.len(), 136, "init data must be 136 bytes");

    let mut full = Vec::with_capacity(1 + data.len());
    full.push(1u8); // INSTRUCTION_INITIALIZE
    full.extend_from_slice(&data);

    let ix = Instruction {
        program_id: ids::pyth_price_resolver(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: full,
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

    let feed_id = [7u8; 32];
    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        feed_id,
        /* earliest_slot = */ 500,
        /* max_staleness_slots = */ 100,
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
    let feed_account = pyth_feed_account(
        feed_owner, feed_id, 350_00000000_i64, threshold_expo, /* posted_slot = */ 590,
    );
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

    let feed_id = [7u8; 32];
    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        feed_id,
        500,
        100, // max_staleness_slots
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
    let feed_account = pyth_feed_account(
        feed_owner, feed_id, 250_00000000_i64, threshold_expo, 590,
    );
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

    let feed_id = [7u8; 32];
    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        feed_id,
        /* earliest_slot = */ 500,
        /* max_staleness_slots = */ 100,
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
    let feed_account = pyth_feed_account(feed_owner, feed_id, 350_00000000, -8, 90);
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

    let feed_id = [7u8; 32];
    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        feed_id,
        500,
        100, // max_staleness_slots
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
    let feed_account = pyth_feed_account(feed_owner, feed_id, 350_000, /* expo = */ -6, 595);
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

#[test]
fn resolve_with_wrong_feed_id_returns_invalid() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 600;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let feed_owner = Pubkey::new_unique();
    let feed = Pubkey::new_unique();

    let configured_feed_id = [7u8; 32];
    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        configured_feed_id,
        500,
        100,
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

    // Feed account stamped with a DIFFERENT feed_id — defends against an
    // attacker substituting the feed contents at our configured address.
    let attacker_feed_id = [9u8; 32];
    let feed_account =
        pyth_feed_account(feed_owner, attacker_feed_id, 350_00000000, -8, 595);
    let result = mollusk.process_and_validate_instruction(
        &build_resolve_ix(state_pda, feed),
        &[(state_pda, state_account), (feed, feed_account)],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[3u8], "feed_id mismatch → INVALID");
}

#[test]
fn resolve_with_stale_feed_returns_invalid() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 600;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let feed_owner = Pubkey::new_unique();
    let feed = Pubkey::new_unique();

    let feed_id = [7u8; 32];
    let max_staleness: u64 = 100;
    let (init_ix, state_pda) = build_initialize_ix(
        payer,
        authority,
        seed_key,
        feed,
        feed_id,
        500,
        max_staleness,
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

    // Feed posted way before the staleness window — clock=600,
    // max_staleness=100, but posted_slot=400 → 600-400=200 > 100, stale.
    let feed_account = pyth_feed_account(feed_owner, feed_id, 350_00000000, -8, 400);
    let result = mollusk.process_and_validate_instruction(
        &build_resolve_ix(state_pda, feed),
        &[(state_pda, state_account), (feed, feed_account)],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[3u8], "stale feed → INVALID");
}
