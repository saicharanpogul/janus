//! Integration test for the slot-height-resolver program.

use janus_tests::{ids, pda, so_paths};
use mollusk_svm::{result::Check, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

fn system_program_id() -> Pubkey {
    "11111111111111111111111111111111".parse().unwrap()
}

fn mollusk() -> Mollusk {
    Mollusk::new(&ids::slot_height_resolver(), so_paths::SLOT_HEIGHT_RESOLVER)
}

/// Build the slot-resolver Initialize instruction. We hand-roll this here
/// rather than reusing `janus_tests::ix` so the test stays close to the
/// on-chain byte layout for debugging.
fn build_initialize_ix(
    payer: Pubkey,
    authority: Pubkey,
    seed_key: Pubkey,
    outcome: u8,
    target_slot: u64,
) -> (Instruction, Pubkey) {
    let (state, bump) = pda::slot_resolver_state(&seed_key);

    let mut data = Vec::with_capacity(49);
    data.push(1u8); // INSTRUCTION_INITIALIZE
    data.push(outcome);
    data.push(bump);
    data.extend_from_slice(&[0u8; 6]); // padding
    data.extend_from_slice(&target_slot.to_le_bytes());
    data.extend_from_slice(seed_key.as_ref());

    let ix = Instruction {
        program_id: ids::slot_height_resolver(),
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

fn build_resolve_ix(state: Pubkey) -> Instruction {
    Instruction {
        program_id: ids::slot_height_resolver(),
        accounts: vec![AccountMeta::new_readonly(state, false)],
        data: vec![0u8], // RESOLVE_INSTRUCTION_TAG
    }
}

#[test]
fn initialize_creates_state_pda_with_expected_bytes() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let outcome = 1u8;
    let target_slot = 500u64;

    let (init_ix, state_pda) = build_initialize_ix(payer, authority, seed_key, outcome, target_slot);

    let result = mollusk.process_and_validate_instruction(
        &init_ix,
        &[
            (payer, Account::new(1_000_000_000_000, 0, &system_program_id())),
            (state_pda, Account::default()),
            (authority, Account::new(0, 0, &system_program_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    let state_account = result
        .get_account(&state_pda)
        .expect("resolver state should exist after init");
    assert_eq!(state_account.owner, ids::slot_height_resolver());
    assert_eq!(state_account.data.len(), 48);
    assert_eq!(state_account.data[1], outcome);

    // target_slot lives at offset 40 (1+1+6+32 = 40 since authority precedes it).
    let mut slot_bytes = [0u8; 8];
    slot_bytes.copy_from_slice(&state_account.data[40..48]);
    assert_eq!(u64::from_le_bytes(slot_bytes), target_slot);
}

#[test]
fn resolve_before_target_slot_returns_unresolved() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();

    let (init_ix, state_pda) = build_initialize_ix(payer, authority, seed_key, 1, 500);
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

    // Still at slot 100 < target 500.
    let resolve_ix = build_resolve_ix(state_pda);
    let result = mollusk.process_and_validate_instruction(
        &resolve_ix,
        &[(state_pda, state_account)],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[0u8]); // ResolutionOutcome::Unresolved
}

#[test]
fn resolve_after_target_slot_returns_configured_outcome() {
    let mut mollusk = mollusk();
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let outcome = 2u8; // No

    let (init_ix, state_pda) = build_initialize_ix(payer, authority, seed_key, outcome, 500);
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

    // Advance clock past the target.
    mollusk.sysvars.clock.slot = 600;

    let resolve_ix = build_resolve_ix(state_pda);
    let result = mollusk.process_and_validate_instruction(
        &resolve_ix,
        &[(state_pda, state_account)],
        &[Check::success()],
    );
    assert_eq!(result.return_data.as_slice(), &[outcome]);
}

#[test]
fn pda_derivation_is_deterministic() {
    let seed_key = Pubkey::new_unique();
    let (a, _) = pda::slot_resolver_state(&seed_key);
    let (b, _) = pda::slot_resolver_state(&seed_key);
    assert_eq!(a, b);
}
