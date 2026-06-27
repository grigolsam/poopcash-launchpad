/**
 * poop.cash SDK
 *
 * Drop this file into your frontend project to connect to the poop.cash smart contract.
 * Compatible with React, Next.js, and any TypeScript project.
 *
 * Usage:
 *   import { buyTokens, sellTokens, launchToken, getTokenState } from './poopcash-sdk'
 */

import {
  Connection,
  PublicKey,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { Program, AnchorProvider, BN } from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

// ============================================================
// CONFIG — Update after deployment
// ============================================================

export const PROGRAM_ID = new PublicKey(
  "PoopCashXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" // Replace after deploy
);

export const RPC_DEVNET  = "https://api.devnet.solana.com";
export const RPC_MAINNET = "https://api.mainnet-beta.solana.com";

// ============================================================
// PDA HELPERS
// ============================================================

export const getPlatformConfigPDA = () =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    PROGRAM_ID
  );

export const getTokenStatePDA = (mint: PublicKey) =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("token_state"), mint.toBuffer()],
    PROGRAM_ID
  );

export const getBondingVaultPDA = (mint: PublicKey) =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("bonding_vault"), mint.toBuffer()],
    PROGRAM_ID
  );

export const getLiquidityVaultPDA = (mint: PublicKey) =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("liquidity_vault"), mint.toBuffer()],
    PROGRAM_ID
  );

export const getSolVaultPDA = (mint: PublicKey) =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("sol_vault"), mint.toBuffer()],
    PROGRAM_ID
  );

export const getBuyerRecordPDA = (mint: PublicKey, buyer: PublicKey) =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("buyer_record"), mint.toBuffer(), buyer.toBuffer()],
    PROGRAM_ID
  );

// ============================================================
// READ FUNCTIONS
// ============================================================

/** Fetch platform-wide config and stats */
export async function getPlatformConfig(program: Program) {
  const [pda] = getPlatformConfigPDA();
  const cfg = await program.account.platformConfig.fetch(pda);
  return {
    admin:               cfg.admin.toString(),
    feeWallet:           cfg.feeWallet.toString(),
    platformFeeBps:      cfg.platformFeeBps,
    creatorFeeBps:       cfg.creatorFeeBps,
    maxWalletBps:        cfg.maxWalletBps,
    bondingSupply:       cfg.bondingSupply.toNumber(),
    liquiditySupply:     cfg.liquiditySupply.toNumber(),
    graduationThreshold: cfg.graduationThreshold.toNumber() / LAMPORTS_PER_SOL,
    paused:              cfg.paused,
    totalLaunched:       cfg.totalLaunched.toNumber(),
    totalVolumeSOL:      cfg.totalVolumeLamports.toNumber() / LAMPORTS_PER_SOL,
    totalGraduated:      cfg.totalGraduated.toNumber(),
  };
}

/** Fetch live state for a single token */
export async function getTokenState(program: Program, mint: PublicKey) {
  const [pda] = getTokenStatePDA(mint);
  const ts = await program.account.tokenState.fetch(pda);
  const pctSold = (ts.tokensSold.toNumber() / ts.bondingSupply.toNumber()) * 100;
  const solRaisedSOL = ts.solRaised.toNumber() / LAMPORTS_PER_SOL;

  return {
    creator:        ts.creator.toString(),
    mint:           ts.mint.toString(),
    name:           ts.name,
    symbol:         ts.symbol,
    uri:            ts.uri,
    launchTs:       ts.launchTs.toNumber(),
    bondingSupply:  ts.bondingSupply.toNumber(),
    totalSupply:    ts.totalSupply.toNumber(),
    tokensSold:     ts.tokensSold.toNumber(),
    solRaisedSOL,
    totalVolumeSOL: ts.totalVolume.toNumber() / LAMPORTS_PER_SOL,
    creatorEarned:  ts.creatorEarned.toNumber() / LAMPORTS_PER_SOL,
    graduated:      ts.graduated,
    percentSold:    pctSold.toFixed(2),
    progressToGrad: (solRaisedSOL / 30) * 100, // % to 30 SOL graduation
  };
}

/** Fetch all tokens — for homepage feed */
export async function getAllTokens(program: Program) {
  const accounts = await program.account.tokenState.all();
  return accounts.map((a) => ({
    pubkey:         a.publicKey.toString(),
    mint:           a.account.mint.toString(),
    creator:        a.account.creator.toString(),
    name:           a.account.name,
    symbol:         a.account.symbol,
    uri:            a.account.uri,
    launchTs:       a.account.launchTs.toNumber(),
    tokensSold:     a.account.tokensSold.toNumber(),
    solRaisedSOL:   a.account.solRaised.toNumber() / LAMPORTS_PER_SOL,
    totalVolumeSOL: a.account.totalVolume.toNumber() / LAMPORTS_PER_SOL,
    graduated:      a.account.graduated,
    percentSold:    ((a.account.tokensSold.toNumber() / a.account.bondingSupply.toNumber()) * 100).toFixed(2),
  }));
}

// ============================================================
// WRITE FUNCTIONS
// ============================================================

