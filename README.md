# 💩 poop.cash — Fair Meme Launchpad

> *"We didn't add more rules. We made breaking them impossible."*

poop.cash is a fair meme token launchpad built on Solana. Every protection is enforced by the smart contract itself — not by trust, not by promises, by code.

---

## The Problem

98.6% of tokens launched on Solana are rugs or pump-and-dumps. This happens because the contracts allow it.

- **Bundles** — devs secretly buy 30–50% of supply at launch through multiple wallets
- **Snipers** — bots buy everything in the first seconds before humans have a chance
- **Whale dominance** — one wallet takes a massive position and dumps on everyone
- **Rug pulls** — dev removes liquidity the moment price pumps
- **No creator rewards** — people who build viral memes earn nothing

Every existing "solution" relies on trust. We believe trust is not enough.

---

## The Solution

poop.cash enforces fair launches at the smart contract level. These rules cannot be bypassed, modified, or turned off by anyone — including us.

### Anti-Bundle
One buy per wallet during the first 60 seconds of launch.
No multi-wallet coordinated buys. No secret accumulation at launch.

### Anti-Snipe
- **0–10 seconds**: All buys blocked
- **10–30 seconds**: Max 0.1 SOL per buy
- **30–60 seconds**: Max 0.5 SOL per buy
- **60+ seconds**: Open trading

Bots cannot snipe because the contract physically rejects their transactions.

### Whale Cap
No wallet can ever hold more than **3% of total supply**.
This applies to everyone — buyers, traders, and the token creator.
Enforced on every single buy, not just at launch.

### Creator Rewards
**1% of every trade** (buy and sell) goes to the token creator — forever.
Even after the token graduates to Raydium.
Pump.fun cuts creators off at graduation. We don't.

### Platform Fee
**0.5% per trade** goes to the platform to fund development.

### Fair Graduation
Tokens graduate to Raydium at **~$10k market cap** (~30 SOL raised).
Liquidity is locked forever — LP tokens are burned immediately on graduation.
Nobody can remove liquidity. Ever.

---

## Token Economics

```
Total Supply:        1,000,000,000 tokens

Bonding Curve:         200,000,000 (20%) — available for trading
Liquidity Reserve:     800,000,000 (80%) — locked until graduation

Dev Allocation:                  0 (0%) — none. ever.
Team Allocation:                 0 (0%) — none. ever.
Presale:                         0 (0%) — none. ever.
```

---

## Bonding Curve

Linear bonding curve. Price increases as tokens are purchased.

```
Start price:  0.000001 SOL per token  (~$1k market cap at launch)
End price:    0.00015  SOL per token  (~$10k market cap at graduation)
SOL to grad:  ~30 SOL

Early buyer at $1k mcap who holds to graduation = 10x
```

Price formula:
```
price(tokens_sold) = BASE + (tokens_sold / bonding_supply) × (MAX - BASE)
BASE = 1,000 lamports
MAX  = 150,000 lamports
```

---

## Program Architecture

```
poop.cash program
│
├── PlatformConfig (global PDA)
│   └── All configurable settings — fees, timings, thresholds
│
├── TokenState (per token PDA, derived from mint)
│   └── Tracks name, creator, tokens_sold, sol_raised, graduated
│
├── BuyerRecord (per buyer per token PDA)
│   └── Tracks anti-bundle window participation
│
├── BondingVault (token PDA per mint)
│   └── Holds 200M tokens for bonding curve trading
│
├── LiquidityVault (token PDA per mint)
│   └── Holds 800M tokens for Raydium at graduation
│
└── SolVault (SOL PDA per mint)
    └── Holds all SOL raised from buyers
```

Every vault is a **Program Derived Address (PDA)**. No human — including the platform admin — holds the private key. Only the program logic can move funds.

---

## Instructions

| Instruction | Who Can Call | Description |
|---|---|---|
| `initialize` | Admin (once) | Set up platform with initial config |
| `update_config` | Admin only | Update any config value anytime |
| `set_paused` | Admin only | Emergency pause / unpause |
| `transfer_admin` | Admin only | Hand over admin to new wallet |
| `launch_token` | Anyone | Launch a new token with fair rules |
| `buy` | Anyone | Buy tokens from bonding curve |
| `sell` | Anyone | Sell tokens back to bonding curve |
| `graduate` | Anyone | Trigger graduation when SOL threshold hit |

