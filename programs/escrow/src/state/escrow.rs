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
    /// Errors out once `expiration` has passed. `expiration == 0` is treated
    /// as "never expires".
    pub fn check_expiry(&self) -> Result<()> {
        require!(
            self.expiration == 0 || Clock::get()?.unix_timestamp < self.expiration,
            EscrowError::Expired
        );
        Ok(())
    }

    pub fn set_expiry(&mut self, expiration: i64) -> Result<()> {
        require!(expiration >= 0, EscrowError::InvalidExpiry);
        self.expiration = expiration;
        Ok(())
    }
}