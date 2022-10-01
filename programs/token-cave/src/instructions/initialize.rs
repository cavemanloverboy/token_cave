use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Mint};

use crate::error::TokenCaveError;

pub const MAX_LOCK_DURATION: u32 = 7 * 24 * 60 * 60;
pub const CAVE_INFO_SIZE: usize = 86;


pub fn handler(
    ctx: Context<Initialize>,
    deposit_amount: u64,
    backup_address: Option<Pubkey>,
    timelock_duration: u32,
) -> Result<()> {

    // Check lock duration is under max lock duration
    require_gte!(
        MAX_LOCK_DURATION,
        timelock_duration,
        TokenCaveError::DurationExceedsMaximum
    );

    // Store backup address, timelock duration, and initialize util vars
    ctx.accounts.cave_info.backup_address = backup_address;
    ctx.accounts.cave_info.timelock_duration = timelock_duration;
    ctx.accounts.cave_info.depositor = ctx.accounts.depositor.key();
    ctx.accounts.cave_info.unlock_request_time = i64::MIN;
    ctx.accounts.cave_info.unlocking = false;

    // Store spl token in the token cave
    anchor_spl::token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.depositor_token_account.to_account_info(),
                to: ctx.accounts.cave.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        deposit_amount,
    )?;

    Ok(())
}


#[derive(Accounts)]
pub struct Initialize<'info> {

    /// The token cave! A program-owned spl token account
    /// which supports deposits and time-locked withdraws.
    /// The time-locked withdraw can be aborted, which sends
    /// the tokens to the specified backup address
    #[account(
        init,
        payer = depositor,
        seeds = [&depositor_token_account.key().to_bytes()],
        bump,
        token::mint = mint,
        token::authority = cave_info,
    )]
    pub cave: Account<'info, TokenAccount>,

    /// This PDA stores the information about the associated cave
    #[account(
        init,
        payer = depositor,
        seeds = [&cave.key().to_bytes()],
        space = CAVE_INFO_SIZE,
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

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct CaveInfo {

    /// Backup address in case things go south
    pub backup_address: Option<Pubkey>,

    /// Depositor
    pub depositor: Pubkey,

    /// Timelock duration
    pub timelock_duration: u32,

    /// Time of unlock request.
    pub unlock_request_time: i64,

    /// Flag whether user is unlocking: bool,
    pub unlocking: bool,
}

impl CaveInfo {

    pub fn is_backup<'info>(
        &self,
        account: &AccountInfo<'info>
    ) -> bool {
        if let Some(backup) = self.backup_address {
            backup == account.key()
        } else {
            false
        }
    }
}