---

## On-Chain Events

The program emits events for every action. Subscribe via Anchor's event listener for real-time updates.

| Event | When Emitted |
|---|---|
| `PlatformInitialized` | Platform setup |
| `ConfigUpdated` | Config changed |
| `PlatformPaused` | Platform paused or unpaused |
| `AdminTransferred` | Admin changed |
| `TokenLaunched` | New token launched |
| `TokenBought` | Buy executed |
| `TokenSold` | Sell executed |
| `TokenGraduated` | Token graduated to Raydium |

---

## Error Codes

| Error | Meaning |
|---|---|
| `Unauthorized` | Caller is not admin |
| `Paused` | Platform is paused |
| `InvalidName` | Name must be 1–32 chars |
| `InvalidSymbol` | Symbol must be 1–10 chars |
| `InvalidUri` | URI must be under 200 chars |
| `AntiSnipeBlocked` | Buy blocked — first 10 seconds |
| `AntiSnipeLowLimit` | Buy too large — max 0.1 SOL in first 30s |
| `AntiSnipeMidLimit` | Buy too large — max 0.5 SOL in first 60s |
| `BundleDetected` | Already bought in anti-bundle window |
| `WalletCapExceeded` | Would exceed 3% wallet cap |
| `AlreadyGraduated` | Token is on Raydium, bonding curve closed |
| `NotReadyToGraduate` | Not enough SOL raised yet |
| `BondingCurveFull` | All bonding curve tokens sold |
| `InsufficientLiquidity` | Not enough tokens or SOL |
| `InsufficientVaultBalance` | SOL vault balance too low |
| `ZeroAmount` | Amount must be greater than zero |
| `MathError` | Bonding curve calculation error |

---

## Default Config Values

```
Platform fee:        0.5%   (50 bps)
Creator fee:         1.0%   (100 bps)
Max wallet:          3.0%   (300 bps)
Bonding supply:      200,000,000 tokens
Liquidity supply:    800,000,000 tokens
Graduation:          30 SOL (30,000,000,000 lamports)
Snipe block:         10 seconds
Snipe low window:    30 seconds (max 0.1 SOL)
Snipe mid window:    60 seconds (max 0.5 SOL)
Bundle window:       60 seconds (1 buy per wallet)
```

---

## Roadmap

### Phase 1 — Fair Launchpad (Current)
- [x] Anti-bundle enforcement
- [x] Anti-snipe enforcement
- [x] 3% whale cap
- [x] Creator rewards (1% forever)
- [x] Linear bonding curve
- [x] Fair graduation mechanics
- [ ] Raydium LP creation via CPI
- [ ] Token metadata (Metaplex)

### Phase 2 — Social Layer
- [ ] Caller system with on-chain reputation
- [ ] Follow top traders
- [ ] Earn rewards for finding gems early
- [ ] Live chat per token
- [ ] Holder-only comments

### Phase 3 — Rewards Economy
- [ ] Virality mining — share tokens, earn forever
- [ ] Volume milestone bonuses for creators
- [ ] Holder royalties — earn from volume just by holding
- [ ] Staking multipliers for creator fees
- [ ] Platform token ($CASH) with fee revenue share

### Phase 4 — Infrastructure
- [ ] poop.cash becomes the rails other launchpads build on
- [ ] $CASH as gas token
- [ ] Cross-chain expansion

---

## Building & Deploying

### Prerequisites
- Rust
- Solana CLI
- Anchor Framework

### Build
```bash
anchor build
```

### Deploy to Devnet
```bash
anchor deploy --provider.cluster devnet
```

### Initialize Platform
```bash
npx ts-node scripts/initialize.ts
```

### Deploy to Mainnet
```bash
anchor deploy --provider.cluster mainnet
```

---

## Security

- All vaults are PDAs — no human holds private keys
- Upgrade authority held by multisig (Phase 2)
- Emergency pause available for critical incidents
- All config changes are on-chain and transparent
- No admin can access user funds
- No admin can remove liquidity
- Audit in progress — results will be published here

---

## License

MIT

---

## Contact

Twitter: [@poopdotcash](https://twitter.com/poopdotcash)

Website: [poop.cash](https://poop.cash)

---

*Built in public. Contracts published. Community owned.*
*Not financial advice. Meme coins are highly speculative.*
