//! Differential test: conditional-tokens Split.
//!
//! Split is the **headline path** — it's the bytecode-level analogue
//! of our collateral-conservation theorem (every Split adds exactly
//! `amount` to the vault and mints exactly `amount` YES + `amount` NO
//! to the user). Proving the binary respects that contract is the
//! Level-2 goal; this test is the byte+CU baseline.
//!
//! Setup chain (driven by Mollusk because we want a single source of
//! truth for the post-init state; both engines then run the Split
//! against snapshots of that state):
//!   1. slot-resolver Initialize → resolver_state
//!   2. conditional-tokens InitializeMarket → market + yes_mint +
//!      no_mint + vault
//!   3. Build user_yes / user_no token accounts as pre-init
//!      (mollusk_svm_programs_token helpers)
//!   4. Run Split via BOTH engines from the snapshotted state,
//!      assert byte+CU equality.

use janus_tests_qedsvm::{
    assert_runs_equal, mollusk_to_qedsvm, print_match, run_mollusk, run_qedsvm, DiffFixture,
    ExtraProgram, MolluskAccount, MolluskLoader,
};
use mollusk_account::Account;
use mollusk_svm::{result::Check, Mollusk};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_program_option::COption;
use solana_program_pack::Pack;
use spl_token_interface::state::{Account as SplTokenAccount, AccountState, Mint};

const CONDITIONAL_TOKENS_ID: &str = "SH9ghSowHqqWR5YcXVtmkXjt8is1qERCmxHXEvf5sw1";
const SLOT_RESOLVER_ID: &str = "3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj";

fn ct_id() -> Pubkey { CONDITIONAL_TOKENS_ID.parse().unwrap() }
fn slot_id() -> Pubkey { SLOT_RESOLVER_ID.parse().unwrap() }
fn token_id() -> Pubkey { mollusk_svm_programs_token::token::ID }
fn system_id() -> Pubkey { "11111111111111111111111111111111".parse().unwrap() }

fn sbf_dir() -> String {
    std::env::var("JANUS_SBF_DIR").unwrap_or_else(|_| "../target/deploy".to_string())
}
fn ct_so_path() -> String { format!("{}/janus_conditional_tokens", sbf_dir()) }
fn ct_so_bytes() -> Vec<u8> { std::fs::read(format!("{}.so", ct_so_path())).unwrap() }
fn slot_so_path() -> String { format!("{}/janus_slot_height_resolver", sbf_dir()) }
fn slot_so_bytes() -> Vec<u8> { std::fs::read(format!("{}.so", slot_so_path())).unwrap() }

// PDA helpers — same seeds as the on-chain program.
fn pda_market(coll: &Pubkey, resolver_state: &Pubkey, deadline: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"market", coll.as_ref(), resolver_state.as_ref(), &deadline.to_le_bytes()],
        &ct_id(),
    )
}
fn pda_yes_mint(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"yes", market.as_ref()], &ct_id())
}
fn pda_no_mint(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"no", market.as_ref()], &ct_id())
}
fn pda_vault(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", market.as_ref()], &ct_id())
}
fn pda_slot_resolver(seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"slot-resolver", seed.as_ref()], &slot_id())
}

