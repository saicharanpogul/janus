//! Shared test helpers for Janus integration tests run via Mollusk.

use solana_pubkey::Pubkey;

// ---------------------------------------------------------------------------
// Program IDs — must mirror each program's `declare_id!`.
// ---------------------------------------------------------------------------

pub mod ids {
    use solana_pubkey::Pubkey;

    pub fn conditional_tokens() -> Pubkey {
        "SH9ghSowHqqWR5YcXVtmkXjt8is1qERCmxHXEvf5sw1"
            .parse()
            .unwrap()
    }
    pub fn lmsr_market() -> Pubkey {
        "GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK"
            .parse()
            .unwrap()
    }
    pub fn slot_height_resolver() -> Pubkey {
        "3y75gGqFK1KhNF5k1sMy6ydnw6WLcbn1SPRoYbyRkjMj"
            .parse()
            .unwrap()
    }
    pub fn pyth_price_resolver() -> Pubkey {
        "3WDargKHd1UaP9UKPhJY8pF5bv5zJnaFAYDA9uahs5aL"
            .parse()
            .unwrap()
    }
    pub fn market_factory() -> Pubkey {
        "8ibKxXAWsdqyNG1wExRSvLhKBgXiPpqtE6ZkA277gPwC"
            .parse()
            .unwrap()
    }
}

// ---------------------------------------------------------------------------
// SBF binary paths (built by `cargo build-sbf` from the workspace root).
// ---------------------------------------------------------------------------

pub mod so_paths {
    /// Mollusk expects the path *without* the `.so` extension. Paths are
    /// computed relative to the test crate's manifest directory at compile
    /// time so tests run from any CWD.
    macro_rules! sbf_path {
        ($name:literal) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../target/deploy/", $name)
        };
    }

    pub const CONDITIONAL_TOKENS: &str = sbf_path!("janus_conditional_tokens");
    pub const LMSR_MARKET: &str = sbf_path!("janus_lmsr_market");
    pub const SLOT_HEIGHT_RESOLVER: &str = sbf_path!("janus_slot_height_resolver");
    pub const PYTH_PRICE_RESOLVER: &str = sbf_path!("janus_pyth_price_resolver");
    pub const MARKET_FACTORY: &str = sbf_path!("janus_market_factory");
}

// ---------------------------------------------------------------------------
// PDA derivation — mirrors the Rust `seeds` used inside each program.
// ---------------------------------------------------------------------------

pub mod pda {
    use super::*;
    use crate::ids;

    pub fn market(collateral: &Pubkey, resolver_state: &Pubkey, deadline_slot: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"market",
                collateral.as_ref(),
                resolver_state.as_ref(),
                &deadline_slot.to_le_bytes(),
            ],
            &ids::conditional_tokens(),
        )
    }

    pub fn yes_mint(market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"yes", market.as_ref()], &ids::conditional_tokens())
    }
    pub fn no_mint(market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"no", market.as_ref()], &ids::conditional_tokens())
    }
    pub fn vault(market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", market.as_ref()], &ids::conditional_tokens())
    }

    pub fn pool(market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"pool", market.as_ref()], &ids::lmsr_market())
    }
    pub fn pool_yes_vault(pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"yes-vault", pool.as_ref()], &ids::lmsr_market())
    }
    pub fn pool_no_vault(pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"no-vault", pool.as_ref()], &ids::lmsr_market())
    }

    pub fn slot_resolver_state(seed_key: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"slot-resolver", seed_key.as_ref()],
            &ids::slot_height_resolver(),
        )
    }

    pub fn registration(market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"registration", market.as_ref()], &ids::market_factory())
    }
}

// ---------------------------------------------------------------------------
// Token fixture helpers — build SPL Token mint and account states for tests.
// ---------------------------------------------------------------------------

pub mod token_fixtures {
    use solana_account::Account;
    use solana_program_option::COption;
    use solana_pubkey::Pubkey;
    use spl_token_interface::state::{Account as TokenAccount, AccountState, Mint};

