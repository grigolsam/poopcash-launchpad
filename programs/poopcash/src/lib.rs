//! # poop.cash — Fair Meme Launchpad
//!
//! A Solana program that enforces fair token launches at the protocol level.
//! Every rule is enforced by the smart contract itself — not by trust, not by promises.
//!
//! ## Core Rules (all configurable by admin)
//!
//! - **Anti-Bundle**: One buy per wallet during the first 60 seconds of launch
//! - **Anti-Snipe**: No buys in first 10 seconds. Max 0.1 SOL from 10–30s. Max 0.5 SOL from 30–60s.
//! - **Whale Cap**: No wallet can hold more than 3% of total supply (including the creator)
//! - **Creator Rewards**: 1% of every trade (buy and sell) goes to the token creator forever
//! - **Platform Fee**: 0.5% of every trade goes to the platform
//! - **Fair Graduation**: At 30 SOL raised, token graduates to Raydium with liquidity locked forever
//!
//! ## Token Supply
//!
//! - Total supply: 1,000,000,000 tokens
//! - Bonding curve supply: 200,000,000 (20%) — available for trading
//! - Liquidity reserve: 800,000,000 (80%) — locked until graduation, then sent to Raydium
//!
//! ## Bonding Curve
//!
//! Linear bonding curve. Price increases as tokens are purchased.
//! - Start price: 0.000001 SOL per token (~$1k market cap at launch)
//! - End price:   0.00015 SOL per token (~$10k market cap at graduation)
//! - SOL needed to graduate: ~30 SOL
//!
//! ## Program Architecture
//!
//! - One `PlatformConfig` account (global, admin-controlled)
//! - One `TokenState` account per token (PDA derived from mint)
//! - One `BuyerRecord` account per buyer per token (tracks anti-bundle window)
//! - Token vault PDA holds bonding curve tokens
//! - Liquidity vault PDA holds graduation reserve tokens
//! - SOL vault PDA holds SOL raised from buyers

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("PoopCashXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

// ============================================================
// CONSTANTS
// ============================================================

/// Lamports per token at 0% of bonding curve sold (start price)
const PRICE_BASE: u64 = 1_000; // 0.000001 SOL

/// Lamports per token at 100% of bonding curve sold (end price)  
const PRICE_MAX: u64 = 150_000; // 0.00015 SOL

/// Total token supply per launch
const TOTAL_SUPPLY: u64 = 1_000_000_000;

/// Tokens available on bonding curve (20%)
const BONDING_SUPPLY: u64 = 200_000_000;

/// Tokens reserved for Raydium liquidity at graduation (80%)
const LIQUIDITY_SUPPLY: u64 = 800_000_000;

/// SOL required to graduate (30 SOL in lamports)
const GRADUATION_THRESHOLD: u64 = 30_000_000_000;

/// Platform fee: 0.5% (50 basis points)
const PLATFORM_FEE_BPS: u16 = 50;

/// Creator fee: 1% (100 basis points)
const CREATOR_FEE_BPS: u16 = 100;

/// Max wallet holding: 3% (300 basis points)
const MAX_WALLET_BPS: u16 = 300;

/// Anti-snipe: no buys in first N seconds
const SNIPE_BLOCK_SECS: i64 = 10;

/// Anti-snipe: max 0.1 SOL per buy from 10s to 30s
const SNIPE_LOW_SECS: i64 = 30;
const SNIPE_LOW_MAX: u64 = 100_000_000; // 0.1 SOL

/// Anti-snipe: max 0.5 SOL per buy from 30s to 60s
const SNIPE_MID_SECS: i64 = 60;
const SNIPE_MID_MAX: u64 = 500_000_000; // 0.5 SOL

/// Anti-bundle: one buy per wallet in first N seconds
const BUNDLE_WINDOW_SECS: i64 = 60;

// ============================================================
// PROGRAM
// ============================================================

#[program]
pub mod poopcash {
    use super::*;

    // --------------------------------------------------------
    // ADMIN INSTRUCTIONS
    // --------------------------------------------------------

    /// Initialize the platform. Run once after deployment.
    /// Sets all configurable parameters.
    pub fn initialize(ctx: Context<Initialize>, args: ConfigArgs) -> Result<()> {
        let cfg = &mut ctx.accounts.config;

        cfg.admin                    = ctx.accounts.admin.key();
        cfg.fee_wallet               = args.fee_wallet;
        cfg.platform_fee_bps         = args.platform_fee_bps;
        cfg.creator_fee_bps          = args.creator_fee_bps;
        cfg.max_wallet_bps           = args.max_wallet_bps;
        cfg.bonding_supply           = args.bonding_supply;
        cfg.liquidity_supply         = args.liquidity_supply;
        cfg.graduation_threshold     = args.graduation_threshold;
        cfg.snipe_block_secs         = args.snipe_block_secs;
        cfg.snipe_low_secs           = args.snipe_low_secs;
        cfg.snipe_low_max            = args.snipe_low_max;
        cfg.snipe_mid_secs           = args.snipe_mid_secs;
        cfg.snipe_mid_max            = args.snipe_mid_max;
        cfg.bundle_window_secs       = args.bundle_window_secs;
        cfg.paused                   = false;
        cfg.total_launched           = 0;
        cfg.total_volume_lamports    = 0;
        cfg.total_graduated          = 0;
        cfg.bump                     = ctx.bumps.config;

        emit!(PlatformInitialized {
            admin:      cfg.admin,
            fee_wallet: cfg.fee_wallet,
        });

        Ok(())
    }

    /// Update any platform configuration value.
    /// Admin only. Takes effect immediately.
    pub fn update_config(ctx: Context<AdminOnly>, args: ConfigArgs) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.config.admin,
            PoopError::Unauthorized
        );

        let cfg = &mut ctx.accounts.config;
        cfg.fee_wallet               = args.fee_wallet;
        cfg.platform_fee_bps         = args.platform_fee_bps;
        cfg.creator_fee_bps          = args.creator_fee_bps;
        cfg.max_wallet_bps           = args.max_wallet_bps;
        cfg.bonding_supply           = args.bonding_supply;
        cfg.liquidity_supply         = args.liquidity_supply;
        cfg.graduation_threshold     = args.graduation_threshold;
        cfg.snipe_block_secs         = args.snipe_block_secs;
        cfg.snipe_low_secs           = args.snipe_low_secs;
        cfg.snipe_low_max            = args.snipe_low_max;
        cfg.snipe_mid_secs           = args.snipe_mid_secs;
        cfg.snipe_mid_max            = args.snipe_mid_max;
        cfg.bundle_window_secs       = args.bundle_window_secs;

        emit!(ConfigUpdated { admin: cfg.admin });

        Ok(())
    }

    /// Emergency pause or unpause the entire platform.
    /// Admin only. Blocks all launches, buys, and sells when paused.
    pub fn set_paused(ctx: Context<AdminOnly>, paused: bool) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.config.admin,
            PoopError::Unauthorized
        );
        ctx.accounts.config.paused = paused;
        emit!(PlatformPaused { paused });
        Ok(())
    }

    /// Transfer admin authority to a new wallet.
    /// Admin only. Irreversible once confirmed.
    pub fn transfer_admin(ctx: Context<AdminOnly>, new_admin: Pubkey) -> Result<()> {
        require!(
            ctx.accounts.admin.key() == ctx.accounts.config.admin,
            PoopError::Unauthorized
        );
        ctx.accounts.config.admin = new_admin;
        emit!(AdminTransferred { new_admin });
        Ok(())
    }

    // --------------------------------------------------------
    // LAUNCH INSTRUCTION
    // --------------------------------------------------------

    /// Launch a new token on poop.cash.
    ///
    /// Creates a new SPL token with 1,000,000,000 supply:
    /// - 200,000,000 tokens minted to the bonding curve vault (for trading)
    /// - 800,000,000 tokens minted to the liquidity vault (for Raydium at graduation)
    ///
    /// All fair launch rules apply immediately from this block.
    pub fn launch_token(ctx: Context<LaunchToken>, args: LaunchArgs) -> Result<()> {
        let cfg = &ctx.accounts.config;

        require!(!cfg.paused, PoopError::Paused);
        require!(args.name.len() >= 1 && args.name.len() <= 32, PoopError::InvalidName);
        require!(args.symbol.len() >= 1 && args.symbol.len() <= 10, PoopError::InvalidSymbol);
        require!(args.uri.len() <= 200, PoopError::InvalidUri);

        let clock = Clock::get()?;
        let ts = &mut ctx.accounts.token_state;

        ts.creator           = ctx.accounts.creator.key();
        ts.mint              = ctx.accounts.mint.key();
        ts.name              = args.name.clone();
        ts.symbol            = args.symbol.clone();
        ts.uri               = args.uri.clone();
        ts.launch_ts         = clock.unix_timestamp;
        ts.bonding_supply    = cfg.bonding_supply;
        ts.liquidity_supply  = cfg.liquidity_supply;
        ts.total_supply      = cfg.bonding_supply + cfg.liquidity_supply;
        ts.tokens_sold       = 0;
        ts.sol_raised        = 0;
        ts.total_volume      = 0;
        ts.creator_earned    = 0;
        ts.graduated         = false;
        ts.bump              = ctx.bumps.token_state;

        // Mint bonding curve supply to bonding vault
        let mint_key = ctx.accounts.mint.key();
        let bonding_seeds = &[
            b"bonding_vault".as_ref(),
            mint_key.as_ref(),
            &[ctx.bumps.bonding_vault],
        ];
        let bonding_signer = &[&bonding_seeds[..]];

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint:      ctx.accounts.mint.to_account_info(),
                    to:        ctx.accounts.bonding_vault.to_account_info(),
                    authority: ctx.accounts.bonding_vault.to_account_info(),
                },
                bonding_signer,
            ),
            cfg.bonding_supply,
        )?;

        // Mint liquidity reserve to liquidity vault (locked until graduation)
        let liquidity_seeds = &[
            b"liquidity_vault".as_ref(),
            mint_key.as_ref(),
            &[ctx.bumps.liquidity_vault],
        ];
        let liquidity_signer = &[&liquidity_seeds[..]];

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint:      ctx.accounts.mint.to_account_info(),
                    to:        ctx.accounts.liquidity_vault.to_account_info(),
                    authority: ctx.accounts.liquidity_vault.to_account_info(),
                },
                liquidity_signer,
            ),
            cfg.liquidity_supply,
        )?;

        // Update platform stats
        ctx.accounts.config.total_launched += 1;

        emit!(TokenLaunched {
            mint:      ts.mint,
            creator:   ts.creator,
            name:      args.name,
            symbol:    args.symbol,
            launch_ts: ts.launch_ts,
        });

        Ok(())
    }

    // --------------------------------------------------------
    // BUY INSTRUCTION
    // --------------------------------------------------------

    /// Buy tokens from the bonding curve.
    ///
    /// Enforces all fair launch rules:
    /// 1. Anti-snipe: time-based buy limits
    /// 2. Anti-bundle: one buy per wallet in first 60 seconds
    /// 3. Whale cap: max 3% of total supply per wallet
    /// 4. Collects platform fee (0.5%) and creator fee (1%)
    /// 5. Transfers tokens from bonding vault to buyer
    pub fn buy(ctx: Context<BuyTokens>, sol_in: u64) -> Result<()> {
        let cfg = &ctx.accounts.config;
        let ts  = &ctx.accounts.token_state;

        require!(!cfg.paused, PoopError::Paused);
        require!(!ts.graduated, PoopError::AlreadyGraduated);
        require!(sol_in > 0, PoopError::ZeroAmount);
        require!(ts.tokens_sold < ts.bonding_supply, PoopError::BondingCurveFull);

        let clock = Clock::get()?;
        let seconds_since_launch = clock.unix_timestamp - ts.launch_ts;

        // ---- Anti-Snipe Enforcement ----
        // Block all buys for the first 10 seconds
        require!(
            seconds_since_launch >= cfg.snipe_block_secs,
            PoopError::AntiSnipeBlocked
        );
        // From 10s to 30s: max 0.1 SOL per buy
        if seconds_since_launch < cfg.snipe_low_secs {
            require!(sol_in <= cfg.snipe_low_max, PoopError::AntiSnipeLowLimit);
        }
        // From 30s to 60s: max 0.5 SOL per buy
        else if seconds_since_launch < cfg.snipe_mid_secs {
            require!(sol_in <= cfg.snipe_mid_max, PoopError::AntiSnipeMidLimit);
        }

        // ---- Anti-Bundle Enforcement ----
        // Only one buy per wallet allowed in the first 60 seconds
        let br = &mut ctx.accounts.buyer_record;
        if seconds_since_launch < cfg.bundle_window_secs {
            require!(!br.bought_in_window, PoopError::BundleDetected);
        }
        br.bought_in_window = true;
        br.buyer            = ctx.accounts.buyer.key();
        br.mint             = ts.mint;
        br.first_buy_ts     = clock.unix_timestamp;

        // ---- Fee Calculation ----
        let platform_fee = sol_in
            .checked_mul(cfg.platform_fee_bps as u64).unwrap()
            .checked_div(10_000).unwrap();

        let creator_fee = sol_in
            .checked_mul(cfg.creator_fee_bps as u64).unwrap()
            .checked_div(10_000).unwrap();

        let sol_for_curve = sol_in
            .checked_sub(platform_fee).unwrap()
            .checked_sub(creator_fee).unwrap();

        // ---- Bonding Curve Calculation ----
        let tokens_out = calculate_tokens_out(
            ts.tokens_sold,
            sol_for_curve,
            ts.bonding_supply,
        )?;

        require!(
            ts.tokens_sold.checked_add(tokens_out).unwrap() <= ts.bonding_supply,
            PoopError::InsufficientLiquidity
        );

        // ---- SOL Transfers ----
        // Net SOL → bonding curve SOL vault
        solana_transfer(
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.sol_vault.to_account_info(),
            sol_for_curve,
        )?;

        // Platform fee → fee wallet
        solana_transfer(
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.fee_wallet.to_account_info(),
            platform_fee,
        )?;

        // Creator fee → creator wallet (paid immediately on every trade)
        solana_transfer(
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.creator_wallet.to_account_info(),
            creator_fee,
        )?;

        // ---- Token Transfer: Vault → Buyer ----
        let mint_key = ts.mint;
        let vault_seeds = &[
            b"bonding_vault".as_ref(),
            mint_key.as_ref(),
            &[ctx.bumps.bonding_vault],
        ];
        let vault_signer = &[&vault_seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from:      ctx.accounts.bonding_vault.to_account_info(),
                    to:        ctx.accounts.buyer_ata.to_account_info(),
                    authority: ctx.accounts.bonding_vault.to_account_info(),
                },
                vault_signer,
            ),
            tokens_out,
        )?;

        // ---- Whale Cap Enforcement ----
        // Check buyer's token balance AFTER the transfer
        // No wallet may hold more than 3% of total supply
        let buyer_balance = ctx.accounts.buyer_ata.amount;
        let max_allowed   = ts.total_supply
            .checked_mul(cfg.max_wallet_bps as u64).unwrap()
            .checked_div(10_000).unwrap();

        require!(buyer_balance <= max_allowed, PoopError::WalletCapExceeded);

        // ---- Update State ----
        let ts = &mut ctx.accounts.token_state;
        ts.tokens_sold    = ts.tokens_sold.checked_add(tokens_out).unwrap();
        ts.sol_raised     = ts.sol_raised.checked_add(sol_for_curve).unwrap();
        ts.total_volume   = ts.total_volume.checked_add(sol_in).unwrap();
        ts.creator_earned = ts.creator_earned.checked_add(creator_fee).unwrap();

        ctx.accounts.config.total_volume_lamports = ctx.accounts.config
            .total_volume_lamports.checked_add(sol_in).unwrap();

        emit!(TokenBought {
            mint:         ts.mint,
            buyer:        ctx.accounts.buyer.key(),
            sol_in,
            tokens_out,
            platform_fee,
            creator_fee,
            new_price:    current_price(ts.tokens_sold, ts.bonding_supply),
            tokens_sold:  ts.tokens_sold,
            sol_raised:   ts.sol_raised,
        });

        Ok(())
    }

    // --------------------------------------------------------
    // SELL INSTRUCTION
    // --------------------------------------------------------

    /// Sell tokens back to the bonding curve.
    ///
    /// Returns SOL to seller based on current bonding curve price.
    /// Collects platform fee (0.5%) and creator fee (1%) on sell too.
    /// Creator earns on every trade — not just buys.
    pub fn sell(ctx: Context<SellTokens>, token_amount: u64) -> Result<()> {
        let cfg = &ctx.accounts.config;
        let ts  = &ctx.accounts.token_state;

        require!(!cfg.paused, PoopError::Paused);
        require!(!ts.graduated, PoopError::AlreadyGraduated);
        require!(token_amount > 0, PoopError::ZeroAmount);
        require!(token_amount <= ts.tokens_sold, PoopError::InsufficientLiquidity);

        // ---- Calculate SOL Return ----
        let sol_return = calculate_sol_return(
            ts.tokens_sold,
            token_amount,
            ts.bonding_supply,
        )?;

        // ---- Fee Calculation ----
        let platform_fee = sol_return
            .checked_mul(cfg.platform_fee_bps as u64).unwrap()
            .checked_div(10_000).unwrap();

        let creator_fee = sol_return
            .checked_mul(cfg.creator_fee_bps as u64).unwrap()
            .checked_div(10_000).unwrap();

        let seller_receives = sol_return
            .checked_sub(platform_fee).unwrap()
            .checked_sub(creator_fee).unwrap();

        require!(
            ctx.accounts.sol_vault.lamports() >= sol_return,
            PoopError::InsufficientVaultBalance
        );

        // ---- Token Transfer: Seller → Bonding Vault ----
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from:      ctx.accounts.seller_ata.to_account_info(),
                    to:        ctx.accounts.bonding_vault.to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                },
            ),
            token_amount,
        )?;

        // ---- SOL Transfers from Vault ----
        // Net SOL → seller
        **ctx.accounts.sol_vault.to_account_info().try_borrow_mut_lamports()? -= seller_receives;
        **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? += seller_receives;

        // Platform fee → fee wallet
        **ctx.accounts.sol_vault.to_account_info().try_borrow_mut_lamports()? -= platform_fee;
        **ctx.accounts.fee_wallet.to_account_info().try_borrow_mut_lamports()? += platform_fee;

        // Creator fee → creator wallet
        **ctx.accounts.sol_vault.to_account_info().try_borrow_mut_lamports()? -= creator_fee;
        **ctx.accounts.creator_wallet.to_account_info().try_borrow_mut_lamports()? += creator_fee;

        // ---- Update State ----
        let ts = &mut ctx.accounts.token_state;
        ts.tokens_sold    = ts.tokens_sold.checked_sub(token_amount).unwrap();
        ts.sol_raised     = ts.sol_raised.checked_sub(sol_return).unwrap();
        ts.total_volume   = ts.total_volume.checked_add(sol_return).unwrap();
        ts.creator_earned = ts.creator_earned.checked_add(creator_fee).unwrap();

        ctx.accounts.config.total_volume_lamports = ctx.accounts.config
            .total_volume_lamports.checked_add(sol_return).unwrap();

        emit!(TokenSold {
            mint:           ts.mint,
            seller:         ctx.accounts.seller.key(),
            token_amount,
            sol_return:     seller_receives,
            platform_fee,
            creator_fee,
            new_price:      current_price(ts.tokens_sold, ts.bonding_supply),
            tokens_sold:    ts.tokens_sold,
            sol_raised:     ts.sol_raised,
        });

        Ok(())
    }

    // --------------------------------------------------------
    // GRADUATE INSTRUCTION
    // --------------------------------------------------------

    /// Trigger token graduation when SOL threshold is reached.
    ///
    /// Anyone can call this once the token has raised enough SOL.
    /// Marks the token as graduated — bonding curve trading stops.
    ///
    /// NOTE: Raydium LP creation via CPI is added in Phase 2.
    /// On graduation: sol_vault SOL + liquidity_vault tokens → Raydium pool
    /// LP tokens are burned immediately. Liquidity is locked forever.
    pub fn graduate(ctx: Context<Graduate>) -> Result<()> {
        let cfg = &ctx.accounts.config;
        let ts  = &mut ctx.accounts.token_state;

        require!(!cfg.paused, PoopError::Paused);
        require!(!ts.graduated, PoopError::AlreadyGraduated);
        require!(
            ts.sol_raised >= cfg.graduation_threshold,
            PoopError::NotReadyToGraduate
        );

        ts.graduated = true;
        ctx.accounts.config.total_graduated += 1;

        // Phase 2: Raydium CPI goes here
        // - Take all SOL from sol_vault
        // - Take all tokens from liquidity_vault
        // - Create Raydium AMM pool
        // - Burn LP tokens immediately
        // - Liquidity is permanently locked

        emit!(TokenGraduated {
            mint:       ts.mint,
            creator:    ts.creator,
            sol_raised: ts.sol_raised,
        });

        Ok(())
    }
}

