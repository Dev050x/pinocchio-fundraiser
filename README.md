# Token Fundraiser (Pinocchio Program)

A Solana on-chain program written using Pinocchio (a low-level Rust framework to write Solana programs without Anchor macros). It allows a maker to create a token fundraiser (SPL token), accept contributions, track contributors, and allow refunds if the fundraiser fails to reach its target within a time period.

## What this program does

| Feature | Description |
|---------|-------------|
| Initialize fundraiser | Maker creates a fundraiser with token mint, amount to raise, and a deadline |
| Accept contributions | Contributors send SPL tokens toward the goal |
| Track contributors | Stores every contributor's total contributed amount |
| Allow refunds | If target not reached, contributors can get their tokens back |
| Check status | Anyone can check whether the goal has been reached |

## Architecture

The program uses two state accounts:

### FundRaiser Account (PDA)

Stores fundraiser metadata.

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FundRaiser {
    pub maker: [u8; 32],
    pub mint_to_raise: [u8; 32],
    pub amount_to_raise: [u8; 8],
    pub current_amount: [u8; 8],
    pub time_started: [u8; 8],
    pub duration: [u8; 1],
    pub bump: [u8; 1],
}
```

| Field | Explanation |
|-------|-------------|
| maker | Wallet public key of fundraiser creator |
| mint_to_raise | SPL token mint expected for contributions |
| amount_to_raise | Target amount (goal) |
| current_amount | Live total of contributed tokens |
| time_started | Unix timestamp of creation |
| duration | Time allowed to reach goal (in days) |
| bump | PDA bump value used for deterministic account derivation |

### Contributor Account (PDA)

Stores contribution amount per contributor.

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contributor {
    pub amount: [u8; 8],
}
```

| Field | Explanation |
|-------|-------------|
| amount | Total contributed tokens by this user |

## Instruction Enum

Defines four callable instructions:

```rust
pub enum Instruction {
    Initialize = 0,
    Contribute = 1,
    Refund = 2,
    Check = 3,
}

impl TryFrom<&u8> for Instruction {
    type Error = pinocchio::program_error::ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Instruction::Initialize),
            1 => Ok(Instruction::Contribute),
            2 => Ok(Instruction::Refund),
            3 => Ok(Instruction::Check),
            _ => Err(pinocchio::program_error::ProgramError::InvalidInstructionData),
        }
    }
}
```

## Entrypoint

```rust
program_entrypoint!(process_instruction);
default_panic_handler!();
no_allocator!();

pinocchio_pubkey::declare_id!("Fg6PaFpoGXkYsidMpWxTWqfQRyQ4aW5n5g5g5g5g5g5g");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(program_id, &ID);
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?;

    match Instruction::try_from(discriminator)? {
        Instruction::Initialize => instructions::intialize::process_initialize(accounts, data)?,
        Instruction::Contribute => instructions::contribute::process_contribute(accounts, data)?,
        Instruction::Refund => instructions::refund::process_refund(accounts)?,
        Instruction::Check => instructions::check_contribution::process_check_contribution(accounts)?,
        _ => return Err(pinocchio::program_error::ProgramError::InvalidInstructionData),
    }

    Ok(())
}
```

## How to Use

### 1. Initialize fundraiser

- Maker signs the transaction
- Pass target amount, duration, and mint address as input

### 2. Contribute

- Contributor sends SPL tokens to fundraiser PDA
- Contributor PDA stores per-user contribution amount

### 3. Refund

Only allowed if:
- Deadline passed AND
- Target amount not reached

Contributor gets back their tokens.

### 4. Check

- Anyone can call

## Build & Deploy

```bash
cargo build-bpf
solana program deploy target/deploy/pinocchio_fundraiser.so
```

Note: Adjust program ID in code before deploying.

## Local Testing (using LiteSVM)

The project uses LiteSVM instead of `solana-test-validator` for faster and more efficient local testing.

```bash
cargo test
```

## Repository Structure

```
src/
 ├── instructions/
 │    ├── initialize.rs
 │    ├── contribute.rs
 │    ├── refund.rs
 │    └── check_contribution.rs
 └── state/
      ├── fundraiser.rs
      └── contributor.rs
```

## Future Enhancements

- Allow multiple fundraisers per maker
- Add event logging
- UI dashboard with contributor leaderboard