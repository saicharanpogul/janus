//! Differential test: slot-resolver Resolve (read-only, no CPI).
//! If this passes, we know qedsvm handles our program's basic paths
//! and the previous Initialize failure is specifically about the
//! System Program CPI for CreateAccount.

use janus_tests_qedsvm::{
    assert_runs_equal, mollusk_to_qedsvm, print_match, run_mollusk, run_qedsvm, DiffFixture,
    MolluskAccount,
};
use mollusk_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const SLOT_RESOLVER_PROGRAM_ID: &str = "3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj";

fn program_id() -> Pubkey {
    SLOT_RESOLVER_PROGRAM_ID.parse().unwrap()
}

fn sbf_path() -> String {
    let dir =
        std::env::var("JANUS_SBF_DIR").unwrap_or_else(|_| "../target/deploy".to_string());
    format!("{dir}/janus_slot_height_resolver")
}

fn sbf_bytes() -> Vec<u8> {
    std::fs::read(format!("{}.so", sbf_path())).expect("SBF binary present")
}

fn slot_resolver_pda(seed_key: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"slot-resolver", seed_key.as_ref()], &program_id())
}

/// Construct a pre-initialized resolver state account, so Resolve can
/// read it without going through Initialize.
fn build_resolved_state_account(outcome: u8, target_slot: u64, seed_key: &Pubkey, bump: u8) -> Account {
    let mut data = vec![0u8; 48];
    data[0] = bump;
    data[1] = outcome;
    // bytes [2..8] padding
    data[8..16].copy_from_slice(&target_slot.to_le_bytes());
    data[16..48].copy_from_slice(seed_key.as_ref());
    Account {
        lamports: 1_000_000,
        data,
        owner: program_id().into(),
        executable: false,
        rent_epoch: 0,
    }
}

#[test]
fn diff_resolve_before_deadline() {
    let seed_key = Pubkey::new_unique();
    let (state, bump) = slot_resolver_pda(&seed_key);
    let outcome = 1u8;
    let target_slot = 500u64;

    let resolve_ix = Instruction {
        program_id: program_id(),
        accounts: vec![AccountMeta::new_readonly(state, false)],
        data: vec![0u8], // Resolve tag
    };

    let state_account = build_resolved_state_account(outcome, target_slot, &seed_key, bump);
    let mollusk_accounts: Vec<(Pubkey, MolluskAccount)> = vec![(state, state_account)];
    let qedsvm_accounts = mollusk_to_qedsvm(&mollusk_accounts);

    let so_bytes = sbf_bytes();
    let so_path = sbf_path();
    let fixture = DiffFixture {
        name: "slot_resolver_resolve",
        program_id: program_id(),
        program_so_path: &so_path,
        program_so_bytes: &so_bytes,
        instruction: resolve_ix,
        mollusk_accounts,
        qedsvm_accounts,
        extra_programs: vec![],
    };

    let m = run_mollusk(&fixture);
    let q = run_qedsvm(&fixture);

    print_match("mollusk", &m);
    print_match("qedsvm ", &q);

    if let Err(msg) = assert_runs_equal("slot_resolver_resolve", &m, &q) {
        panic!("{msg}");
    }
}