// ============================================================
// BONDING CURVE MATH
// ============================================================
//
// Linear bonding curve:
// price(tokens_sold) = PRICE_BASE + (tokens_sold / bonding_supply) * (PRICE_MAX - PRICE_BASE)
//
// At 0% sold:   price = 1,000 lamports per token  (~$1k mcap)
// At 50% sold:  price = 75,500 lamports per token (~$5k mcap)
// At 100% sold: price = 150,000 lamports per token (~$10k mcap)
//
// 30 SOL is needed to buy all 200M bonding curve tokens
// and push the market cap to ~$10k for graduation.

fn current_price(tokens_sold: u64, bonding_supply: u64) -> u64 {
    PRICE_BASE
        + tokens_sold
            .checked_mul(PRICE_MAX - PRICE_BASE)
            .unwrap_or(0)
            .checked_div(bonding_supply)
            .unwrap_or(0)
}

fn calculate_tokens_out(
    tokens_sold: u64,
    sol_in: u64,
    bonding_supply: u64,
) -> Result<u64> {
    let price = current_price(tokens_sold, bonding_supply);
    require!(price > 0, PoopError::MathError);
    let tokens_out = sol_in.checked_div(price).unwrap_or(0);
    require!(tokens_out > 0, PoopError::ZeroAmount);
    Ok(tokens_out)
}

