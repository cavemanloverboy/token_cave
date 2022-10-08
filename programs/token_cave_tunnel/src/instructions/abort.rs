use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Mint};
use super::initialize::CaveInfo;
use crate::error::TokenCaveError;

pub fn handler(
    ctx: Context<Abort>,
) -> Result<()> {

    // Check that this is the depositor and the backup account
    require!(
        ctx.accounts.depositor.key() == ctx.accounts.cave_info.depositor
        && ctx.accounts.cave_info.is_backup(&ctx.accounts.backup),
        TokenCaveError::Unauthorized,
    );

    // Check that the token account belongs to the backup
    msg!(
        "owners {} {}",
        ctx.accounts.backup_spl_account.owner,
        ctx.accounts.backup.key(),
    );
    require_eq!(
        ctx.accounts.backup_spl_account.owner,
        ctx.accounts.backup.key(),
        TokenCaveError::IncorrectBackupTokenAccount,
    );

    // Check that the user has requested an unlock
    require!(
        ctx.accounts.cave_info.unlocking,
        TokenCaveError::DidNotRequestUnlock,
    );

    // Withdraw spl token from the token cave to backup spl
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.cave.to_account_info(),
                to: ctx.accounts.backup_spl_account.to_account_info(),
                authority: ctx.accounts.cave_info.to_account_info(),
            },
            &[&[ctx.accounts.cave.key().as_ref(), &[*ctx.bumps.get("cave_info").unwrap()]]]
        ),
        ctx.accounts.cave.amount,
    )?;

    anchor_spl::token::close_account(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.cave.to_account_info(),
                destination: ctx.accounts.backup_spl_account.to_account_info(),
                authority: ctx.accounts.cave_info.to_account_info(),
            },
            &[&[&ctx.accounts.cave.key().to_bytes(), &[*ctx.bumps.get("cave_info").unwrap()]]]
        ),
    )?;


    Ok(())
}


#[derive(Accounts)]
pub struct Abort<'info> {

    /// The token cave! A program-owned spl token account
    /// which supports deposits and time-locked withdraws.
    /// The time-locked withdraw can be aborted, which sends
    /// the tokens to the specified backup address
    #[account(
        mut,
        seeds = [&depositor_token_account.key().to_bytes()],
        bump,
        token::mint = mint,
        token::authority = cave_info,
    )]
    pub cave: Account<'info, TokenAccount>,

    /// This PDA stores the information about the associated cave
    #[account(
        mut,
        close = depositor,
        seeds = [&cave.key().to_bytes()],
        bump,
    )]
    pub cave_info: Account<'info, CaveInfo>,

    #[account()]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(mut)]
    pub backup: AccountInfo<'info>,

    #[account()]
    pub depositor_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub backup_spl_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

}