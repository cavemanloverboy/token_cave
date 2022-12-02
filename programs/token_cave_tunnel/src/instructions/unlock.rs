use crate::error::TokenCaveError;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

use super::initialize::CaveInfo;

pub fn handler(ctx: Context<Unlock>) -> Result<()> {
    // Check that this is the depositor
    require_keys_eq!(
        ctx.accounts.cave_info.depositor,
        ctx.accounts.depositor.key(),
        TokenCaveError::Unauthorized,
    );

    // Check unlock is not already active
    require_keys_eq!(
        ctx.accounts.cave_info.depositor,
        ctx.accounts.depositor.key(),
        TokenCaveError::UnlockAlreadyActive,
    );

    // Initialize unlock
    ctx.accounts.cave_info.unlock_request_time = Clock::get()?.unix_timestamp;
    ctx.accounts.cave_info.unlocking = true;

    Ok(())
}

#[derive(Accounts)]
pub struct Unlock<'info> {
    /// The token cave! A program-owned spl token account
    /// which supports deposits and time-locked withdraws.
    /// The time-locked withdraw can be aborted, which sends
    /// the tokens to the specified backup address
    #[account(
        seeds = [&depositor_token_account.key().to_bytes()],
        bump,
        token::mint = mint,
        token::authority = cave_info,
    )]
    pub cave: Account<'info, TokenAccount>,

    /// This PDA stores the information about the associated cave
    #[account(
        mut,
        seeds = [&cave.key().to_bytes()],
        bump,
    )]
    pub cave_info: Account<'info, CaveInfo>,

    #[account()]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    /// NOTE: this has no additional checks because the spl transfer
    /// instruction requires `depositor` to have authority over funds
    /// inside of this token account.
    #[account(mut)]
    pub depositor_token_account: Account<'info, TokenAccount>,
}
