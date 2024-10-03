use anchor_lang::prelude::*;

declare_id!("49gLH2qk1djQLGBwBc89YKgU7sqx4QoCLnTq7J9wEwkE");

pub mod state;
pub use state::*;

pub mod instructions;
pub use instructions::*;

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(ctx: Context<Make>, seed: u64, receive: u64, deposit: u64) -> Result<()> {
        ctx.accounts.init_escrow(seed, receive, &ctx.bumps)?;
        ctx.accounts.deposit(deposit)
    }
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.deposit_from_taker_to_maker()?;
        ctx.accounts.withdraw_from_vault_to_taker_and_close_vault()
    }
}
