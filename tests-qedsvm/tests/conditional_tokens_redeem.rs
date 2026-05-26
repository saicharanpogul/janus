//! Differential test: conditional-tokens Redeem on a resolved market.
//!
//! Setup chain via Mollusk: resolver init → market init → split (user
//! gets YES + NO) → advance slot past deadline → resolve (market →
//! ResolvedYes) → run Redeem(YES, amount) on the snapshotted state.
//!
//! Exercises post-resolution payout: burns user's YES tokens and PDA-
//! signs a Token::Transfer from the vault back to the user's collateral
//! ATA.

use janus_tests_qedsvm::{
    assert_runs_equal, mollusk_to_qedsvm, print_match, run_mollusk, run_qedsvm, DiffFixture,
    ExtraProgram, MolluskAccount, MolluskLoader,
};
use mollusk_account::Account;
use mollusk_svm::{result::Check, Mollusk};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_program_option::COption;
use spl_token_interface::state::{Account as SplTokenAccount, AccountState, Mint};

const CONDITIONAL_TOKENS_ID: &str = "SH9ghSowHqqWR5YcXVtmkXjt8is1qERCmxHXEvf5sw1";
const SLOT_RESOLVER_ID: &str = "3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj";

fn ct_id() -> Pubkey { CONDITIONAL_TOKENS_ID.parse().unwrap() }
fn slot_id() -> Pubkey { SLOT_RESOLVER_ID.parse().unwrap() }
fn token_id() -> Pubkey { mollusk_svm_programs_token::token::ID }
fn system_id() -> Pubkey { "11111111111111111111111111111111".parse().unwrap() }

fn sbf_dir() -> String { std::env::var("JANUS_SBF_DIR").unwrap_or_else(|_| "../target/deploy".to_string()) }
fn ct_so_path() -> String { format!("{}/janus_conditional_tokens", sbf_dir()) }
fn ct_so_bytes() -> Vec<u8> { std::fs::read(format!("{}.so", ct_so_path())).unwrap() }
fn slot_so_path() -> String { format!("{}/janus_slot_height_resolver", sbf_dir()) }

fn pda_market(coll: &Pubkey, rs: &Pubkey, deadline: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"market", coll.as_ref(), rs.as_ref(), &deadline.to_le_bytes()],
        &ct_id(),
    )
}
fn pda_yes_mint(market: &Pubkey) -> (Pubkey, u8) { Pubkey::find_program_address(&[b"yes", market.as_ref()], &ct_id()) }
fn pda_no_mint(market: &Pubkey) -> (Pubkey, u8) { Pubkey::find_program_address(&[b"no", market.as_ref()], &ct_id()) }
fn pda_vault(market: &Pubkey) -> (Pubkey, u8) { Pubkey::find_program_address(&[b"vault", market.as_ref()], &ct_id()) }
fn pda_slot(seed: &Pubkey) -> (Pubkey, u8) { Pubkey::find_program_address(&[b"slot-resolver", seed.as_ref()], &slot_id()) }

fn mint_account(authority: &Pubkey, decimals: u8, supply: u64) -> Account {
    mollusk_svm_programs_token::token::create_account_for_mint(Mint {
        mint_authority: COption::Some(*authority),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    })
}
fn token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Account {
    mollusk_svm_programs_token::token::create_account_for_token_account(SplTokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    })
}
fn loader_v3_program() -> Account {
    Account {
        lamports: 1, data: vec![],
        owner: mollusk_svm::program::loader_keys::LOADER_V3,
        executable: true, rent_epoch: 0,
    }
}

