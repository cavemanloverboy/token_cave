use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Mint};

pub const CAVE_TUNNEL_INFO_SIZE: usize = 8 + core::mem::size_of::<CaveTunnelInfo>();

// Delay period before receiving payment
pub const TIMELOCK_DURATION: usize = 5;

pub const COST_OF_SERVICE_PER_SECOND: u64 = 1_000_000;

pub fn handler(
    ctx: Context<Payment>,
    service_time: u32,
) -> Result<()> {

    // Store payer, and payment time. Initialize payee to default
    ctx.accounts.cave_tunnel_info.payer = ctx.accounts.payer.key();
    ctx.accounts.cave_tunnel_info.payment_time = Clock::get()?.unix_timestamp;
    ctx.accounts.cave_tunnel_info.service_time = service_time;

    // Store spl token in the token cave
    anchor_spl::token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.payer_token_account.to_account_info(),
                to: ctx.accounts.cave_tunnel.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        ),
        service_time as u64 * COST_OF_SERVICE_PER_SECOND,
    )
}


#[derive(Accounts)]
pub struct Payment<'info> {

    /// The token cave tunnel! A program-owned spl token account
    /// which supports payment with a time-locked payout.
    #[account(
        init,
        payer = payer,
        seeds = [&payer.key().to_bytes()],
        bump,
        token::mint = mint,
        token::authority = cave_tunnel_info,
    )]
    pub cave_tunnel: Account<'info, TokenAccount>,

    /// This PDA stores the information about the associated cave
    #[account(
        init,
        payer = payer,
        seeds = [&cave_tunnel.key().to_bytes()],
        space = CAVE_TUNNEL_INFO_SIZE,
        bump,
    )]
    pub cave_tunnel_info: Account<'info, CaveTunnelInfo>,

    #[account()]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// NOTE: this has no additional checks because the spl transfer
    /// instruction requires `depositor` to have authority over funds
    /// inside of this token account.
    #[account(mut)]
    pub payer_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct CaveTunnelInfo {

    /// Payer
    pub payer: Pubkey,

    /// Time of client payment
    pub payment_time: i64,

    /// Duration of service
    pub service_time: u32,
    
}