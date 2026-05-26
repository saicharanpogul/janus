//! Differential test: slot-height-resolver Initialize through mollusk
//! and qedsvm. Smallest Janus program (≈12 KB SBF), no CPI complexity.

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

fn system_program_id() -> Pubkey {
    "11111111111111111111111111111111".parse().unwrap()
}

fn sbf_path() -> String {
    let dir =
        std::env::var("JANUS_SBF_DIR").unwrap_or_else(|_| "../target/deploy".to_string());
    format!("{dir}/janus_slot_height_resolver")
}

fn sbf_bytes() -> Vec<u8> {
    let p = format!("{}.so", sbf_path());
    std::fs::read(&p).unwrap_or_else(|e| {
        panic!(
            "missing SBF binary at {p}: {e}\n\nRun `cargo build-sbf` from the janus repo root first."
        )
    })
}

fn slot_resolver_pda(seed_key: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"slot-resolver", seed_key.as_ref()], &program_id())
}

fn build_initialize_ix(
    payer: Pubkey,
    authority: Pubkey,
    seed_key: Pubkey,
    outcome: u8,
    target_slot: u64,
) -> (Instruction, Pubkey) {
    let (state, bump) = slot_resolver_pda(&seed_key);

    let mut data = Vec::with_capacity(49);
    data.push(1u8); // Initialize tag
    data.push(outcome);
    data.push(bump);
    data.extend_from_slice(&[0u8; 6]);
    data.extend_from_slice(&target_slot.to_le_bytes());
    data.extend_from_slice(seed_key.as_ref());

    let ix = Instruction {
        program_id: program_id(),
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

fn system_program_account() -> Account {
    Account {
        lamports: 1,
        data: vec![],
        owner: solana_sdk_ids::native_loader::id().into(),
        executable: true,
        rent_epoch: 0,
    }
}

#[test]
fn diff_initialize() {
    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let seed_key = Pubkey::new_unique();
    let outcome = 1u8;
    let target_slot = 500u64;

    let (ix, state) = build_initialize_ix(payer, authority, seed_key, outcome, target_slot);

    let mollusk_accounts: Vec<(Pubkey, MolluskAccount)> = vec![
        (payer, Account::new(1_000_000_000_000, 0, &system_program_id())),
        (state, Account::default()),
        (authority, Account::new(0, 0, &system_program_id())),
        (
            system_program_id(),
            mollusk_svm::program::keyed_account_for_system_program().1,
        ),
    ];
    let qedsvm_accounts = mollusk_to_qedsvm(&mollusk_accounts);

    let so_bytes = sbf_bytes();
    let so_path = sbf_path();
    let fixture = DiffFixture {
        name: "slot_resolver_initialize",
        program_id: program_id(),
        program_so_path: &so_path,
        program_so_bytes: &so_bytes,
        instruction: ix,
        mollusk_accounts,
        qedsvm_accounts,
        extra_programs: vec![],
    };

    let m = run_mollusk(&fixture);
    let q = run_qedsvm(&fixture);

    print_match("mollusk", &m);
    print_match("qedsvm ", &q);

    if let Err(msg) = assert_runs_equal("slot_resolver_initialize", &m, &q) {
        panic!("{msg}");
    }
}
