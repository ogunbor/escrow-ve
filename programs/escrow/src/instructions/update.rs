use anchor_lang::prelude::*;

use crate::{Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        has_one = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,
}

impl<'info> Update<'info> {
    pub fn update_escrow(&mut self, expiration: i64) -> Result<()> {
        self.escrow.check_expired()?;

        self.escrow.expiration = expiration;

        Ok(())
    }
}
