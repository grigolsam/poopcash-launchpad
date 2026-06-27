/**
 * poop.cash — Platform Initialization Script
 *
 * Run once after deployment to set up the platform config.
 * Usage: npx ts-node scripts/initialize.ts
 */

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Poopcash as Program;
  const admin = provider.wallet.publicKey;

  console.log("=".repeat(50));
  console.log("poop.cash Platform Initialization");
  console.log("=".repeat(50));
  console.log("Admin wallet:", admin.toString());
  console.log("Program ID:", program.programId.toString());
  console.log("");

  const [configPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")],
    program.programId
  );

  console.log("Config PDA:", configPDA.toString());
  console.log("Initializing...");

  const tx = await program.methods
    .initialize({
      feeWallet:             admin,              // Change to your fee wallet
      platformFeeBps:        50,                 // 0.5%
      creatorFeeBps:         100,                // 1.0%
      maxWalletBps:          300,                // 3.0%
      bondingSupply:         new anchor.BN(200_000_000),
      liquiditySupply:       new anchor.BN(800_000_000),
      graduationThreshold:   new anchor.BN(30 * LAMPORTS_PER_SOL),
      snipeBlockSecs:        new anchor.BN(10),
      snipeLowSecs:          new anchor.BN(30),
      snipeLowMax:           new anchor.BN(100_000_000),  // 0.1 SOL
      snipeMidSecs:          new anchor.BN(60),
      snipeMidMax:           new anchor.BN(500_000_000),  // 0.5 SOL
      bundleWindowSecs:      new anchor.BN(60),
    })
    .accounts({
      config:        configPDA,
      admin,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("");
  console.log("✅ Platform initialized successfully!");
  console.log("Transaction:", tx);
  console.log("");
  console.log("Platform Settings:");
  console.log("  Platform fee:     0.5%");
  console.log("  Creator fee:      1.0%");
  console.log("  Max wallet:       3.0%");
  console.log("  Bonding supply:   200,000,000 tokens (20%)");
  console.log("  Liquidity supply: 800,000,000 tokens (80%)");
  console.log("  Graduation:       30 SOL");
  console.log("  Anti-snipe:       10s block, 30s 0.1 SOL, 60s 0.5 SOL");
  console.log("  Anti-bundle:      60s window");
  console.log("");
  console.log("Save these addresses:");
  console.log("  Program ID:", program.programId.toString());
  console.log("  Config PDA:", configPDA.toString());
}

main().catch((err) => {
  console.error("Initialization failed:", err);
  process.exit(1);
});