/** Launch a new token */
export async function launchToken(
  program: Program,
  wallet: PublicKey,
  mint: PublicKey,
  args: { name: string; symbol: string; uri: string }
) {
  const [configPDA]        = getPlatformConfigPDA();
  const [tokenStatePDA]    = getTokenStatePDA(mint);
  const [bondingVaultPDA]  = getBondingVaultPDA(mint);
  const [liquidityVaultPDA] = getLiquidityVaultPDA(mint);
  const [solVaultPDA]      = getSolVaultPDA(mint);

  return await program.methods
    .launchToken({ name: args.name, symbol: args.symbol, uri: args.uri })
    .accounts({
      config:          configPDA,
      tokenState:      tokenStatePDA,
      mint,
      bondingVault:    bondingVaultPDA,
      liquidityVault:  liquidityVaultPDA,
      solVault:        solVaultPDA,
      creator:         wallet,
      tokenProgram:    TOKEN_PROGRAM_ID,
      systemProgram:   anchor.web3.SystemProgram.programId,
      rent:            anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .rpc();
}

/** Buy tokens from bonding curve */
export async function buyTokens(
  program: Program,
  wallet: PublicKey,
  mint: PublicKey,
  solAmountSOL: number
) {
  const [configPDA]       = getPlatformConfigPDA();
  const [tokenStatePDA]   = getTokenStatePDA(mint);
  const [bondingVaultPDA] = getBondingVaultPDA(mint);
  const [solVaultPDA]     = getSolVaultPDA(mint);
  const [buyerRecordPDA]  = getBuyerRecordPDA(mint, wallet);
  const config            = await program.account.platformConfig.fetch(configPDA);
  const tokenState        = await program.account.tokenState.fetch(tokenStatePDA);
  const buyerATA          = await getAssociatedTokenAddress(mint, wallet);
  const lamports          = Math.floor(solAmountSOL * LAMPORTS_PER_SOL);

  return await program.methods
    .buy(new BN(lamports))
    .accounts({
      config:          configPDA,
      tokenState:      tokenStatePDA,
      buyerRecord:     buyerRecordPDA,
      bondingVault:    bondingVaultPDA,
      solVault:        solVaultPDA,
      buyerAta:        buyerATA,
      feeWallet:       config.feeWallet,
      creatorWallet:   tokenState.creator,
      mint,
      buyer:           wallet,
      tokenProgram:             TOKEN_PROGRAM_ID,
      associatedTokenProgram:   ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram:            anchor.web3.SystemProgram.programId,
      rent:                     anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .rpc();
}

/** Sell tokens back to bonding curve */
export async function sellTokens(
  program: Program,
  wallet: PublicKey,
  mint: PublicKey,
  tokenAmount: number
) {
  const [configPDA]       = getPlatformConfigPDA();
  const [tokenStatePDA]   = getTokenStatePDA(mint);
  const [bondingVaultPDA] = getBondingVaultPDA(mint);
  const [solVaultPDA]     = getSolVaultPDA(mint);
  const config            = await program.account.platformConfig.fetch(configPDA);
  const tokenState        = await program.account.tokenState.fetch(tokenStatePDA);
  const sellerATA         = await getAssociatedTokenAddress(mint, wallet);

  return await program.methods
    .sell(new BN(tokenAmount))
    .accounts({
      config:        configPDA,
      tokenState:    tokenStatePDA,
      bondingVault:  bondingVaultPDA,
      solVault:      solVaultPDA,
      sellerAta:     sellerATA,
      feeWallet:     config.feeWallet,
      creatorWallet: tokenState.creator,
      mint,
      seller:        wallet,
      tokenProgram:  TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();
}

/** Trigger graduation — anyone can call when threshold is reached */
export async function graduateToken(
  program: Program,
  wallet: PublicKey,
  mint: PublicKey
) {
  const [configPDA]     = getPlatformConfigPDA();
  const [tokenStatePDA] = getTokenStatePDA(mint);

  return await program.methods
    .graduate()
    .accounts({
      config:     configPDA,
      tokenState: tokenStatePDA,
      caller:     wallet,
    })
    .rpc();
}

// ============================================================
// PRICE HELPERS (client-side calculation, matches contract)
// ============================================================

const PRICE_BASE = 1_000;
const PRICE_MAX  = 150_000;

export function getCurrentPrice(tokensSold: number, bondingSupply: number): number {
  return PRICE_BASE + (tokensSold * (PRICE_MAX - PRICE_BASE)) / bondingSupply;
}

export function calcTokensOut(tokensSold: number, solIn: number, bondingSupply: number): number {
  const price = getCurrentPrice(tokensSold, bondingSupply);
  return Math.floor((solIn * LAMPORTS_PER_SOL) / price);
}

export function calcSolReturn(tokensSold: number, tokenAmount: number, bondingSupply: number): number {
  const price = getCurrentPrice(tokensSold, bondingSupply);
  return (tokenAmount * price) / LAMPORTS_PER_SOL;
}

/** Anti-snipe status for UI display */
export function getAntiSnipeStatus(launchTs: number) {
  const secs = Math.floor(Date.now() / 1000) - launchTs;

  if (secs < 10) return {
    blocked: true,
    label: "🚫 Blocked — anti-snipe active",
    maxSOL: null,
    secondsRemaining: 10 - secs,
  };

  if (secs < 30) return {
    blocked: false,
    label: "⚠️ Max 0.1 SOL per buy",
    maxSOL: 0.1,
    secondsRemaining: 30 - secs,
  };

  if (secs < 60) return {
    blocked: false,
    label: "⚠️ Max 0.5 SOL per buy",
    maxSOL: 0.5,
    secondsRemaining: 60 - secs,
  };

  return {
    blocked: false,
    label: "✅ Open trading",
    maxSOL: null,
    secondsRemaining: 0,
  };
}