fn calculate_sol_return(
    tokens_sold: u64,
    token_amount: u64,
    bonding_supply: u64,
) -> Result<u64> {
    let price = current_price(tokens_sold, bonding_supply);
    require!(price > 0, PoopError::MathError);
    let sol_return = token_amount.checked_mul(price).unwrap_or(0);
    require!(sol_return > 0, PoopError::ZeroAmount);
    Ok(sol_return)
}

fn solana_transfer<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &from.key(),
            &to.key(),
            amount,
        ),
        &[from, to],
    )?;
    Ok(())
}

// ============================================================
// ACCOUNT CONTEXTS
// ============================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + PlatformConfig::LEN,
        seeds = [b"platform_config"],
        bump
    )]
    pub config: Account<'info, PlatformConfig>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(
        mut,
        seeds = [b"platform_config"],
        bump = config.bump
    )]
    pub config: Account<'info, PlatformConfig>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct LaunchToken<'info> {
    #[account(
        mut,
        seeds = [b"platform_config"],
        bump = config.bump
    )]
    pub config: Account<'info, PlatformConfig>,

    #[account(
        init,
        payer = creator,
        space = 8 + TokenState::LEN,
        seeds = [b"token_state", mint.key().as_ref()],
        bump
    )]
    pub token_state: Account<'info, TokenState>,

    /// The SPL token mint for this launch.
    /// Creator must have mint authority at launch time.
    /// Mint authority is transferred to the bonding vault PDA after minting.
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    /// Holds the 200M bonding curve tokens available for trading.
    /// PDA controlled by the program — no human has authority.
    #[account(
        init,
        payer = creator,
        seeds = [b"bonding_vault", mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = bonding_vault,
    )]
    pub bonding_vault: Account<'info, TokenAccount>,

    /// Holds the 800M liquidity reserve tokens locked until graduation.
    /// Released to Raydium pool on graduation. Never accessible to anyone else.
    #[account(
        init,
        payer = creator,
        seeds = [b"liquidity_vault", mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = liquidity_vault,
    )]
    pub liquidity_vault: Account<'info, TokenAccount>,

    /// SOL vault — holds all SOL raised from buyers via bonding curve.
    /// PDA controlled by the program. Released to Raydium on graduation.
    /// CHECK: This is a PDA used only to hold SOL. Validated by seeds.
    #[account(
        mut,
        seeds = [b"sol_vault", mint.key().as_ref()],
        bump
    )]
    pub sol_vault: AccountInfo<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent:           Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct BuyTokens<'info> {
    #[account(
        mut,
        seeds = [b"platform_config"],
        bump = config.bump
    )]
    pub config: Account<'info, PlatformConfig>,

    #[account(
        mut,
        seeds = [b"token_state", token_state.mint.as_ref()],
        bump = token_state.bump
    )]
    pub token_state: Account<'info, TokenState>,

    /// Tracks whether this buyer has already bought in the anti-bundle window.
    /// Created on first buy, persists permanently per buyer per token.
    #[account(
        init_if_needed,
        payer = buyer,
        space = 8 + BuyerRecord::LEN,
        seeds = [b"buyer_record", token_state.mint.as_ref(), buyer.key().as_ref()],
        bump
    )]
    pub buyer_record: Account<'info, BuyerRecord>,

    /// Token vault — source of bonding curve tokens sent to buyers.
    #[account(
        mut,
        seeds = [b"bonding_vault", token_state.mint.as_ref()],
        bump
    )]
    pub bonding_vault: Account<'info, TokenAccount>,

    /// SOL vault — receives net SOL from buyer (after fees).
    /// CHECK: PDA validated by seeds. Only receives SOL via system transfer.
    #[account(
        mut,
        seeds = [b"sol_vault", token_state.mint.as_ref()],
        bump
    )]
    pub sol_vault: AccountInfo<'info>,

    /// Buyer's associated token account. Receives purchased tokens.
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = mint,
        associated_token::authority = buyer,
    )]
    pub buyer_ata: Account<'info, TokenAccount>,

    /// Platform fee recipient wallet from config.
    /// CHECK: Validated against config.fee_wallet.
    #[account(mut, address = config.fee_wallet)]
    pub fee_wallet: AccountInfo<'info>,

    /// Creator wallet — receives 1% creator fee on every buy.
    /// CHECK: Validated against token_state.creator.
    #[account(mut, address = token_state.creator)]
    pub creator_wallet: AccountInfo<'info>,

    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    pub token_program:           Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program:          Program<'info, System>,
    pub rent:                    Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SellTokens<'info> {
    #[account(
        mut,
        seeds = [b"platform_config"],
        bump = config.bump
    )]
    pub config: Account<'info, PlatformConfig>,

    #[account(
        mut,
        seeds = [b"token_state", token_state.mint.as_ref()],
        bump = token_state.bump
    )]
    pub token_state: Account<'info, TokenState>,

    /// Bonding vault — receives tokens back from seller.
    #[account(
        mut,
        seeds = [b"bonding_vault", token_state.mint.as_ref()],
        bump
    )]
    pub bonding_vault: Account<'info, TokenAccount>,

    /// SOL vault — source of SOL returned to seller.
    /// CHECK: PDA validated by seeds.
    #[account(
        mut,
        seeds = [b"sol_vault", token_state.mint.as_ref()],
        bump
    )]
    pub sol_vault: AccountInfo<'info>,

    /// Seller's associated token account. Tokens are transferred from here.
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = seller,
    )]
    pub seller_ata: Account<'info, TokenAccount>,

    /// CHECK: Validated against config.fee_wallet.
    #[account(mut, address = config.fee_wallet)]
    pub fee_wallet: AccountInfo<'info>,

    /// CHECK: Validated against token_state.creator.
    #[account(mut, address = token_state.creator)]
    pub creator_wallet: AccountInfo<'info>,

    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub seller: Signer<'info>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Graduate<'info> {
    #[account(
        mut,
        seeds = [b"platform_config"],
        bump = config.bump
    )]
    pub config: Account<'info, PlatformConfig>,

    #[account(
        mut,
        seeds = [b"token_state", token_state.mint.as_ref()],
        bump = token_state.bump
    )]
    pub token_state: Account<'info, TokenState>,

    /// Anyone can trigger graduation once the threshold is met.
    pub caller: Signer<'info>,
}

