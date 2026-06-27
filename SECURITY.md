# Security

## Design Principles

### No Human Controls User Funds
Every token vault (bonding vault, liquidity vault, SOL vault) is a Program Derived Address (PDA).
PDAs have no private key. Only the program logic can move funds from them.
This means nobody — including the platform admin — can steal user funds.

### Admin Capabilities (Limited)
The admin wallet can:
- Update platform config (fees, timings, thresholds)
- Pause the platform in emergencies
- Transfer admin to a new wallet

The admin wallet cannot:
- Access funds in any vault
- Remove token liquidity
- Bypass any fair launch rules
- Stop a token from graduating
- Modify individual token state

### Upgrade Authority
The program is currently upgradeable. This is intentional during the early development phase
so bugs can be fixed quickly without deploying a new program.

Planned: Move upgrade authority to a multisig (Phase 2) and eventually make the program
immutable once the codebase is fully audited and stable.

### Liquidity Lock
On graduation, 800,000,000 tokens + all raised SOL are sent to a Raydium pool.
LP tokens are burned immediately. This is permanent and irreversible.
Nobody can remove liquidity after graduation. Ever.

## Known Limitations (Phase 1)

1. **Raydium CPI not yet implemented** — graduation records on-chain but doesn't create
   the Raydium pool yet. This is the top priority for Phase 2.

2. **Price oracle not used** — graduation threshold is based on SOL raised, not USD market cap.
   A SOL price oracle will be integrated in Phase 2 for accurate USD-denominated graduation.

3. **No token metadata** — Metaplex metadata integration planned for Phase 2.
   Currently using URI field for image/description.

4. **Upgrade authority** — program is upgradeable. Multisig planned for Phase 2.

## Audit Status

Audit in progress. Results will be published here when complete.

## Reporting Vulnerabilities

Found a bug? Please report it responsibly.
DM [@poopdotcash](https://twitter.com/poopdotcash) on Twitter.
Do not publicly disclose vulnerabilities before they are patched.