    /// Construct an SPL `Mint` account owned by the SPL Token program.
    pub fn mint_account(authority: &Pubkey, decimals: u8, supply: u64) -> Account {
        let mint = Mint {
            mint_authority: COption::Some(*authority),
            supply,
            decimals,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        mollusk_svm_programs_token::token::create_account_for_mint(mint)
    }

    /// Construct an SPL `TokenAccount` holding `amount` of `mint` for `owner`.
    pub fn token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Account {
        let token_account = TokenAccount {
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
}

// ---------------------------------------------------------------------------
// Instruction builders — produce solana_sdk::instruction::Instruction values.
// ---------------------------------------------------------------------------

pub mod ix {
    use super::{ids, pda};
    use solana_instruction::{AccountMeta, Instruction};
    use solana_pubkey::Pubkey;

    fn token_program_id() -> Pubkey {
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .parse()
            .unwrap()
    }
    fn system_program_id() -> Pubkey {
        "11111111111111111111111111111111".parse().unwrap()
    }

    pub fn initialize_market(
        payer: Pubkey,
        authority: Pubkey,
        collateral_mint: Pubkey,
        resolver_program: Pubkey,
        resolver_state: Pubkey,
        deadline_slot: u64,
    ) -> (Instruction, Pubkey, Pubkey, Pubkey, Pubkey) {
        let (market, market_bump) = pda::market(&collateral_mint, &resolver_state, deadline_slot);
        let (yes_mint, yes_bump) = pda::yes_mint(&market);
        let (no_mint, no_bump) = pda::no_mint(&market);
        let (vault, vault_bump) = pda::vault(&market);

        let mut data = Vec::with_capacity(17);
        data.push(0u8); // InitializeMarket tag
        data.extend_from_slice(&deadline_slot.to_le_bytes());
        data.push(market_bump);
        data.push(yes_bump);
        data.push(no_bump);
        data.push(vault_bump);
        data.extend_from_slice(&[0u8; 4]); // padding

        let ix = Instruction {
            program_id: ids::conditional_tokens(),
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(market, false),
                AccountMeta::new_readonly(collateral_mint, false),
                AccountMeta::new(yes_mint, false),
                AccountMeta::new(no_mint, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(resolver_program, false),
                AccountMeta::new_readonly(resolver_state, false),
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new_readonly(token_program_id(), false),
                AccountMeta::new_readonly(system_program_id(), false),
            ],
            data,
        };
        (ix, market, yes_mint, no_mint, vault)
    }

    pub fn split(
        user: Pubkey,
        market: Pubkey,
        user_collateral: Pubkey,
        vault: Pubkey,
        yes_mint: Pubkey,
        no_mint: Pubkey,
        user_yes: Pubkey,
        user_no: Pubkey,
        amount: u64,
    ) -> Instruction {
        amount_ix(1, user, market, user_collateral, vault, yes_mint, no_mint, user_yes, user_no, amount)
    }

    pub fn merge(
        user: Pubkey,
        market: Pubkey,
        user_collateral: Pubkey,
        vault: Pubkey,
        yes_mint: Pubkey,
        no_mint: Pubkey,
        user_yes: Pubkey,
        user_no: Pubkey,
        amount: u64,
    ) -> Instruction {
        amount_ix(2, user, market, user_collateral, vault, yes_mint, no_mint, user_yes, user_no, amount)
    }

    #[allow(clippy::too_many_arguments)]
    fn amount_ix(
        tag: u8,
        user: Pubkey,
        market: Pubkey,
        user_collateral: Pubkey,
        vault: Pubkey,
        yes_mint: Pubkey,
        no_mint: Pubkey,
        user_yes: Pubkey,
        user_no: Pubkey,
        amount: u64,
    ) -> Instruction {
        let mut data = Vec::with_capacity(9);
        data.push(tag);
        data.extend_from_slice(&amount.to_le_bytes());
        Instruction {
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
            data,
        }
    }

    pub fn initialize_slot_resolver(
        payer: Pubkey,
        authority: Pubkey,
        seed_key: Pubkey,
        outcome: u8,
        target_slot: u64,
    ) -> (Instruction, Pubkey) {
        let (state, bump) = pda::slot_resolver_state(&seed_key);

        let mut data = Vec::with_capacity(49);
        data.push(1u8); // Initialize tag
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

    pub fn resolve_market(
        caller: Pubkey,
        market: Pubkey,
        resolver_program: Pubkey,
        resolver_state: Pubkey,
        extras: &[Pubkey],
    ) -> Instruction {
        let mut accounts = vec![
            AccountMeta::new_readonly(caller, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(resolver_program, false),
            AccountMeta::new_readonly(resolver_state, false),
        ];
        for e in extras {
            accounts.push(AccountMeta::new_readonly(*e, false));
        }
        Instruction {
            program_id: ids::conditional_tokens(),
            accounts,
            data: vec![4u8],
        }
    }

    pub fn redeem(
        user: Pubkey,
        market: Pubkey,
        user_collateral: Pubkey,
        vault: Pubkey,
        winning_mint: Pubkey,
        user_winning: Pubkey,
        amount: u64,
    ) -> Instruction {
        let mut data = Vec::with_capacity(9);
        data.push(3u8);
        data.extend_from_slice(&amount.to_le_bytes());
        Instruction {
            program_id: ids::conditional_tokens(),
            accounts: vec![
                AccountMeta::new_readonly(user, true),
                AccountMeta::new_readonly(market, false),
                AccountMeta::new(user_collateral, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(winning_mint, false),
                AccountMeta::new(user_winning, false),
                AccountMeta::new_readonly(token_program_id(), false),
            ],
            data,
        }
    }
}