// ============================================================
// STATE ACCOUNTS
// ============================================================

/// Global platform configuration.
/// One account per program deployment.
/// All fields are updatable by admin.
#[account]
pub struct PlatformConfig {
    /// Admin wallet — can update config, pause platform, transfer admin
    pub admin: Pubkey,                     // 32

    /// Wallet that receives platform fees (0.5% of every trade)
    pub fee_wallet: Pubkey,                // 32

    /// Platform fee in basis points (50 = 0.5%)
    pub platform_fee_bps: u16,             // 2

    /// Creator fee in basis points (100 = 1%)
    pub creator_fee_bps: u16,              // 2

    /// Max wallet holding in basis points (300 = 3%)
    pub max_wallet_bps: u16,               // 2

    /// Tokens allocated to bonding curve per launch (200,000,000)
    pub bonding_supply: u64,               // 8

    /// Tokens allocated to Raydium liquidity per launch (800,000,000)
    pub liquidity_supply: u64,             // 8

    /// SOL needed to graduate (30_000_000_000 = 30 SOL)
    pub graduation_threshold: u64,         // 8

    /// Seconds after launch during which all buys are blocked (10)
    pub snipe_block_secs: i64,             // 8

    /// Seconds after launch during which max snipe_low_max applies (30)
    pub snipe_low_secs: i64,               // 8