/// Build a mint account directly (using spl_token_interface) so we
/// don't have to thread an InitializeMint instruction through Mollusk.
fn mint_account(authority: &Pubkey, decimals: u8, supply: u64) -> Account {
    let mint = Mint {
        mint_authority: COption::Some(*authority),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    mollusk_svm_programs_token::token::create_account_for_mint(mint)
}

fn token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Account {
    let token_account = SplTokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    mollusk_svm_programs_token::token::create_account_for_token_account(token_account)
}

#[test]
fn diff_split() {
    // ---- World-build via mollusk: init resolver + init market ----
    let mut mollusk = Mollusk::new(&ct_id(), &ct_so_path());
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk.add_program(&slot_id(), &slot_so_path());
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let coll_mint = Pubkey::new_unique();
    let deadline_slot: u64 = 1_000;
    let seed = Pubkey::new_unique();

    // 1. init slot resolver
    let (resolver_state, rbump) = pda_slot_resolver(&seed);
    let init_r = {
        let mut d = Vec::with_capacity(49);
        d.push(1u8);
        d.push(1u8); // outcome=Yes
        d.push(rbump);
        d.extend_from_slice(&[0u8; 6]);
        d.extend_from_slice(&deadline_slot.to_le_bytes());
        d.extend_from_slice(seed.as_ref());
        d
    };
    let init_r_ix = Instruction {
        program_id: slot_id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(resolver_state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(system_id(), false),
        ],
        data: init_r,
    };
    let r1 = mollusk.process_and_validate_instruction(
        &init_r_ix,
        &[
            (payer, Account::new(1_000_000_000_000, 0, &system_id())),
            (resolver_state, Account::default()),
            (authority, Account::new(0, 0, &system_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let payer_acct = r1.get_account(&payer).unwrap().clone();
    let resolver_state_acct = r1.get_account(&resolver_state).unwrap().clone();

    // 2. init market
    let (market, mb) = pda_market(&coll_mint, &resolver_state, deadline_slot);
    let (yes_mint, yb) = pda_yes_mint(&market);
    let (no_mint, nb) = pda_no_mint(&market);
    let (vault, vb) = pda_vault(&market);

    let init_m = {
        let mut d = Vec::with_capacity(17);
        d.push(0u8);
        d.extend_from_slice(&deadline_slot.to_le_bytes());
        d.push(mb);
        d.push(yb);
        d.push(nb);
        d.push(vb);
        d.extend_from_slice(&[0u8; 4]);
        d
    };
    let coll_mint_acct = mint_account(&payer, 6, 0);
    let init_m_ix = Instruction {
        program_id: ct_id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(coll_mint, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(slot_id(), false),
            AccountMeta::new_readonly(resolver_state, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(token_id(), false),
            AccountMeta::new_readonly(system_id(), false),
        ],
        data: init_m,
    };
    let resolver_program_acct = Account {
        lamports: 1,
        data: vec![],
        owner: mollusk_svm::program::loader_keys::LOADER_V3,
        executable: true,
        rent_epoch: 0,
    };
    let r2 = mollusk.process_and_validate_instruction(
        &init_m_ix,
        &[
            (payer, payer_acct),
            (market, Account::default()),
            (coll_mint, coll_mint_acct.clone()),
            (yes_mint, Account::default()),
            (no_mint, Account::default()),
            (vault, Account::default()),
            (slot_id(), resolver_program_acct),
            (resolver_state, resolver_state_acct),
            (authority, Account::new(0, 0, &system_id())),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    // Capture post-init state for the Split call.
    let market_acct = r2.get_account(&market).unwrap().clone();
    let yes_mint_acct = r2.get_account(&yes_mint).unwrap().clone();
    let no_mint_acct = r2.get_account(&no_mint).unwrap().clone();
    let vault_acct = r2.get_account(&vault).unwrap().clone();

    // 3. Build user accounts.
    let user_collateral = Pubkey::new_unique();
    let user_yes = Pubkey::new_unique();
    let user_no = Pubkey::new_unique();
    let initial_collateral: u64 = 5_000_000;
    let user_coll_acct = token_account(&coll_mint, &user, initial_collateral);
    let user_yes_acct = token_account(&yes_mint, &user, 0);
    let user_no_acct = token_account(&no_mint, &user, 0);

    // ---- The Split instruction ----
    let split_amount: u64 = 1_000_000;
    let split_data = {
        let mut d = Vec::with_capacity(9);
        d.push(1u8); // Split tag
        d.extend_from_slice(&split_amount.to_le_bytes());
        d
    };
    let split_ix = Instruction {
        program_id: ct_id(),
        accounts: vec![
            AccountMeta::new_readonly(user, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(user_yes, false),
            AccountMeta::new(user_no, false),
            AccountMeta::new_readonly(token_id(), false),
        ],
        data: split_data,
    };

    let mollusk_accounts: Vec<(Pubkey, MolluskAccount)> = vec![
        (user, Account::new(1_000_000, 0, &system_id())),
        (market, market_acct),
        (user_collateral, user_coll_acct),
        (vault, vault_acct),
        (yes_mint, yes_mint_acct),
        (no_mint, no_mint_acct),
        (user_yes, user_yes_acct),
        (user_no, user_no_acct),
        mollusk_svm_programs_token::token::keyed_account(),
    ];
    let qedsvm_accounts = mollusk_to_qedsvm(&mollusk_accounts);

    let ct_so_bytes_v = ct_so_bytes();
    let ct_so_path_v = ct_so_path();
    let token_elf: &[u8] = mollusk_svm_programs_token::token::ELF;
    let fixture = DiffFixture {
        name: "conditional_tokens_split",
        program_id: ct_id(),
        program_so_path: &ct_so_path_v,
        program_so_bytes: &ct_so_bytes_v,
        instruction: split_ix,
        mollusk_accounts,
        qedsvm_accounts,
        extra_programs: vec![ExtraProgram {
            id: token_id(),
            elf: token_elf,
            loader: MolluskLoader::V2,
        }],
    };

    let m = run_mollusk(&fixture);
    let q = run_qedsvm(&fixture);
    print_match("mollusk", &m);
    print_match("qedsvm ", &q);
    if let Err(msg) = assert_runs_equal("conditional_tokens_split", &m, &q) {
        panic!("{msg}");
    }
}
