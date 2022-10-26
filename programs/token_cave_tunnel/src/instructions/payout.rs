use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Mint};
use super::payment::CaveTunnelInfo;
use crate::{error::TokenCaveError, instructions::payment::TIMELOCK_DURATION};

pub fn handler(
    ctx: Context<Payout>,
) -> Result<()> {

    // Check that the timelock is up
    let earliest_withdraw_time = ctx.accounts.cave_tunnel_info.payment_time
        .checked_add(TIMELOCK_DURATION.try_into().expect("small usize to i64"))
        .expect("integer overflow on time addition");
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
                from: ctx.accounts.cave_tunnel.to_account_info(),
                to: ctx.accounts.payee_token_account.to_account_info(),
                authority: ctx.accounts.cave_tunnel_info.to_account_info(),
            },
            &[&[&ctx.accounts.cave_tunnel.key().to_bytes(), &[*ctx.bumps.get("cave_tunnel_info").unwrap()]]]
        ),
        ctx.accounts.cave_tunnel.amount,
    )?;

    anchor_spl::token::close_account(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.cave_tunnel.to_account_info(),
                destination: ctx.accounts.payee_token_account.to_account_info(),
                authority: ctx.accounts.cave_tunnel_info.to_account_info(),
            },
            &[&[&ctx.accounts.cave_tunnel.key().to_bytes(), &[*ctx.bumps.get("cave_tunnel_info").unwrap()]]]
        ),
    )?;


    Ok(())
}


#[derive(Accounts)]
#[instruction(payee_pubkey: Pubkey)]
pub struct Payout<'info> {

    /// The token cave tunnel! A program-owned spl token account
    /// which supports payment with a time-locked payout.
    #[account(
        mut,
        seeds = [&cave_tunnel_info.payer.to_bytes()],
        bump,
        token::mint = mint,
        token::authority = cave_tunnel_info,
    )]
    pub cave_tunnel: Account<'info, TokenAccount>,

    /// This PDA stores the information about the associated cave tunnel
    #[account(
        mut,
        close = placeholder_for_threshold_signature,
        seeds = [&cave_tunnel.key().to_bytes()],
        bump,
    )]
    pub cave_tunnel_info: Account<'info, CaveTunnelInfo>,

    #[account()]
    pub mint: Account<'info, Mint>,
    
    /// NOTE: this would have to be checked but is not checked here
    #[account(mut)]
    pub placeholder_for_threshold_signature: Signer<'info>,

    #[account(
        mut,
        address = payee_pubkey,
    )]
    /// CHECK: this should be checked by the tss signers
    pub payee: AccountInfo<'info>,

    /// NOTE: this has no additional checks because the spl transfer
    /// instruction requires `depositor` to have authority over funds
    /// inside of this token account.
    #[account(
        mut,
        token::mint = mint,
        token::authority = payee,
    )]
    pub payee_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

}
