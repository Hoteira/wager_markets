# WAGER Protocol

Permissionless prediction market infrastructure on Solana.

## What is WAGER?

Infrastructure that lets anyone create, embed, and monetize prediction markets in 60 seconds.

- **For Creators:** Turn predictions into markets, earn fees
- **For Users:** Bet on anything, anywhere
- **For Developers:** Composable market infrastructure

## Tech Stack

- **Smart Contracts:** Anchor (Rust)
- **Blockchain:** Solana
- **Frontend:** Next.js + Tailwind
- **Oracles:** Pyth Network (prices), Manual resolution (events)

## Status

🚧 In development - Building in public

Follow progress: [@wager_markets](https://twitter.com/wager_markets)

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

## Roadmap

- [<span style="color: yellow;"> 50%</span>] Core market contracts (binary outcomes) 
- [<span style="color: red;">  0%</span>] AMM pricing mechanism
- [<span style="color: red;">  0%</span>] Resolution system (manual → oracle)
- [<span style="color: red;">  0%</span>] Market creation UI
- [<span style="color: red;">  0%</span>] Embeddable widget
- [<span style="color: red;">  0%</span>] Public launch

## License

MIT
