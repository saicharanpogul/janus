use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{create_program_address, Pubkey},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{
    instructions::{InitializeAccount3, Transfer},
    state::TokenAccount,
};

use crate::{
    error::LmsrError,
    instruction::{InitializePoolData, SwapData},
    state::Pool,
};

const MAX_FEE_BPS: u16 = 1_000;

const TOKEN_PROGRAM_ID: Pubkey =
    pinocchio_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// -------------------------------------------------------------------------
// InitializePool
// -------------------------------------------------------------------------

/// Accounts:
///   0. `[signer, writable]` payer / creator (also pulls subsidy from)
///   1. `[writable]`         pool PDA (will be created)
///   2. `[readonly]`         conditional-tokens market the pool trades on
///   3. `[readonly]`         YES mint
///   4. `[readonly]`         NO mint
///   5. `[writable]`         YES vault PDA (will be created; token account)
///   6. `[writable]`         NO vault PDA (will be created; token account)
///   7. `[writable]`         creator's YES token account (subsidy source)
///   8. `[writable]`         creator's NO token account (subsidy source)
///   9. `[readonly]`         SPL Token program
///  10. `[readonly]`         System program
pub fn process_initialize_pool(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = InitializePoolData::from_bytes(data)?;
    if ix.subsidy_yes == 0 || ix.subsidy_no == 0 {
        return Err(LmsrError::ZeroAmount.into());
    }
    if ix.fee_bps > MAX_FEE_BPS {
        return Err(LmsrError::InvalidFee.into());
    }

    let [
        payer,
        pool_ai,
        market_ai,
        yes_mint_ai,
        no_mint_ai,
        yes_vault_ai,
        no_vault_ai,
        creator_yes_ai,
        creator_no_ai,
        _token_program_ai,
        _system_program_ai,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !pool_ai.data_is_empty() {
        return Err(LmsrError::PoolAlreadyInitialized.into());
    }

    // Verify pool PDA: ["pool", market, bump]
    let pool_bump_arr = [ix.pool_bump];
    let pool_seeds: [&[u8]; 3] = [Pool::SEED, market_ai.key().as_ref(), pool_bump_arr.as_ref()];
    let derived_pool =
        create_program_address(&pool_seeds, &crate::ID).map_err(|_| LmsrError::InvalidPda)?;
    if &derived_pool != pool_ai.key() {
        return Err(LmsrError::InvalidPda.into());
    }

    // Vault PDAs derive from pool.
    let yes_vault_bump_arr = [ix.yes_vault_bump];
    let yes_vault_seeds: [&[u8]; 3] = [
        Pool::YES_VAULT_SEED,
        pool_ai.key().as_ref(),
        yes_vault_bump_arr.as_ref(),
    ];
    let derived_yes_vault = create_program_address(&yes_vault_seeds, &crate::ID)
        .map_err(|_| LmsrError::InvalidPda)?;
    if &derived_yes_vault != yes_vault_ai.key() {
        return Err(LmsrError::InvalidPda.into());
    }
    let no_vault_bump_arr = [ix.no_vault_bump];
    let no_vault_seeds: [&[u8]; 3] = [
        Pool::NO_VAULT_SEED,
        pool_ai.key().as_ref(),
        no_vault_bump_arr.as_ref(),
    ];
    let derived_no_vault =
        create_program_address(&no_vault_seeds, &crate::ID).map_err(|_| LmsrError::InvalidPda)?;
    if &derived_no_vault != no_vault_ai.key() {
        return Err(LmsrError::InvalidPda.into());
    }

    // ---- Create pool account ----
    let pool_lamports = (Pool::LEN as u64 + 128) * 3_480 * 2;
    let pool_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::SEED),
        Seed::from(market_ai.key().as_ref()),
        Seed::from(pool_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: pool_ai,
        lamports: pool_lamports,
        space: Pool::LEN as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[Signer::from(&pool_signer_seeds[..])])?;

    // ---- Create + initialize YES vault ----
    let yes_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::YES_VAULT_SEED),
        Seed::from(pool_ai.key().as_ref()),
        Seed::from(yes_vault_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: yes_vault_ai,
        lamports: (TokenAccount::LEN as u64 + 128) * 3_480 * 2,
        space: TokenAccount::LEN as u64,
        owner: &TOKEN_PROGRAM_ID,
    }
    .invoke_signed(&[Signer::from(&yes_signer_seeds[..])])?;
    InitializeAccount3 {
        account: yes_vault_ai,
        mint: yes_mint_ai,
        owner: pool_ai.key(),
    }
    .invoke()?;

    // ---- Create + initialize NO vault ----
    let no_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::NO_VAULT_SEED),
        Seed::from(pool_ai.key().as_ref()),
        Seed::from(no_vault_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: no_vault_ai,
        lamports: (TokenAccount::LEN as u64 + 128) * 3_480 * 2,
        space: TokenAccount::LEN as u64,
        owner: &TOKEN_PROGRAM_ID,
    }
    .invoke_signed(&[Signer::from(&no_signer_seeds[..])])?;
    InitializeAccount3 {
        account: no_vault_ai,
        mint: no_mint_ai,
        owner: pool_ai.key(),
    }
    .invoke()?;

    // ---- Pull subsidy from creator into vaults ----
    Transfer {
        from: creator_yes_ai,
        to: yes_vault_ai,
        authority: payer,
        amount: ix.subsidy_yes,
    }
    .invoke()?;
    Transfer {
        from: creator_no_ai,
        to: no_vault_ai,
        authority: payer,
        amount: ix.subsidy_no,
    }
    .invoke()?;

    // ---- Write pool state ----
    {
        let mut d = pool_ai.try_borrow_mut_data()?;
        let p = Pool::from_data_mut(&mut d)?;
        p.bump = ix.pool_bump;
        p._padding = [0; 7];
        p.market = *market_ai.key();
        p.yes_mint = *yes_mint_ai.key();
        p.no_mint = *no_mint_ai.key();
        p.yes_vault = *yes_vault_ai.key();
        p.no_vault = *no_vault_ai.key();
        p.authority = *payer.key();
        p.yes_reserves = ix.subsidy_yes;
        p.no_reserves = ix.subsidy_no;
        p.fee_bps = ix.fee_bps;
        p._padding2 = [0; 6];
    }
    Ok(())
}