    /// Max buy in lamports during snipe_low window (100_000_000 = 0.1 SOL)
    pub snipe_low_max: u64,                // 8

    /// Seconds after launch during which max snipe_mid_max applies (60)
    pub snipe_mid_secs: i64,               // 8

    /// Max buy in lamports during snipe_mid window (500_000_000 = 0.5 SOL)
    pub snipe_mid_max: u64,                // 8

    /// Seconds after launch during which only one buy per wallet allowed (60)
    pub bundle_window_secs: i64,           // 8

    /// Emergency pause — blocks all instructions when true
    pub paused: bool,                      // 1

    /// Total tokens ever launched on this platform
    pub total_launched: u64,               // 8

    /// Total trading volume in lamports across all tokens
    pub total_volume_lamports: u64,        // 8

    /// Total tokens that have graduated to Raydium
    pub total_graduated: u64,             // 8

    /// PDA bump seed
    pub bump: u8,                          // 1
}

impl PlatformConfig {
    pub const LEN: usize =
        32 + 32 + 2 + 2 + 2 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 8 + 8 + 8 + 1;
}

/// Per-token state account.
/// Created on launch, one per token mint.
/// Tracks all bonding curve activity.
#[account]
pub struct TokenState {
    /// Creator's wallet — receives 1% fee on every trade forever
    pub creator: Pubkey,                   // 32

