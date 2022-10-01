use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod instructions;
pub mod error;

use instructions::{
    initialize::*,
    unlock::*,
    withdraw::*,
    abort::*,
};


#[program]
pub mod token_cave {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        deposit_amount: u64,
        backup_address: Option<Pubkey>,
        timelock_duration: u32,
    ) -> Result<()> {
        instructions::initialize::handler(
            ctx,
            deposit_amount,
            backup_address,
            timelock_duration,
        )
    }

    pub fn unlock(
        ctx: Context<Unlock>,
    ) -> Result<()> {
        instructions::unlock::handler(ctx)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx)
    }

    pub fn abort(
        ctx: Context<Abort>,
    ) -> Result<()> {
        instructions::abort::handler(ctx)
    }
    
}