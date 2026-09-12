# escrow-ve

A token-swap escrow program built with [Anchor](https://www.anchor-lang.com/) (`anchor-lang` / `anchor-spl` 1.1.2).

A maker deposits token A into a vault and specifies how much of token B they want in return. Any taker can fulfill the trade before the escrow expires; after expiry, only a refund back to the maker is allowed.

Program ID (localnet): `8aFsMHXSjEW2wViLdBjYCnVexXCjAq13KowoABFrT6Hs`

## Instructions

| Instruction  | Who calls it | What it does |
|---|---|---|
| `initialize` | maker | Creates the escrow PDA, deposits token A into the vault, sets `receive`, `expiration`, and `created_at` |
| `take`       | taker | Sends token B to the maker, withdraws token A from the vault, closes the escrow (rent returns to taker) — fails if expired |
| `refund`     | maker | Returns token A from the vault to the maker and closes the escrow — only allowed once expired |
| `update`     | maker | Changes `expiration` on an existing escrow — only allowed once the current expiration has already passed |

## Expiry model

`expiration` is stored as a duration in seconds, not an absolute timestamp. The actual expiry moment is `created_at + expiration`.

This logic lives on the `Escrow` account itself (`state/escrow.rs`), as two directional checks:

- `check_not_expired()` — used by `take`; errors with `EscrowExpired` once past the expiry window
- `check_expired()` — used by `refund` and `update`; errors with `EscrowNotExpired` until past the expiry window

## Accounts

- `Escrow` (PDA, seeds `["escrow", maker, seed]`) — `seed`, `maker`, `mint_a`, `mint_b`, `receive`, `bump`, `expiration`, `created_at`
- `vault` — an associated token account for `mint_a`, owned by the `Escrow` PDA

## Project structure

```
programs/escrow/src/
  lib.rs                # program entrypoints
  state/escrow.rs        # Escrow account + expiry check methods
  errors.rs              # EscrowError
  constants.rs           # ESCROW_SEED
  instructions/
    make.rs
    take.rs
    refund.rs
    update.rs
programs/escrow/tests/
  escrow.rs              # litesvm integration tests (builder pattern + #[test] fns)
```

## Build & test

```bash
anchor build   # compiles the program to target/deploy/escrow.so
cargo test     # runs the litesvm test suite against that .so
```

Tests run entirely in-process via [litesvm](https://github.com/LiteSVM/litesvm) — no local validator required.

## Toolchain

- `anchor-cli` 1.1.2
- `solana-cli` (Agave) 3.1.10 or newer
- Rust: pinned via `rust-toolchain.toml` (stable, with `rustfmt`/`clippy`)