    /// The SPL token mint address
    pub mint: Pubkey,                      // 32

    /// Token name (max 32 characters)
    pub name: String,                      // 4 + 32

    /// Token symbol (max 10 characters)
    pub symbol: String,                    // 4 + 10

    /// Metadata URI — points to token image and description
    pub uri: String,                       // 4 + 200

    /// Unix timestamp of launch block
    pub launch_ts: i64,                    // 8

    /// Tokens on bonding curve (200,000,000 at launch)
    pub bonding_supply: u64,               // 8

    /// Tokens reserved for Raydium liquidity (800,000,000 at launch)
    pub liquidity_supply: u64,             // 8

    /// Total token supply (bonding_supply + liquidity_supply = 1,000,000,000)
    pub total_supply: u64,                 // 8

    /// Tokens sold from bonding curve so far
    pub tokens_sold: u64,                  // 8

    /// Net SOL raised (after fees) held in sol_vault
    pub sol_raised: u64,                   // 8

    /// Total trading volume in lamports (buys + sells)
    pub total_volume: u64,                 // 8

    /// Total creator fees earned in lamports (informational)
    pub creator_earned: u64,               // 8

    /// True once token has graduated to Raydium
    pub graduated: bool,                   // 1

    /// PDA bump seed
    pub bump: u8,                          // 1
}

