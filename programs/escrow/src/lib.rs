use anchor_lang::prelude::*;

declare_id!("8aFsMHXSjEW2wViLdBjYCnVexXCjAq13KowoABFrT6Hs");

pub mod state;
pub use state::*;

pub mod errors;
pub use errors::*;

pub mod constants;
pub use constants::*;

pub mod instructions;
pub use instructions::*;

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(
        ctx: Context<Make>,
        seed: u64,
        receive: u64,
        deposit: u64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts
            .init_escrow(seed, receive, &ctx.bumps, expiration)?;
        ctx.accounts.deposit(deposit)
    }
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }
    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.deposit_from_taker_to_maker()?;
        ctx.accounts.withdraw_from_vault_to_taker_and_close_vault()
    }
    pub fn update(ctx: Context<Update>, expiration: i64) -> Result<()> {
        ctx.accounts.update_escrow(expiration)
    }
}
