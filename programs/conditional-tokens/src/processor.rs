use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{create_program_address, Pubkey},
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{
    instructions::{Burn, InitializeAccount3, InitializeMint2, MintTo, Transfer},
    state::{Mint, TokenAccount},
};

use crate::{
    error::ConditionalTokensError,
    instruction::{AmountData, InitializeMarketData},
    state::{Market, MarketStatus},
};

const TOKEN_PROGRAM_ID: Pubkey = pinocchio_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// -------------------------------------------------------------------------
// InitializeMarket
// -------------------------------------------------------------------------

/// Accounts (in order):
///   0. `[signer, writable]` payer (funds the new accounts)
///   1. `[writable]`         market PDA (will be created)
///   2. `[readonly]`         collateral mint
///   3. `[writable]`         yes mint PDA (will be created)
///   4. `[writable]`         no mint PDA (will be created)
///   5. `[writable]`         vault PDA (will be created; token account)
///   6. `[readonly]`         resolver program
///   7. `[readonly]`         resolver state account
///   8. `[signer]`           market authority
///   9. `[readonly]`         SPL Token program
///  10. `[readonly]`         System program
pub fn process_initialize_market(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = InitializeMarketData::from_bytes(data)?;

    let [
        payer,
        market_ai,
        collateral_mint_ai,
        yes_mint_ai,
        no_mint_ai,
        vault_ai,
        resolver_program_ai,
        resolver_state_ai,
        authority_ai,
        _token_program_ai,
        _system_program_ai,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !authority_ai.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !market_ai.data_is_empty() {
        return Err(ConditionalTokensError::MarketAlreadyInitialized.into());
    }

    let deadline_slot_le = ix.deadline_slot.to_le_bytes();
    let market_bump_arr = [ix.market_bump];
    let yes_bump_arr = [ix.yes_mint_bump];
    let no_bump_arr = [ix.no_mint_bump];
    let vault_bump_arr = [ix.vault_bump];

    // Verify deadline is in the future.
    let clock = Clock::get()?;
    if ix.deadline_slot <= clock.slot {
        return Err(ConditionalTokensError::DeadlineInPast.into());
    }

    // Verify market PDA: ["market", collateral_mint, resolver_state, deadline_slot_le, bump]
    let market_seeds: [&[u8]; 5] = [
        Market::SEED,
        collateral_mint_ai.key().as_ref(),
        resolver_state_ai.key().as_ref(),
        deadline_slot_le.as_ref(),
        market_bump_arr.as_ref(),
    ];
    let derived_market = create_program_address(&market_seeds, &crate::ID)
        .map_err(|_| ConditionalTokensError::InvalidPda)?;
    if &derived_market != market_ai.key() {
        return Err(ConditionalTokensError::InvalidPda.into());
    }

    // Derive child PDAs from the market key.
    let yes_seeds: [&[u8]; 3] = [
        Market::YES_MINT_SEED,
        market_ai.key().as_ref(),
        yes_bump_arr.as_ref(),
    ];
    let derived_yes = create_program_address(&yes_seeds, &crate::ID)
        .map_err(|_| ConditionalTokensError::InvalidPda)?;
    if &derived_yes != yes_mint_ai.key() {
        return Err(ConditionalTokensError::InvalidPda.into());
    }
    let no_seeds: [&[u8]; 3] = [
        Market::NO_MINT_SEED,
        market_ai.key().as_ref(),
        no_bump_arr.as_ref(),
    ];
    let derived_no = create_program_address(&no_seeds, &crate::ID)
        .map_err(|_| ConditionalTokensError::InvalidPda)?;
    if &derived_no != no_mint_ai.key() {
        return Err(ConditionalTokensError::InvalidPda.into());
    }
    let vault_seeds: [&[u8]; 3] = [
        Market::VAULT_SEED,
        market_ai.key().as_ref(),
        vault_bump_arr.as_ref(),
    ];
    let derived_vault = create_program_address(&vault_seeds, &crate::ID)
        .map_err(|_| ConditionalTokensError::InvalidPda)?;
    if &derived_vault != vault_ai.key() {
        return Err(ConditionalTokensError::InvalidPda.into());
    }

    // ---- Create the market account ----
    let market_lamports = minimum_balance(Market::LEN);
    let market_signer_seeds: [Seed; 5] = [
        Seed::from(Market::SEED),
        Seed::from(collateral_mint_ai.key().as_ref()),
        Seed::from(resolver_state_ai.key().as_ref()),
        Seed::from(deadline_slot_le.as_ref()),
        Seed::from(market_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: market_ai,
        lamports: market_lamports,
        space: Market::LEN as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[Signer::from(&market_signer_seeds[..])])?;

    // ---- Create + initialize YES mint ----
    let yes_signer_seeds: [Seed; 3] = [
        Seed::from(Market::YES_MINT_SEED),
        Seed::from(market_ai.key().as_ref()),
        Seed::from(yes_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: yes_mint_ai,
        lamports: minimum_balance(Mint::LEN),
        space: Mint::LEN as u64,
        owner: &TOKEN_PROGRAM_ID,
    }
    .invoke_signed(&[Signer::from(&yes_signer_seeds[..])])?;
    InitializeMint2 {
        mint: yes_mint_ai,
        decimals: 6,
        mint_authority: market_ai.key(),
        freeze_authority: None,
    }
    .invoke()?;

    // ---- Create + initialize NO mint ----
    let no_signer_seeds: [Seed; 3] = [
        Seed::from(Market::NO_MINT_SEED),
        Seed::from(market_ai.key().as_ref()),
        Seed::from(no_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: no_mint_ai,
        lamports: minimum_balance(Mint::LEN),
        space: Mint::LEN as u64,
        owner: &TOKEN_PROGRAM_ID,
    }
    .invoke_signed(&[Signer::from(&no_signer_seeds[..])])?;
    InitializeMint2 {
        mint: no_mint_ai,
        decimals: 6,
        mint_authority: market_ai.key(),
        freeze_authority: None,
    }
    .invoke()?;

    // ---- Create + initialize the collateral vault token account ----
    let vault_signer_seeds: [Seed; 3] = [
        Seed::from(Market::VAULT_SEED),
        Seed::from(market_ai.key().as_ref()),
        Seed::from(vault_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: vault_ai,
        lamports: minimum_balance(TokenAccount::LEN),
        space: TokenAccount::LEN as u64,
        owner: &TOKEN_PROGRAM_ID,
    }
    .invoke_signed(&[Signer::from(&vault_signer_seeds[..])])?;
    InitializeAccount3 {
        account: vault_ai,
        mint: collateral_mint_ai,
        owner: market_ai.key(),
    }
    .invoke()?;

    // ---- Write market state ----
    {
        let mut data_ref = market_ai.try_borrow_mut_data()?;
        let market = Market::from_account_data_mut(&mut data_ref)?;
        market.bump = ix.market_bump;
        market.status = MarketStatus::Active as u8;
        market._padding = [0; 6];
        market.collateral_mint = *collateral_mint_ai.key();
        market.yes_mint = *yes_mint_ai.key();
        market.no_mint = *no_mint_ai.key();
        market.vault = *vault_ai.key();
        market.resolver_program = *resolver_program_ai.key();
        market.resolver_state = *resolver_state_ai.key();
        market.authority = *authority_ai.key();
        market.deadline_slot = ix.deadline_slot;
        market.created_at_slot = clock.slot;
    }

    Ok(())
}

// -------------------------------------------------------------------------
// Split
// -------------------------------------------------------------------------

/// Accounts:
///   0. `[signer]`   user
///   1. `[readonly]` market
///   2. `[writable]` user collateral token account
///   3. `[writable]` vault token account
///   4. `[writable]` yes mint
///   5. `[writable]` no mint
///   6. `[writable]` user yes token account
///   7. `[writable]` user no token account
///   8. `[readonly]` SPL Token program
pub fn process_split(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = AmountData::from_bytes(data)?;
    if ix.amount == 0 {
        return Err(ConditionalTokensError::InvalidInstructionData.into());
    }

    let [
        user,
        market_ai,
        user_collateral_ai,
        vault_ai,
        yes_mint_ai,
        no_mint_ai,
        user_yes_ai,
        user_no_ai,
        _token_program_ai,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !user.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market = read_market(market_ai)?;
    if market.status != MarketStatus::Active as u8 {
        return Err(ConditionalTokensError::MarketAlreadyResolved.into());
    }
    if &market.vault != vault_ai.key()
        || &market.yes_mint != yes_mint_ai.key()
        || &market.no_mint != no_mint_ai.key()
    {
        return Err(ConditionalTokensError::InvalidAccountData.into());
    }

    // Pull collateral into vault.
    Transfer {
        from: user_collateral_ai,
        to: vault_ai,
        authority: user,
        amount: ix.amount,
    }
    .invoke()?;

    // Mint matching YES + NO to user, signed by the market PDA.
    let bump_arr = [market.bump];
    let deadline_slot_le = market.deadline_slot.to_le_bytes();
    let market_signer_seeds: [Seed; 5] = [
        Seed::from(Market::SEED),
        Seed::from(market.collateral_mint.as_ref()),
        Seed::from(market.resolver_state.as_ref()),
        Seed::from(deadline_slot_le.as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    let signer = Signer::from(&market_signer_seeds[..]);

    MintTo {
        mint: yes_mint_ai,
        account: user_yes_ai,
        mint_authority: market_ai,
        amount: ix.amount,
    }
    .invoke_signed(&[signer.clone()])?;

    MintTo {
        mint: no_mint_ai,
        account: user_no_ai,
        mint_authority: market_ai,
        amount: ix.amount,
    }
    .invoke_signed(&[signer])?;

    Ok(())
}

// -------------------------------------------------------------------------
// Merge
// -------------------------------------------------------------------------

/// Accounts:
///   0. `[signer]`   user
///   1. `[readonly]` market
///   2. `[writable]` user collateral token account
///   3. `[writable]` vault token account
///   4. `[writable]` yes mint
///   5. `[writable]` no mint
///   6. `[writable]` user yes token account
///   7. `[writable]` user no token account
///   8. `[readonly]` SPL Token program
pub fn process_merge(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = AmountData::from_bytes(data)?;
    if ix.amount == 0 {
        return Err(ConditionalTokensError::InvalidInstructionData.into());
    }

    let [
        user,
        market_ai,
        user_collateral_ai,
        vault_ai,
        yes_mint_ai,
        no_mint_ai,
        user_yes_ai,
        user_no_ai,
        _token_program_ai,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !user.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market = read_market(market_ai)?;
    let status = MarketStatus::try_from(market.status)?;
    // Allow merging while Active (regular unwind) or after INVALID resolution
    // (so users can recover their collateral 1:1 when the resolver couldn't
    // determine an outcome).
    if status != MarketStatus::Active && status != MarketStatus::ResolvedInvalid {
        return Err(ConditionalTokensError::MarketAlreadyResolved.into());
    }
    if &market.vault != vault_ai.key()
        || &market.yes_mint != yes_mint_ai.key()
        || &market.no_mint != no_mint_ai.key()
    {
        return Err(ConditionalTokensError::InvalidAccountData.into());
    }

    // Burn matching YES + NO from the user.
    Burn {
        account: user_yes_ai,
        mint: yes_mint_ai,
        authority: user,
        amount: ix.amount,
    }
    .invoke()?;
    Burn {
        account: user_no_ai,
        mint: no_mint_ai,
        authority: user,
        amount: ix.amount,
    }
    .invoke()?;

    // Return collateral from vault, signed by the market PDA.
    let bump_arr = [market.bump];
    let deadline_slot_le = market.deadline_slot.to_le_bytes();
    let signer_seeds: [Seed; 5] = [
        Seed::from(Market::SEED),
        Seed::from(market.collateral_mint.as_ref()),
        Seed::from(market.resolver_state.as_ref()),
        Seed::from(deadline_slot_le.as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    Transfer {
        from: vault_ai,
        to: user_collateral_ai,
        authority: market_ai,
        amount: ix.amount,
    }
    .invoke_signed(&[Signer::from(&signer_seeds[..])])?;

    Ok(())
}

// -------------------------------------------------------------------------
// Redeem
// -------------------------------------------------------------------------

/// Accounts:
///   0. `[signer]`   user
///   1. `[readonly]` market
///   2. `[writable]` user collateral token account
///   3. `[writable]` vault token account
///   4. `[writable]` winning outcome mint (yes_mint or no_mint, must match
///                   the market's resolved outcome)
///   5. `[writable]` user winning outcome token account
///   6. `[readonly]` SPL Token program
pub fn process_redeem(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = AmountData::from_bytes(data)?;
    if ix.amount == 0 {
        return Err(ConditionalTokensError::InvalidInstructionData.into());
    }

    let [
        user,
        market_ai,
        user_collateral_ai,
        vault_ai,
        winning_mint_ai,
        user_winning_ai,
        _token_program_ai,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !user.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market = read_market(market_ai)?;
    let status = MarketStatus::try_from(market.status)?;

    let expected_winning_mint = match status {
        MarketStatus::ResolvedYes => &market.yes_mint,
        MarketStatus::ResolvedNo => &market.no_mint,
        MarketStatus::ResolvedInvalid => return Err(ConditionalTokensError::MarketResolvedInvalid.into()),
        MarketStatus::Active => return Err(ConditionalTokensError::MarketNotResolved.into()),
    };
    if expected_winning_mint != winning_mint_ai.key() {
        return Err(ConditionalTokensError::InvalidMint.into());
    }
    if &market.vault != vault_ai.key() {
        return Err(ConditionalTokensError::InvalidVault.into());
    }

    // Burn the user's winning tokens.
    Burn {
        account: user_winning_ai,
        mint: winning_mint_ai,
        authority: user,
        amount: ix.amount,
    }
    .invoke()?;

    // Transfer collateral 1:1 from the vault.
    let bump_arr = [market.bump];
    let deadline_slot_le = market.deadline_slot.to_le_bytes();
    let signer_seeds: [Seed; 5] = [
        Seed::from(Market::SEED),
        Seed::from(market.collateral_mint.as_ref()),
        Seed::from(market.resolver_state.as_ref()),
        Seed::from(deadline_slot_le.as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    Transfer {
        from: vault_ai,
        to: user_collateral_ai,
        authority: market_ai,
        amount: ix.amount,
    }
    .invoke_signed(&[Signer::from(&signer_seeds[..])])?;

    Ok(())
}

// -------------------------------------------------------------------------
// Resolve
// -------------------------------------------------------------------------

/// Accounts:
///   0. `[signer]`   caller (anyone may call once the deadline has passed)
///   1. `[writable]` market
///   2. `[readonly]` resolver program (must match `market.resolver_program`)
///   3. `[readonly]` resolver state account (must match `market.resolver_state`)
///   4+. additional accounts forwarded to the resolver as-is
///
/// The resolver is expected to expose a single instruction returning a
/// 1-byte outcome via `set_return_data`:
///   - `1` = YES wins
///   - `2` = NO wins
///   - `3` = INVALID
///   - `0` = UNRESOLVED (resolver does not yet have a determination)
///
/// TODO: the actual CPI to the resolver is wired in once the resolver
/// registry program lands; for now we only validate accounts and the
/// deadline so users get the right error shape during development.
pub fn process_resolve(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let [caller, market_ai, resolver_program_ai, resolver_state_ai, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !caller.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market = read_market(market_ai)?;
    if market.status != MarketStatus::Active as u8 {
        return Err(ConditionalTokensError::MarketAlreadyResolved.into());
    }
    if &market.resolver_program != resolver_program_ai.key()
        || &market.resolver_state != resolver_state_ai.key()
    {
        return Err(ConditionalTokensError::InvalidResolver.into());
    }

    let clock = Clock::get()?;
    if clock.slot < market.deadline_slot {
        return Err(ConditionalTokensError::DeadlineNotReached.into());
    }

    // TODO(resolver-registry): CPI to resolver and decode return data.
    // Until the resolver program ships, this instruction stops here and
    // returns a clear "not implemented" custom error so tests can pin the
    // contract surface without depending on resolver code yet.
    Err(ConditionalTokensError::InvalidResolver.into())
}

// -------------------------------------------------------------------------
// helpers
// -------------------------------------------------------------------------

#[inline(always)]
fn read_market(market_ai: &AccountInfo) -> Result<Market, ProgramError> {
    // SAFETY: `AccountInfo::owner` is only undefined if the runtime hands
    // the program a mis-aligned account header; the Solana loader never
    // does this, so this dereference is safe in practice.
    let owner = unsafe { market_ai.owner() };
    if owner != &crate::ID {
        return Err(ConditionalTokensError::InvalidAccountData.into());
    }
    let data = market_ai.try_borrow_data()?;
    let m = Market::from_account_data(&data)?;
    Ok(*m)
}

/// Conservative rent-exempt minimum balance approximation used in account
/// creation CPIs. The exact value is queryable from the rent sysvar; using
/// a static estimate keeps the CPI cheap and predictable for the markets
/// we deploy. Equal to the runtime's rent calculation for accounts of the
/// given size at the canonical lamports-per-byte-year of 3480.
#[inline(always)]
fn minimum_balance(space: usize) -> u64 {
    // (DATA_SIZE + 128) * 3_480 * 2 years, expressed in lamports.
    // The +128 covers the account-header overhead the runtime accounts for.
    ((space as u64) + 128) * 3_480 * 2
}