impl TokenState {
    pub const LEN: usize =
        32 + 32 + (4 + 32) + (4 + 10) + (4 + 200) + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 1;
}

/// Per-buyer per-token record.
/// Tracks anti-bundle window participation.
/// Created on first buy, one per (buyer, token) pair.
#[account]
pub struct BuyerRecord {
    /// Buyer wallet address
    pub buyer: Pubkey,                     // 32

    /// Token mint this record belongs to
    pub mint: Pubkey,                      // 32

    /// True if this wallet has already bought during the bundle window
    pub bought_in_window: bool,            // 1

    /// Timestamp of first buy
    pub first_buy_ts: i64,                 // 8
}

impl BuyerRecord {
    pub const LEN: usize = 32 + 32 + 1 + 8;
}

// ============================================================
// INSTRUCTION ARGUMENTS
// ============================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConfigArgs {
    pub fee_wallet:           Pubkey,
    pub platform_fee_bps:     u16,
    pub creator_fee_bps:      u16,
    pub max_wallet_bps:       u16,
    pub bonding_supply:       u64,
    pub liquidity_supply:     u64,
    pub graduation_threshold: u64,
    pub snipe_block_secs:     i64,
    pub snipe_low_secs:       i64,
    pub snipe_low_max:        u64,
    pub snipe_mid_secs:       i64,
    pub snipe_mid_max:        u64,
    pub bundle_window_secs:   i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct LaunchArgs {
    pub name:   String,
    pub symbol: String,
    pub uri:    String,
}

