<div align="center">
  <br>
  <img src="icon/icon.svg" alt="Wager Protocol Logo" width="120" height="120">

# Wager Protocol

**AMM-based binary prediction market on Solana**

[![Solana](https://img.shields.io/badge/Solana-9945FF?style=flat&logo=solana&logoColor=white)](https://solana.com/)
[![Anchor](https://img.shields.io/badge/Anchor-5865F2?style=flat&logo=anchor&logoColor=white)](https://www.anchor-lang.com/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

<sub>⚡ Constant Product AMM • 💰 Binary Outcomes • 🔒 On-Chain Settlement</sub>

</div>

<br>

## Overview

Wager Protocol is a decentralized prediction market for binary outcomes using constant-product AMM mechanics. Users can:

- **Buy outcome tokens** — Bet on YES/NO outcomes
- **Sell anytime** — Exit positions before market resolution using AMM pricing
- **Claim winnings** — Redeem winning tokens for share of losing pool

Markets resolve after `end_time` when the creator sets the winning outcome.

## Features

- 🔄 **Constant Product AMM** — Dynamic pricing with x·y=k formula
- ⚡ **Instant Liquidity** — No order books, trade anytime
- 💸 **Flexible Positions** — Add to or withdraw from bets before resolution
- 🛡️ **Slippage Protection** — Minimum payout parameters prevent front-running
- 💰 **Fee Distribution** — Protocol and dev fees split 50/50

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (1.17+)
- [Anchor](https://www.anchor-lang.com/docs/installation) (0.32+)
- [Node.js](https://nodejs.org/) (18+)

### Installation
```bash
# Clone the repository
git clone https://github.com/yourusername/wager-protocol.git
cd wager-protocol

# Install dependencies
npm install

# Build the program
anchor build
```

### Deployment

#### 1. Generate a New Solana Wallet
```bash
# Create new keypair
solana-keygen new --outfile ~/.config/solana/deployer.json

# Set as default
solana config set --keypair ~/.config/solana/deployer.json

# Check your address
solana address
```

#### 2. Fund Your Wallet
```bash
# For devnet
solana config set --url devnet
solana airdrop 5

# For mainnet, transfer SOL from an exchange
solana config set --url mainnet-beta
```

#### 3. Update Program ID
```bash
# Get your program ID
anchor keys list

# Update in Anchor.toml and lib.rs
# Replace: declare_id!("82JeHWWTAsHLzQg6XfoXf2HyotFoUrNheNaCd8QphfAC");
#
# And: [programs.devnet]
# wager_protocol = "82JeHWWTAsHLzQg6XfoXf2HyotFoUrNheNaCd8QphfAC"
#
# With your program ID from above
```

#### 4. Build and Deploy
```bash
# Build with new program ID
anchor build

# Deploy to devnet
anchor deploy

# Or deploy to mainnet
anchor deploy --provider.cluster mainnet
```

#### 5. Initialize Protocol
```bash
# Remember to replace the PubKey in /migrations/deply.ts eith your private wallet address to receive the fees !
# const tx = await program.methods
#     .initializeProtocol(
#          500,  // 5% protocol fee
#          200,   // 2% cancel fee
#          30, //0.3& AMM fee
#          new PublicKey("8Nq7eMbvhZiPzZFeYutAoiHqF2uJTZZWwnBRzvkiUUid") //Replace with your wallet's address
# )
#
# Run initialization script
anchor migrate
```

## Usage Example
```typescript
// Create a market
const market = await program.methods
  .createMarket(
    "Will ETH reach $5000 by EOY?",
    ["YES", "NO"],
    new anchor.BN(Date.now() / 1000 + 86400 * 30) // 30 days
  )
  .accounts({
    creator: user.publicKey,
    tokenMint: usdcMint,
  })
  .rpc();

// Place a bet
await program.methods
  .placeBet(0, new anchor.BN(1000000)) // 1 USDC on outcome 0
  .accounts({
    market,
    user: user.publicKey,
    tokenMint: usdcMint,
  })
  .rpc();

// Withdraw early (with slippage protection)
await program.methods
  .withdrawFromPosition(
    new anchor.BN(500000),  // Withdraw 0.5 tokens
    new anchor.BN(450000)   // Minimum 0.45 USDC payout
  )
  .accounts({
    market,
    position,
    user: user.publicKey,
  })
  .rpc();

// Resolve market (after end_time)
await program.methods
  .resolveMarket(0) // Outcome 0 wins
  .accounts({
    market,
    creator: creator.publicKey,
  })
  .rpc();

// Claim winnings
await program.methods
  .claimWinnings()
  .accounts({
    market,
    position,
    user: user.publicKey,
  })
  .rpc();
```

## AMM Mechanics

The protocol uses constant-product formula for withdrawals:
```
k = pool_outcome × pool_other (constant)

When withdrawing X tokens:
1. new_pool_outcome = pool_outcome - X
2. new_pool_other = k / new_pool_outcome
3. payout = pool_other - new_pool_other
4. net_payout = payout - fees
```

## Program Structure
```
wager-protocol/
├── programs/
│   └── wager-protocol/
│       └── src/
│           ├── lib.rs           # Main program logic
│           ├── structs.rs       # Account structures
│           ├── events.rs        # Event definitions
│           ├── error.rs         # Error codes
│           └── constants.rs     # Constants
├── tests/
│   └── wager-protocol.ts        # Integration tests
├── migrations/
│   └── deploy.ts                # Deployment script
└── Anchor.toml                  # Anchor configuration
```

## Testing
```bash
# Run all tests
anchor test

# Test on devnet
anchor test --provider.cluster devnet

# Run specific test
anchor test --skip-deploy -- --grep "withdraw"
```

## Security Considerations

⚠️ **Important:** This is an educational project. Before mainnet deployment:

1. **Audit the code** — Get professional security audit
2. **Oracle integration** — Replace creator-based resolution with oracle (Pyth, Switchboard)
3. **Dispute mechanism** — Add timelock and dispute period
4. **Admin controls** — Implement pause/emergency withdrawal
5. **Rate limiting** — Prevent manipulation attacks
6. **Upgrade authority** — Consider using multisig

## Fees

- **AMM Fee:** 0.3% on withdrawals (stays in pool)
- **Cancel Fee:** Configurable (default 2%) on early exits
- **Protocol Fee:** Configurable (default 5%) on winnings

Fees split 50/50 between protocol authority and dev(me).

## License

Licensed under the [MIT License](LICENSE).

## Contributing

Contributions welcome! Open an issue or PR on GitHub.

---

<div align="center">
  <sub>Built with ⚓ Anchor and ❤️ for DeFi</sub>
</div>
