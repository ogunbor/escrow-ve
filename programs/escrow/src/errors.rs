use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("The escrow has expired")]
    EscrowExpired,
    #[msg("The escrow has not expired yet")]
    EscrowNotExpired,
}
