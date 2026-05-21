use janus_lmsr_math::{buy_no_cost, buy_yes_cost, Q32_32, LN2_Q};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{create_program_address, Pubkey},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{
    instructions::{Burn, InitializeAccount3, MintTo, Transfer},
    state::TokenAccount,
};

use crate::{
    error::LmsrTrueError,
    instruction::{BuyData, InitializePoolData, SellData, WithdrawResidualData},
    state::{Pool, PoolStatus},
};

const TOKEN_PROGRAM_ID: Pubkey =
    pinocchio_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// =========================================================================
// InitializePool
// =========================================================================

/// Accounts:
///   0. `[signer, writable]` payer / authority (also funds subsidy)
///   1. `[writable]`         pool PDA (will be created)
///   2. `[readonly]`         resolver program (stored as `resolver_program`)
///   3. `[readonly]`         resolver state (stored as `resolver_state`)
///   4. `[readonly]`         collateral mint
///   5. `[writable]`         collateral vault PDA (will be created)
///   6. `[readonly]`         YES mint (caller must pre-create with pool PDA as mint authority)
///   7. `[readonly]`         NO mint  (caller must pre-create with pool PDA as mint authority)
///   8. `[writable]`         payer's collateral token account (subsidy source)
///   9. `[readonly]`         SPL Token program
///  10. `[readonly]`         System program
pub fn process_initialize_pool(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = InitializePoolData::from_bytes(data)?;
    if ix.b == 0 {
        return Err(LmsrTrueError::InvalidLiquidityParameter.into());
    }
    if ix.b > u32::MAX as u64 {
        return Err(LmsrTrueError::InvalidLiquidityParameter.into());
    }

    // Minimum subsidy = ceil(b · ln(2)) in collateral units. We compute
    // in u128 fixed-point: `(b << 32) · LN2_Q >> 64` and round up.
    let min_subsidy = ceil_b_ln2(ix.b)?;
    if ix.initial_subsidy < min_subsidy {
        return Err(LmsrTrueError::InvalidLiquidityParameter.into());
    }

    let [
        payer,
        pool_ai,
        resolver_program_ai,
        resolver_state_ai,
        collateral_mint_ai,
        collateral_vault_ai,
        yes_mint_ai,
        no_mint_ai,
        payer_collateral_ai,
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
        return Err(LmsrTrueError::PoolAlreadyInitialized.into());
    }

    // Verify pool PDA: ["pool", resolver_state, bump].
    let pool_bump_arr = [ix.pool_bump];
    let pool_seeds: [&[u8]; 3] = [
        Pool::SEED,
        resolver_state_ai.key().as_ref(),
        pool_bump_arr.as_ref(),
    ];
    let derived_pool =
        create_program_address(&pool_seeds, &crate::ID).map_err(|_| LmsrTrueError::InvalidPda)?;
    if &derived_pool != pool_ai.key() {
        return Err(LmsrTrueError::InvalidPda.into());
    }

    // Verify collateral vault PDA: ["coll-vault", pool, bump].
    let vault_bump_arr = [ix.collateral_vault_bump];
    let vault_seeds: [&[u8]; 3] = [
        Pool::COLLATERAL_VAULT_SEED,
        pool_ai.key().as_ref(),
        vault_bump_arr.as_ref(),
    ];
    let derived_vault = create_program_address(&vault_seeds, &crate::ID)
        .map_err(|_| LmsrTrueError::InvalidPda)?;
    if &derived_vault != collateral_vault_ai.key() {
        return Err(LmsrTrueError::InvalidPda.into());
    }

    // ---- Create pool account ----
    let pool_lamports = (Pool::LEN as u64 + 128) * 3_480 * 2;
    let pool_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::SEED),
        Seed::from(resolver_state_ai.key().as_ref()),
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

    // ---- Create + initialize collateral vault ----
    let vault_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::COLLATERAL_VAULT_SEED),
        Seed::from(pool_ai.key().as_ref()),
        Seed::from(vault_bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: collateral_vault_ai,
        lamports: (TokenAccount::LEN as u64 + 128) * 3_480 * 2,
        space: TokenAccount::LEN as u64,
        owner: &TOKEN_PROGRAM_ID,
    }
    .invoke_signed(&[Signer::from(&vault_signer_seeds[..])])?;
    InitializeAccount3 {
        account: collateral_vault_ai,
        mint: collateral_mint_ai,
        owner: pool_ai.key(),
    }
    .invoke()?;

    // ---- Pull subsidy ----
    Transfer {
        from: payer_collateral_ai,
        to: collateral_vault_ai,
        authority: payer,
        amount: ix.initial_subsidy,
    }
    .invoke()?;

    // ---- Write pool state ----
    {
        let mut d = pool_ai.try_borrow_mut_data()?;
        let p = Pool::from_data_mut(&mut d)?;
        p.bump = ix.pool_bump;
        p.status = PoolStatus::Active as u8;
        p._padding = [0; 6];
        p.authority = *payer.key();
        p.resolver_program = *resolver_program_ai.key();
        p.resolver_state = *resolver_state_ai.key();
        p.collateral_mint = *collateral_mint_ai.key();
        p.collateral_vault = *collateral_vault_ai.key();
        p.yes_mint = *yes_mint_ai.key();
        p.no_mint = *no_mint_ai.key();
        p.b = ix.b;
        p.q_yes = 0;
        p.q_no = 0;
        p.initial_subsidy = ix.initial_subsidy;
    }
    Ok(())
}

