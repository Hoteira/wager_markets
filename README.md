# WAGER Protocol

Permissionless prediction market infrastructure on Solana.

## What is WAGER?

Infrastructure that lets anyone create, embed, and monetize prediction markets in less than 60 seconds.

## Tech Stack

- **Smart Contracts:** Anchor (Rust)
- **Blockchain:** Solana
- **Oracles:** Manual resolution (events)

## Architecture
```
┌─────────────┐
│   Creator   │ Creates market via UI
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│  Market Factory │ Deploys market contract
└────────┬────────┘
         │
         ▼
┌──────────────────┐
│  Market Contract │ ← Users bet here
│  - Escrow        │
│  - AMM pricing   │
│  - Resolution    │
└──────────────────┘
```

## Core Contracts

1. **Market Factory** - Creates new markets
2. **Market** - Individual prediction market
3. **Position** - User's bet in a market

## License

MIT
