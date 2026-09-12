use anchor_lang::prelude::*;

use crate::errors::EscrowError;

#[derive(InitSpace)]
#[account(discriminator = 1)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive: u64,
    pub bump: u8,
    pub expiration: i64,
    pub created_at: i64,
}

impl Escrow {
    pub fn check_not_expired(&self) -> Result<()> {
        require!(
            Clock::get()?.unix_timestamp <= self.created_at + self.expiration,
            EscrowError::EscrowExpired
        );
        Ok(())
    }

    pub fn check_expired(&self) -> Result<()> {
        require!(
            Clock::get()?.unix_timestamp >= self.created_at + self.expiration,
            EscrowError::EscrowNotExpired
        );
        Ok(())
    }
}
