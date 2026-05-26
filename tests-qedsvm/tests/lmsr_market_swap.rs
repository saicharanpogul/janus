//! Differential test: lmsr-market Swap.
//!
//! Exercises **PDA-signed token transfer** — the pool PDA signs a
//! Token::Transfer to move outcome tokens out of one of its vaults
//! into the user's account. Distinct from the conditional-tokens
//! Split CPI shape (which uses MintTo with the program as mint
//! authority).
//!
//! Setup chain via Mollusk: resolver init → market init → split (so
//! the creator has YES+NO to seed the pool) → initialize pool → run
//! Swap through both engines from the same state.

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
const LMSR_MARKET_ID: &str = "GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK";
const SLOT_RESOLVER_ID: &str = "3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj";

fn ct_id() -> Pubkey { CONDITIONAL_TOKENS_ID.parse().unwrap() }
fn lm_id() -> Pubkey { LMSR_MARKET_ID.parse().unwrap() }
fn slot_id() -> Pubkey { SLOT_RESOLVER_ID.parse().unwrap() }
fn token_id() -> Pubkey { mollusk_svm_programs_token::token::ID }
fn system_id() -> Pubkey { "11111111111111111111111111111111".parse().unwrap() }

fn sbf_dir() -> String { std::env::var("JANUS_SBF_DIR").unwrap_or_else(|_| "../target/deploy".to_string()) }
fn lm_so_path() -> String { format!("{}/janus_lmsr_market", sbf_dir()) }
fn lm_so_bytes() -> Vec<u8> { std::fs::read(format!("{}.so", lm_so_path())).unwrap() }
fn ct_so_path() -> String { format!("{}/janus_conditional_tokens", sbf_dir()) }
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
fn pda_pool(market: &Pubkey) -> (Pubkey, u8) { Pubkey::find_program_address(&[b"pool", market.as_ref()], &lm_id()) }
fn pda_pool_yes_vault(pool: &Pubkey) -> (Pubkey, u8) { Pubkey::find_program_address(&[b"yes-vault", pool.as_ref()], &lm_id()) }
fn pda_pool_no_vault(pool: &Pubkey) -> (Pubkey, u8) { Pubkey::find_program_address(&[b"no-vault", pool.as_ref()], &lm_id()) }

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
        lamports: 1,
        data: vec![],
        owner: mollusk_svm::program::loader_keys::LOADER_V3,
        executable: true,
        rent_epoch: 0,
    }
}