#[test]
fn diff_redeem_yes() {
    let mut mollusk = Mollusk::new(&ct_id(), &ct_so_path());
    mollusk.add_program(&slot_id(), &slot_so_path());
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let coll_mint = Pubkey::new_unique();
    let deadline: u64 = 1_000;
    let seed = Pubkey::new_unique();

    // 1. resolver init (outcome=Yes)
    let (rs, rb) = pda_slot(&seed);
    let mut d = Vec::with_capacity(49);
    d.push(1u8); d.push(1u8); d.push(rb);
    d.extend_from_slice(&[0u8; 6]);
    d.extend_from_slice(&deadline.to_le_bytes());
    d.extend_from_slice(seed.as_ref());
    let init_r = Instruction {
        program_id: slot_id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(rs, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(system_id(), false),
        ],
        data: d,
    };
    let r1 = mollusk.process_and_validate_instruction(
        &init_r,
        &[
            (payer, Account::new(1_000_000_000_000, 0, &system_id())),
            (rs, Account::default()),
            (authority, Account::new(0, 0, &system_id())),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let payer_acct = r1.get_account(&payer).unwrap().clone();
    let rs_acct = r1.get_account(&rs).unwrap().clone();

    // 2. market init
    let (market, mb) = pda_market(&coll_mint, &rs, deadline);
    let (yes_mint, yb) = pda_yes_mint(&market);
    let (no_mint, nb) = pda_no_mint(&market);
    let (vault, vb) = pda_vault(&market);
    let coll_mint_acct = mint_account(&payer, 6, 0);
    let mut d = Vec::with_capacity(17);
    d.push(0u8);
    d.extend_from_slice(&deadline.to_le_bytes());
    d.push(mb); d.push(yb); d.push(nb); d.push(vb);
    d.extend_from_slice(&[0u8; 4]);
    let init_m = Instruction {
        program_id: ct_id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(coll_mint, false),
            AccountMeta::new(yes_mint, false),
            AccountMeta::new(no_mint, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(slot_id(), false),
            AccountMeta::new_readonly(rs, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(token_id(), false),
            AccountMeta::new_readonly(system_id(), false),
        ],
        data: d,
    };
    let r2 = mollusk.process_and_validate_instruction(
        &init_m,
        &[
            (payer, payer_acct),
            (market, Account::default()),
            (coll_mint, coll_mint_acct.clone()),
            (yes_mint, Account::default()),
            (no_mint, Account::default()),
            (vault, Account::default()),
            (slot_id(), loader_v3_program()),
            (rs, rs_acct.clone()),
            (authority, Account::new(0, 0, &system_id())),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let market_acct = r2.get_account(&market).unwrap().clone();
    let yes_mint_acct = r2.get_account(&yes_mint).unwrap().clone();
    let no_mint_acct = r2.get_account(&no_mint).unwrap().clone();
    let vault_acct = r2.get_account(&vault).unwrap().clone();

    // 3. user collateral + ATAs + split 1_000_000
    let user_collateral = Pubkey::new_unique();
    let user_yes = Pubkey::new_unique();
    let user_no = Pubkey::new_unique();
    let user_coll_acct = token_account(&coll_mint, &user, 5_000_000);
    let user_yes_acct = token_account(&yes_mint, &user, 0);
    let user_no_acct = token_account(&no_mint, &user, 0);
    let split_amount: u64 = 1_000_000;
    let mut d = Vec::with_capacity(9);
    d.push(1u8);
    d.extend_from_slice(&split_amount.to_le_bytes());
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
        data: d,
    };
    let r3 = mollusk.process_and_validate_instruction(
        &split_ix,
        &[
            (user, Account::new(1_000_000, 0, &system_id())),
            (market, market_acct.clone()),
            (user_collateral, user_coll_acct),
            (vault, vault_acct),
            (yes_mint, yes_mint_acct),
            (no_mint, no_mint_acct),
            (user_yes, user_yes_acct),
            (user_no, user_no_acct),
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::success()],
    );
    let vault_acct = r3.get_account(&vault).unwrap().clone();
    let user_coll_acct = r3.get_account(&user_collateral).unwrap().clone();
    let user_yes_acct = r3.get_account(&user_yes).unwrap().clone();

    // 4. Advance slot past deadline + resolve. resolver_state outcome
    // was set to Yes, so resolution sets market.status = ResolvedYes.
    mollusk.sysvars.clock.slot = deadline + 1;
    let resolve_ix = Instruction {
        program_id: ct_id(),
        accounts: vec![
            AccountMeta::new_readonly(user, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(slot_id(), false),
            AccountMeta::new_readonly(rs, false),
        ],
        data: vec![4u8], // Resolve tag
    };
    let r4 = mollusk.process_and_validate_instruction(
        &resolve_ix,
        &[
            (user, Account::new(1_000_000, 0, &system_id())),
            (market, market_acct),
            (slot_id(), loader_v3_program()),
            (rs, rs_acct),
        ],
        &[Check::success()],
    );
    let market_acct = r4.get_account(&market).unwrap().clone();
    assert_eq!(market_acct.data[1], 1, "market.status should be ResolvedYes=1");

    // 5. The Redeem instruction: burn 400_000 YES, receive 400_000 collateral.
    let redeem_amount: u64 = 400_000;
    let mut d = Vec::with_capacity(9);
    d.push(3u8); // Redeem tag
    d.extend_from_slice(&redeem_amount.to_le_bytes());
    let redeem_ix = Instruction {
        program_id: ct_id(),
        accounts: vec![
            AccountMeta::new_readonly(user, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(user_collateral, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(yes_mint, false),       // winning mint
            AccountMeta::new(user_yes, false),
            AccountMeta::new_readonly(token_id(), false),
        ],
        data: d,
    };

    let mollusk_accounts: Vec<(Pubkey, MolluskAccount)> = vec![
        (user, Account::new(1_000_000, 0, &system_id())),
        (market, market_acct),
        (user_collateral, user_coll_acct),
        (vault, vault_acct),
        (yes_mint, r3.get_account(&yes_mint).unwrap().clone()),
        (user_yes, user_yes_acct),
        mollusk_svm_programs_token::token::keyed_account(),
    ];
    let qedsvm_accounts = mollusk_to_qedsvm(&mollusk_accounts);

    let ct_so_bytes_v = ct_so_bytes();
    let ct_so_path_v = ct_so_path();
    let token_elf: &[u8] = mollusk_svm_programs_token::token::ELF;
    let fixture = DiffFixture {
        name: "conditional_tokens_redeem_yes",
        program_id: ct_id(),
        program_so_path: &ct_so_path_v,
        program_so_bytes: &ct_so_bytes_v,
        instruction: redeem_ix,
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
    if let Err(msg) = assert_runs_equal("conditional_tokens_redeem", &m, &q) {
        panic!("{msg}");
    }
}