// =========================================================================
// Buy
// =========================================================================

/// Accounts:
///   0. `[signer]`   user
///   1. `[writable]` pool
///   2. `[writable]` collateral vault
///   3. `[writable]` user's collateral token account
///   4. `[writable]` YES mint OR NO mint, depending on `side`
///   5. `[writable]` user's destination token account for outcome tokens
///   6. `[readonly]` SPL Token program
pub fn process_buy(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = BuyData::from_bytes(data)?;
    if ix.delta == 0 {
        return Err(LmsrTrueError::ZeroAmount.into());
    }

    let [user, pool_ai, vault_ai, user_coll_ai, mint_ai, user_out_ai, _token_program_ai] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !user.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let pool = read_pool(pool_ai)?;
    if pool.status != PoolStatus::Active as u8 {
        return Err(LmsrTrueError::PoolNotActive.into());
    }
    if &pool.collateral_vault != vault_ai.key() {
        return Err(LmsrTrueError::InvalidVault.into());
    }
    let expected_mint = match ix.side {
        0 => &pool.yes_mint,
        1 => &pool.no_mint,
        _ => return Err(LmsrTrueError::InvalidInstructionData.into()),
    };
    if expected_mint != mint_ai.key() {
        return Err(LmsrTrueError::InvalidMint.into());
    }

    // Compute cost via LMSR. All values must fit in u32 for from_int.
    let cost_units = compute_buy_cost(pool.b, pool.q_yes, pool.q_no, ix.delta, ix.side)?;
    if cost_units > ix.max_collateral_in {
        return Err(LmsrTrueError::SlippageExceeded.into());
    }

    // ---- Pull collateral from user ----
    Transfer {
        from: user_coll_ai,
        to: vault_ai,
        authority: user,
        amount: cost_units,
    }
    .invoke()?;

    // ---- Mint outcome tokens to user, signed by pool PDA ----
    let pool_bump_arr = [pool.bump];
    let pool_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::SEED),
        Seed::from(pool.resolver_state.as_ref()),
        Seed::from(pool_bump_arr.as_ref()),
    ];
    MintTo {
        mint: mint_ai,
        account: user_out_ai,
        mint_authority: pool_ai,
        amount: ix.delta,
    }
    .invoke_signed(&[Signer::from(&pool_signer_seeds[..])])?;

    // ---- Update reserves ----
    {
        let mut d = pool_ai.try_borrow_mut_data()?;
        let p = Pool::from_data_mut(&mut d)?;
        match ix.side {
            0 => {
                p.q_yes = p
                    .q_yes
                    .checked_add(ix.delta)
                    .ok_or(LmsrTrueError::MathOverflow)?;
            }
            1 => {
                p.q_no = p
                    .q_no
                    .checked_add(ix.delta)
                    .ok_or(LmsrTrueError::MathOverflow)?;
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

// =========================================================================
// Sell
// =========================================================================

/// Accounts:
///   0. `[signer]`   user
///   1. `[writable]` pool
///   2. `[writable]` collateral vault
///   3. `[writable]` user's collateral token account
///   4. `[writable]` YES mint OR NO mint, depending on `side`
///   5. `[writable]` user's source token account (burned from)
///   6. `[readonly]` SPL Token program
pub fn process_sell(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = SellData::from_bytes(data)?;
    if ix.delta == 0 {
        return Err(LmsrTrueError::ZeroAmount.into());
    }

    let [user, pool_ai, vault_ai, user_coll_ai, mint_ai, user_src_ai, _token_program_ai] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !user.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let pool = read_pool(pool_ai)?;
    if pool.status != PoolStatus::Active as u8 {
        return Err(LmsrTrueError::PoolNotActive.into());
    }
    if &pool.collateral_vault != vault_ai.key() {
        return Err(LmsrTrueError::InvalidVault.into());
    }
    let expected_mint = match ix.side {
        0 => &pool.yes_mint,
        1 => &pool.no_mint,
        _ => return Err(LmsrTrueError::InvalidInstructionData.into()),
    };
    if expected_mint != mint_ai.key() {
        return Err(LmsrTrueError::InvalidMint.into());
    }

    // Check the user isn't selling more than outstanding (would corrupt
    // q_yes / q_no into underflow).
    let (q_side, q_other) = match ix.side {
        0 => (pool.q_yes, pool.q_no),
        1 => (pool.q_no, pool.q_yes),
        _ => unreachable!(),
    };
    if ix.delta > q_side {
        return Err(LmsrTrueError::MathOverflow.into());
    }
    let new_q = q_side - ix.delta;

    // Sell payout = C(q_yes, q_no) - C(q_yes - delta, q_no).
    // Equivalent to: "negative buy_yes_cost from the new state".
    let payout = match ix.side {
        0 => compute_buy_cost(pool.b, new_q, q_other, ix.delta, 0)?,
        1 => compute_buy_cost(pool.b, q_other, new_q, ix.delta, 1)?,
        _ => unreachable!(),
    };
    if payout < ix.min_collateral_out {
        return Err(LmsrTrueError::SlippageExceeded.into());
    }

    // ---- Burn outcome tokens from user ----
    Burn {
        account: user_src_ai,
        mint: mint_ai,
        authority: user,
        amount: ix.delta,
    }
    .invoke()?;

    // ---- Transfer collateral out, signed by pool PDA ----
    let pool_bump_arr = [pool.bump];
    let pool_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::SEED),
        Seed::from(pool.resolver_state.as_ref()),
        Seed::from(pool_bump_arr.as_ref()),
    ];
    Transfer {
        from: vault_ai,
        to: user_coll_ai,
        authority: pool_ai,
        amount: payout,
    }
    .invoke_signed(&[Signer::from(&pool_signer_seeds[..])])?;

    // ---- Update reserves ----
    {
        let mut d = pool_ai.try_borrow_mut_data()?;
        let p = Pool::from_data_mut(&mut d)?;
        match ix.side {
            0 => p.q_yes = new_q,
            1 => p.q_no = new_q,
            _ => unreachable!(),
        }
    }
    Ok(())
}

// =========================================================================
// WithdrawResidual
// =========================================================================
//
// Sweep collateral left in the vault after resolution. Pre-MVP: requires
// status != Active. A more careful version would also gate on "after all
// winning-side claims have been redeemed" by tracking outstanding YES/NO
// supply. Leaving that as a follow-on; for now the authority is trusted
// not to withdraw before claims complete (a soft constraint — bounded
// loss still holds regardless).
pub fn process_withdraw_residual(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let ix = WithdrawResidualData::from_bytes(data)?;
    if ix.amount == 0 {
        return Err(LmsrTrueError::ZeroAmount.into());
    }
    let [authority, pool_ai, vault_ai, dest_ai, _token_program_ai] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let pool = read_pool(pool_ai)?;
    if &pool.authority != authority.key() {
        return Err(ProgramError::IllegalOwner);
    }
    if &pool.collateral_vault != vault_ai.key() {
        return Err(LmsrTrueError::InvalidVault.into());
    }
    if pool.status == PoolStatus::Active as u8 {
        return Err(LmsrTrueError::PoolStillActive.into());
    }

    let pool_bump_arr = [pool.bump];
    let pool_signer_seeds: [Seed; 3] = [
        Seed::from(Pool::SEED),
        Seed::from(pool.resolver_state.as_ref()),
        Seed::from(pool_bump_arr.as_ref()),
    ];
    Transfer {
        from: vault_ai,
        to: dest_ai,
        authority: pool_ai,
        amount: ix.amount,
    }
    .invoke_signed(&[Signer::from(&pool_signer_seeds[..])])?;
    Ok(())
}

// =========================================================================
// Helpers
// =========================================================================

#[inline(always)]
fn read_pool(ai: &AccountInfo) -> Result<Pool, ProgramError> {
    let owner = unsafe { ai.owner() };
    if owner != &crate::ID {
        return Err(LmsrTrueError::InvalidAccountData.into());
    }
    let d = ai.try_borrow_data()?;
    Ok(*Pool::from_data(&d)?)
}

/// Cost to buy `delta` shares of the given side, in collateral units.
///
/// All quantities must fit in u32 (the Q32.32 integer envelope). For
/// realistic markets with 6-decimal-place tokens this caps notional at
/// ~4000 units of collateral per side — adequate for tail / niche
/// markets but undersized for liquidity-deep markets. Wider precision
/// (Q56.8) is a follow-on if the demand surfaces.
fn compute_buy_cost(
    b: u64,
    q_yes: u64,
    q_no: u64,
    delta: u64,
    side: u8,
) -> Result<u64, ProgramError> {
    let b_q = q_from_u64(b)?;
    let qy_q = q_from_u64(q_yes)?;
    let qn_q = q_from_u64(q_no)?;
    let d_q = q_from_u64(delta)?;
    let cost_q = match side {
        0 => buy_yes_cost(b_q, qy_q, qn_q, d_q),
        1 => buy_no_cost(b_q, qy_q, qn_q, d_q),
        _ => return Err(LmsrTrueError::InvalidInstructionData.into()),
    }
    .ok_or(LmsrTrueError::MathOverflow)?;
    // Cost in collateral units = round(cost_q). Round up so the pool
    // never charges less than the true cost (favor the subsidizer).
    Ok(round_up_q(cost_q))
}

#[inline(always)]
fn q_from_u64(x: u64) -> Result<Q32_32, ProgramError> {
    if x > u32::MAX as u64 {
        return Err(LmsrTrueError::MathOverflow.into());
    }
    Ok(Q32_32::from_int(x as u32))
}

/// `ceil(x · 2^32)` → u64 integer part.
#[inline(always)]
fn round_up_q(x: Q32_32) -> u64 {
    let bits = x.0;
    let int_part = bits >> 32;
    // If any fractional bit is set, round up.
    if (bits & ((1u64 << 32) - 1)) != 0 {
        int_part + 1
    } else {
        int_part
    }
}

/// `ceil(b · ln(2))` in collateral units, given `b` as a u64 fitting in
/// u32. Computed in u128 fixed-point to avoid loss of precision:
///
///     b · LN2_Q ≈ b · ln(2) · 2^32.
fn ceil_b_ln2(b: u64) -> Result<u64, ProgramError> {
    let prod = (b as u128).checked_mul(LN2_Q as u128).ok_or(LmsrTrueError::MathOverflow)?;
    // ceil divide by 2^32.
    let div = prod >> 32;
    let rem = prod & ((1u128 << 32) - 1);
    let ceil = if rem == 0 { div } else { div + 1 };
    if ceil > u64::MAX as u128 {
        return Err(LmsrTrueError::MathOverflow.into());
    }
    Ok(ceil as u64)
}