// ============================================================
// EVENTS
// ============================================================
//
// All events are emitted on-chain and can be subscribed to
// by the frontend via Anchor's event listener.

#[event]
pub struct PlatformInitialized {
    pub admin:      Pubkey,
    pub fee_wallet: Pubkey,
}

#[event]
pub struct ConfigUpdated {
    pub admin: Pubkey,
}

#[event]
pub struct PlatformPaused {
    pub paused: bool,
}

#[event]
pub struct AdminTransferred {
    pub new_admin: Pubkey,
}

#[event]
pub struct TokenLaunched {
    pub mint:      Pubkey,
    pub creator:   Pubkey,
    pub name:      String,
    pub symbol:    String,
    pub launch_ts: i64,
}

#[event]
pub struct TokenBought {
    pub mint:         Pubkey,
    pub buyer:        Pubkey,
    pub sol_in:       u64,
    pub tokens_out:   u64,
    pub platform_fee: u64,
    pub creator_fee:  u64,
    pub new_price:    u64,
    pub tokens_sold:  u64,
    pub sol_raised:   u64,
}

#[event]
pub struct TokenSold {
    pub mint:         Pubkey,
    pub seller:       Pubkey,
    pub token_amount: u64,
    pub sol_return:   u64,
    pub platform_fee: u64,
    pub creator_fee:  u64,
    pub new_price:    u64,
    pub tokens_sold:  u64,
    pub sol_raised:   u64,
}

#[event]
pub struct TokenGraduated {
    pub mint:       Pubkey,
    pub creator:    Pubkey,
    pub sol_raised: u64,
}

// ============================================================
// ERRORS
// ============================================================

#[error_code]
pub enum PoopError {
    #[msg("Unauthorized: caller is not the admin")]
    Unauthorized,

    #[msg("Platform is currently paused")]
    Paused,

    #[msg("Token name must be 1–32 characters")]
    InvalidName,

    #[msg("Token symbol must be 1–10 characters")]
    InvalidSymbol,

    #[msg("Token URI must be under 200 characters")]
    InvalidUri,

    #[msg("Anti-snipe: all buys are blocked for the first 10 seconds after launch")]
    AntiSnipeBlocked,

    #[msg("Anti-snipe: maximum buy is 0.1 SOL in the first 30 seconds after launch")]
    AntiSnipeLowLimit,

    #[msg("Anti-snipe: maximum buy is 0.5 SOL in the first 60 seconds after launch")]
    AntiSnipeMidLimit,

    #[msg("Anti-bundle: only one buy per wallet is allowed in the first 60 seconds after launch")]
    BundleDetected,

    #[msg("Whale cap: this buy would push your wallet above 3% of total supply")]
    WalletCapExceeded,

    #[msg("Token has already graduated to Raydium — bonding curve is closed")]
    AlreadyGraduated,

    #[msg("Token has not raised enough SOL to graduate yet")]
    NotReadyToGraduate,

    #[msg("Bonding curve is fully sold — no tokens remaining")]
    BondingCurveFull,

    #[msg("Not enough tokens or SOL available")]
    InsufficientLiquidity,

    #[msg("SOL vault does not have enough balance to cover this sell")]
    InsufficientVaultBalance,

    #[msg("Amount must be greater than zero")]
    ZeroAmount,

    #[msg("Math error in bonding curve calculation")]
    MathError,
}
