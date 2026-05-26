//! Differential test: pyth-price-resolver Resolve (read-only).
//! Second confirmation that the post-execution buffer parser works
//! for pure-read Pinocchio handlers.

use janus_tests_qedsvm::{
    assert_runs_equal, mollusk_to_qedsvm, print_match, run_mollusk, run_qedsvm, DiffFixture,
    MolluskAccount,
};
use mollusk_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const PYTH_RESOLVER_PROGRAM_ID: &str = "3WDargKHd1UaP9UKPhJY8pF5bv5zJnaFAYDA9uahs5aL";

fn program_id() -> Pubkey {
    PYTH_RESOLVER_PROGRAM_ID.parse().unwrap()
}

fn sbf_path() -> String {
    let dir =
        std::env::var("JANUS_SBF_DIR").unwrap_or_else(|_| "../target/deploy".to_string());
    format!("{dir}/janus_pyth_price_resolver")
}

fn sbf_bytes() -> Vec<u8> {
    std::fs::read(format!("{}.so", sbf_path())).expect("SBF binary present")
}

fn pyth_resolver_pda(seed_key: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"pyth-resolver", seed_key.as_ref()], &program_id())
}

/// Build a pre-initialized pyth-resolver state (136 bytes per the
/// program's layout: bump + comparison + padding + price_feed +
/// feed_id + earliest_slot + max_staleness_slots + threshold_price +
/// threshold_expo + padding + seed_key).
fn build_state_account(
    bump: u8,
    seed_key: &Pubkey,
    price_feed: &Pubkey,
) -> Account {
    let mut data = vec![0u8; 136];
    data[0] = bump;
    data[1] = 0; // comparison: gte
    data[8..40].copy_from_slice(price_feed.as_ref());
    // feed_id (32 bytes) at [40..72]: zeroed
    // earliest_slot at [72..80]
    data[72..80].copy_from_slice(&100u64.to_le_bytes());
    // max_staleness_slots at [80..88]
    data[80..88].copy_from_slice(&3600u64.to_le_bytes());
    // threshold_price at [88..96]
    data[88..96].copy_from_slice(&100_000_000i64.to_le_bytes());
    // threshold_expo at [96..100]
    data[96..100].copy_from_slice(&(-8i32).to_le_bytes());
    data[104..136].copy_from_slice(seed_key.as_ref());
    Account {
        lamports: 1_000_000,
        data,
        owner: program_id().into(),
        executable: false,
        rent_epoch: 0,
    }
}

#[test]
fn diff_resolve_before_earliest_slot() {
    let seed_key = Pubkey::new_unique();
    let (state, bump) = pyth_resolver_pda(&seed_key);
    let price_feed = Pubkey::new_unique();

    // Resolve takes state + feed account.
    let resolve_ix = Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new_readonly(state, false),
            AccountMeta::new_readonly(price_feed, false),
        ],
        data: vec![0u8], // Resolve tag
    };

    let state_account = build_state_account(bump, &seed_key, &price_feed);
    // Feed account: empty data (program returns Invalid for missing
    // discriminator without needing valid data).
    let feed_account = Account {
        lamports: 1_000_000,
        data: vec![0u8; 256],
        owner: Pubkey::new_unique().into(),
        executable: false,
        rent_epoch: 0,
    };

    let mollusk_accounts: Vec<(Pubkey, MolluskAccount)> = vec![
        (state, state_account),
        (price_feed, feed_account),
    ];
    let qedsvm_accounts = mollusk_to_qedsvm(&mollusk_accounts);

    let so_bytes = sbf_bytes();
    let so_path = sbf_path();
    let fixture = DiffFixture {
        name: "pyth_resolver_resolve",
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

    if let Err(msg) = assert_runs_equal("pyth_resolver_resolve", &m, &q) {
        panic!("{msg}");
    }
}