// -------------------------------------------------------------------------
// Swap
// -------------------------------------------------------------------------

/// Accounts:
///   0. `[signer]`   user
///   1. `[writable]` pool
///   2. `[writable]` YES vault
///   3. `[writable]` NO vault
///   4. `[writable]` user's input-side token account
///   5. `[writable]` user's output-side token account
///   6. `[readonly]` SPL Token program
pub fn process_swap(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = SwapData::from_bytes(data)?;
    if ix.amount_in == 0 {
        return Err(LmsrError::ZeroAmount.into());
    }

    let [
        user,
        pool_ai,
        yes_vault_ai,
        no_vault_ai,
        user_in_ai,
        user_out_ai,
        _token_program_ai,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !user.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Read + validate pool.
    let pool = read_pool(pool_ai)?;
    if &pool.yes_vault != yes_vault_ai.key() || &pool.no_vault != no_vault_ai.key() {
        return Err(LmsrError::InvalidVault.into());
    }

    // direction: 0 = YES in, NO out; 1 = NO in, YES out.
    let (in_reserves, out_reserves, in_vault, out_vault) = match ix.direction {
        0 => (pool.yes_reserves, pool.no_reserves, yes_vault_ai, no_vault_ai),
        1 => (pool.no_reserves, pool.yes_reserves, no_vault_ai, yes_vault_ai),
        _ => return Err(LmsrError::InvalidInstructionData.into()),
    };

    // CPMM: amount_out = out_r * amount_in_after_fee / (in_r + amount_in_after_fee)
    let amount_in_u128 = ix.amount_in as u128;
    let fee_bps = pool.fee_bps as u128;
    let amount_in_after_fee = amount_in_u128
        .checked_mul(10_000u128 - fee_bps)
        .ok_or(LmsrError::MathOverflow)?
        / 10_000u128;

    let numerator = (out_reserves as u128)
        .checked_mul(amount_in_after_fee)
        .ok_or(LmsrError::MathOverflow)?;
    let denominator = (in_reserves as u128)
        .checked_add(amount_in_after_fee)
        .ok_or(LmsrError::MathOverflow)?;
    if denominator == 0 {
        return Err(LmsrError::InsufficientLiquidity.into());
    }
    let amount_out_u128 = numerator / denominator;
    if amount_out_u128 >= out_reserves as u128 {
        // Drain protection — leave at least 1 token in the pool so the
        // invariant remains well-defined.
        return Err(LmsrError::InsufficientLiquidity.into());
    }
    let amount_out = amount_out_u128 as u64;
    if amount_out < ix.min_amount_out {
        return Err(LmsrError::SlippageExceeded.into());
    }

    // ---- Transfer input from user to in_vault ----
    Transfer {
        from: user_in_ai,
        to: in_vault,
        authority: user,
        amount: ix.amount_in,
    }
    .invoke()?;

    // ---- Transfer output from out_vault to user, signed by pool ----
    let pool_bump_arr = [pool.bump];
    let pool_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::SEED),
        Seed::from(pool.market.as_ref()),
        Seed::from(pool_bump_arr.as_ref()),
    ];
    Transfer {
        from: out_vault,
        to: user_out_ai,
        authority: pool_ai,
        amount: amount_out,
    }
    .invoke_signed(&[Signer::from(&pool_signer_seeds[..])])?;

    // ---- Update reserves ----
    {
        let mut d = pool_ai.try_borrow_mut_data()?;
        let p = Pool::from_data_mut(&mut d)?;
        match ix.direction {
            0 => {
                p.yes_reserves = p
                    .yes_reserves
                    .checked_add(ix.amount_in)
                    .ok_or(LmsrError::MathOverflow)?;
                p.no_reserves = p
                    .no_reserves
                    .checked_sub(amount_out)
                    .ok_or(LmsrError::MathOverflow)?;
            }
            1 => {
                p.no_reserves = p
                    .no_reserves
                    .checked_add(ix.amount_in)
                    .ok_or(LmsrError::MathOverflow)?;
                p.yes_reserves = p
                    .yes_reserves
                    .checked_sub(amount_out)
                    .ok_or(LmsrError::MathOverflow)?;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

#[inline(always)]
fn read_pool(ai: &AccountInfo) -> Result<Pool, ProgramError> {
    // SAFETY: see note in conditional-tokens::processor::read_market.
    let owner = unsafe { ai.owner() };
    if owner != &crate::ID {
        return Err(LmsrError::InvalidAccountData.into());
    }
    let d = ai.try_borrow_data()?;
    Ok(*Pool::from_data(&d)?)
}
