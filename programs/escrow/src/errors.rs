use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow has expired")]
    Expired,
    #[msg("Expiration must be zero or a positive unix timestamp")]
    InvalidExpiry,
}