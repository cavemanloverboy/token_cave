use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod instructions;
pub mod error;

use instructions::{
    payment::*,
    payout::*
};


#[program]
pub mod token_cave {

    use super::*;

    pub fn payment(
        ctx: Context<Payment>,
        service_time: u32,
    ) -> Result<()> {
        instructions::payment::handler(ctx, service_time)
    }

    #[allow(unused_variables)]
    pub fn payout(
        ctx: Context<Payout>,
        payee_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::payout::handler(ctx)
    }
    
}