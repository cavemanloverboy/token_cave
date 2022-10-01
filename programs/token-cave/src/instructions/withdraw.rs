use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Mint};
use super::initialize::CaveInfo;
use crate::error::TokenCaveError;

pub fn handler(
    ctx: Context<Withdraw>,
) -> Result<()> {

    // Check that this is the depositor
    require_keys_eq!(
        ctx.accounts.cave_info.depositor,
        ctx.accounts.depositor.key(),
        TokenCaveError::Unauthorized,
    );
    
    // Check that the user has requested an unlock
    require!(
        ctx.accounts.cave_info.unlocking,
        TokenCaveError::DidNotRequestUnlock,
    );

    // Check that the timelock is up
    let earliest_withdraw_time = ctx.accounts.cave_info.unlock_request_time
        .checked_add(ctx.accounts.cave_info.timelock_duration.into())
        .unwrap();
    require_gt!(
        Clock::get()?.unix_timestamp,
        earliest_withdraw_time,
        TokenCaveError::LockIsActive,
    );


    // Withdraw spl token from the token cave
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.cave.to_account_info(),
                to: ctx.accounts.depositor_token_account.to_account_info(),
                authority: ctx.accounts.cave_info.to_account_info(),
            },
            &[&[&ctx.accounts.cave.key().to_bytes(), &[*ctx.bumps.get("cave_info").unwrap()]]]
        ),
        ctx.accounts.cave.amount,
    )?;

    anchor_spl::token::close_account(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.cave.to_account_info(),
                destination: ctx.accounts.depositor_token_account.to_account_info(),
                authority: ctx.accounts.cave_info.to_account_info(),
            },
            &[&[&ctx.accounts.cave.key().to_bytes(), &[*ctx.bumps.get("cave_info").unwrap()]]]
        ),
    )?;


    Ok(())
}


#[derive(Accounts)]
pub struct Withdraw<'info> {

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
    pub depositor: AccountInfo<'info>,

    /// NOTE: this has no additional checks because the spl transfer
    /// instruction requires `depositor` to have authority over funds
    /// inside of this token account.
    #[account(mut)]
    pub depositor_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

}