#[test]
fn diff_swap() {
    // ---- Setup: drive resolver init + market init + split + pool init through Mollusk ----
    let mut mollusk = Mollusk::new(&lm_id(), &lm_so_path());
    mollusk.add_program(&ct_id(), &ct_so_path());
    mollusk.add_program(&slot_id(), &slot_so_path());
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk.sysvars.clock.slot = 100;

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let coll_mint = Pubkey::new_unique();
    let deadline: u64 = 1_000;
    let seed = Pubkey::new_unique();

    // resolver init
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

    // market init
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
            (rs, rs_acct),
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
    let payer_acct = r2.get_account(&payer).unwrap().clone();

    // Authority needs collateral to split.
    let user_collateral = Pubkey::new_unique();
    let user_yes = Pubkey::new_unique();
    let user_no = Pubkey::new_unique();
    let user_coll_acct = token_account(&coll_mint, &authority, 10_000_000);
    let user_yes_acct = token_account(&yes_mint, &authority, 0);
    let user_no_acct = token_account(&no_mint, &authority, 0);

    // split → gives authority 2M YES + 2M NO to seed the pool
    let split_amount: u64 = 2_000_000;
    let mut d = Vec::with_capacity(9);
    d.push(1u8);
    d.extend_from_slice(&split_amount.to_le_bytes());
    let split = Instruction {
        program_id: ct_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
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
        &split,
        &[
            (authority, Account::new(1_000_000_000, 0, &system_id())),
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
    let user_yes_acct = r3.get_account(&user_yes).unwrap().clone();
    let user_no_acct = r3.get_account(&user_no).unwrap().clone();
    let yes_mint_acct = r3.get_account(&yes_mint).unwrap().clone();
    let no_mint_acct = r3.get_account(&no_mint).unwrap().clone();

    // pool init
    let (pool, pb) = pda_pool(&market);
    let (yes_vault, yvb) = pda_pool_yes_vault(&pool);
    let (no_vault, nvb) = pda_pool_no_vault(&pool);
    let subsidy_each: u64 = 1_000_000;
    let mut d = Vec::with_capacity(25);
    d.push(0u8);
    d.extend_from_slice(&subsidy_each.to_le_bytes());
    d.extend_from_slice(&subsidy_each.to_le_bytes());
    d.extend_from_slice(&100u16.to_le_bytes()); // 1% fee
    d.push(pb); d.push(yvb); d.push(nvb);
    d.extend_from_slice(&[0u8; 3]);
    let init_pool = Instruction {
        program_id: lm_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new_readonly(yes_mint, false),
            AccountMeta::new_readonly(no_mint, false),
            AccountMeta::new(yes_vault, false),
            AccountMeta::new(no_vault, false),
            AccountMeta::new(user_yes, false),
            AccountMeta::new(user_no, false),
            AccountMeta::new_readonly(token_id(), false),
            AccountMeta::new_readonly(system_id(), false),
        ],
        data: d,
    };
    let r4 = mollusk.process_and_validate_instruction(
        &init_pool,
        &[
            (authority, Account::new(1_000_000_000_000, 0, &system_id())),
            (pool, Account::default()),
            (market, market_acct.clone()),
            (yes_mint, yes_mint_acct.clone()),
            (no_mint, no_mint_acct.clone()),
            (yes_vault, Account::default()),
            (no_vault, Account::default()),
            (user_yes, user_yes_acct),
            (user_no, user_no_acct),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm::program::keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    let pool_acct = r4.get_account(&pool).unwrap().clone();
    let yes_vault_acct = r4.get_account(&yes_vault).unwrap().clone();
    let no_vault_acct = r4.get_account(&no_vault).unwrap().clone();
    let user_yes_acct = r4.get_account(&user_yes).unwrap().clone();
    let user_no_acct = r4.get_account(&user_no).unwrap().clone();

    // ---- The Swap: YES in, NO out, 100k ----
    let amount_in: u64 = 100_000;
    let min_amount_out: u64 = 80_000;
    let mut d = Vec::with_capacity(25);
    d.push(1u8); // Swap tag
    d.extend_from_slice(&amount_in.to_le_bytes());
    d.extend_from_slice(&min_amount_out.to_le_bytes());
    d.push(0u8); // direction = YES in, NO out
    d.extend_from_slice(&[0u8; 7]);
    let swap_ix = Instruction {
        program_id: lm_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(pool, false),
            AccountMeta::new(yes_vault, false),
            AccountMeta::new(no_vault, false),
            AccountMeta::new(user_yes, false), // input
            AccountMeta::new(user_no, false),  // output
            AccountMeta::new_readonly(token_id(), false),
        ],
        data: d,
    };

    let mollusk_accounts: Vec<(Pubkey, MolluskAccount)> = vec![
        (authority, Account::new(1_000_000, 0, &system_id())),
        (pool, pool_acct),
        (yes_vault, yes_vault_acct),
        (no_vault, no_vault_acct),
        (user_yes, user_yes_acct),
        (user_no, user_no_acct),
        mollusk_svm_programs_token::token::keyed_account(),
    ];
    let qedsvm_accounts = mollusk_to_qedsvm(&mollusk_accounts);

    let lm_so_bytes_v = lm_so_bytes();
    let lm_so_path_v = lm_so_path();
    let token_elf: &[u8] = mollusk_svm_programs_token::token::ELF;
    let fixture = DiffFixture {
        name: "lmsr_market_swap",
        program_id: lm_id(),
        program_so_path: &lm_so_path_v,
        program_so_bytes: &lm_so_bytes_v,
        instruction: swap_ix,
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
    if let Err(msg) = assert_runs_equal("lmsr_market_swap", &m, &q) {
        panic!("{msg}");
    }
}
