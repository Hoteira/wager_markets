import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { WagerProtocol } from "../target/types/wager_protocol";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createInitializeMintInstruction,
    createAssociatedTokenAccountInstruction,
    createMintToInstruction,
    getAssociatedTokenAddress,
    MINT_SIZE,
    getMinimumBalanceForRentExemptMint,
} from "@solana/spl-token";
import { assert } from "chai";

describe("wager_protocol", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.WagerProtocol as Program<WagerProtocol>;

    let protocolPda: PublicKey;
    let marketPda: PublicKey;
    let positionPda: PublicKey;
    let usdcMint: Keypair;
    let marketEscrow: PublicKey;
    let userTokenAccount: PublicKey;
    let authorityFeeAccount: PublicKey;
    let devTokenAccount: PublicKey;
    let marketId: number;

    const authorityFeeRecipient = provider.wallet.publicKey;
    const devRecipient = new PublicKey("8Nq7eMbvhZiPzZFeYutAoiHqF2uJTZZWwnBRzvkiUUid");

    before(async () => {
        // Create USDC mock mint
        usdcMint = Keypair.generate();
        const lamports = await getMinimumBalanceForRentExemptMint(provider.connection);

        const createMintTx = new anchor.web3.Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: provider.wallet.publicKey,
                newAccountPubkey: usdcMint.publicKey,
                space: MINT_SIZE,
                lamports,
                programId: TOKEN_PROGRAM_ID,
            }),
            createInitializeMintInstruction(
                usdcMint.publicKey,
                6,
                provider.wallet.publicKey,
                null
            )
        );
        await provider.sendAndConfirm(createMintTx, [usdcMint]);

        // Create user token account
        userTokenAccount = await getAssociatedTokenAddress(
            usdcMint.publicKey,
            provider.wallet.publicKey
        );
        const createAtaTx = new anchor.web3.Transaction().add(
            createAssociatedTokenAccountInstruction(
                provider.wallet.publicKey,
                userTokenAccount,
                provider.wallet.publicKey,
                usdcMint.publicKey
            )
        );
        await provider.sendAndConfirm(createAtaTx);

        // Mint USDC to user
        const mintToTx = new anchor.web3.Transaction().add(
            createMintToInstruction(
                usdcMint.publicKey,
                userTokenAccount,
                provider.wallet.publicKey,
                1000_000000
            )
        );
        await provider.sendAndConfirm(mintToTx);

        console.log("✅ Setup complete. User has 1000 USDC");
    });

    it("Initializes the protocol", async () => {
        [protocolPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("protocol")],
            program.programId
        );

        await program.methods
            .initializeProtocol(500, 200, 30, authorityFeeRecipient)
            .accounts({
                authority: provider.wallet.publicKey,
            })
            .rpc();

        const protocol = await program.account.protocol.fetch(protocolPda);

        assert.ok(protocol.authority.equals(provider.wallet.publicKey));
        assert.equal(protocol.protocolFeeBps, 500);
        assert.equal(protocol.cancelFeeBps, 200);
        assert.equal(protocol.ammFee, 30);
        assert.equal(protocol.marketCount.toNumber(), 0);

        console.log("✅ Protocol initialized");
    });

    it("Creates a market", async () => {
        const protocol = await program.account.protocol.fetch(protocolPda);
        marketId = protocol.marketCount.toNumber();

        [marketPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("market"), new anchor.BN(marketId).toArrayLike(Buffer, "le", 8)],
            program.programId
        );

        marketEscrow = await getAssociatedTokenAddress(
            usdcMint.publicKey,
            marketPda,
            true
        );

        const now = Math.floor(Date.now() / 1000);
        const endTime = new anchor.BN(now + 86400);

        await program.methods
            .createMarket("Will ETH hit $5000 by end of year?", ["Yes", "No"], endTime)
            .accounts({
                protocol: protocolPda,
                creator: provider.wallet.publicKey,
                tokenMint: usdcMint.publicKey,
            })
            .rpc();

        const market = await program.account.market.fetch(marketPda);

        assert.ok(market.creator.equals(provider.wallet.publicKey));
        assert.equal(market.question, "Will ETH hit $5000 by end of year?");
        assert.equal(market.outcomes.length, 2);
        assert.equal(market.resolved, false);

        console.log("✅ Market created:", market.question);
    });

    it("Places a bet on the market", async () => {
        const market = await program.account.market.fetch(marketPda);
        const positionId = market.positionCount.toNumber();

        [positionPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("position"),
                provider.wallet.publicKey.toBuffer(),
                marketPda.toBuffer(),
                new anchor.BN(positionId).toArrayLike(Buffer, "le", 8)
            ],
            program.programId
        );

        const betAmount = new anchor.BN(100_000000);
        const outcome = 0;

        const userBalanceBefore = (
            await provider.connection.getTokenAccountBalance(userTokenAccount)
        ).value.amount;

        await program.methods
            .placeBet(outcome, betAmount)
            .accounts({
                user: provider.wallet.publicKey,
                userTokenAccount: userTokenAccount,
                tokenMint: usdcMint.publicKey,
            })
            .rpc();

        const position = await program.account.position.fetch(positionPda);
        const marketAfter = await program.account.market.fetch(marketPda);
        const userBalanceAfter = (
            await provider.connection.getTokenAccountBalance(userTokenAccount)
        ).value.amount;

        assert.ok(position.user.equals(provider.wallet.publicKey));
        assert.equal(position.outcome, 0);
        assert.equal(position.amount.toNumber(), betAmount.toNumber());
        assert.equal(marketAfter.totalVolume.toNumber(), betAmount.toNumber());
        assert.equal(
            BigInt(userBalanceBefore) - BigInt(userBalanceAfter),
            BigInt(betAmount.toNumber())
        );

        console.log("✅ Bet placed: 100 USDC on outcome 0");
    });

    it("Places opposing bet to create liquidity", async () => {
        const market = await program.account.market.fetch(marketPda);
        const positionId = market.positionCount.toNumber();

        const [opposingPositionPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("position"),
                provider.wallet.publicKey.toBuffer(),
                marketPda.toBuffer(),
                new anchor.BN(positionId).toArrayLike(Buffer, "le", 8)
            ],
            program.programId
        );

        const betAmount = new anchor.BN(100_000000);

        await program.methods
            .placeBet(1, betAmount)
            .accounts({
                user: provider.wallet.publicKey,
                userTokenAccount: userTokenAccount,
                tokenMint: usdcMint.publicKey,
            })
            .rpc();

        console.log("✅ Opposing bet placed: 100 USDC on outcome 1");
    });

    it("Resolves the market", async () => {
        // Wait for end time or skip time check in test
        const winningOutcome = 0;

        // For testing, you may need to modify end_time or wait
        // Here we assume time check passes or is mocked

        await program.methods
            .resolveMarket(winningOutcome)
            .accounts({
                creator: provider.wallet.publicKey,
            })
            .rpc();

        const market = await program.account.market.fetch(marketPda);

        assert.equal(market.resolved, true);
        assert.equal(market.winningOutcome, winningOutcome);

        console.log("✅ Market resolved. Winning outcome:", winningOutcome);
    });

    it("Claims winnings", async () => {
        // Create fee recipient accounts
        authorityFeeAccount = await getAssociatedTokenAddress(
            usdcMint.publicKey,
            authorityFeeRecipient,
            true
        );

        devTokenAccount = await getAssociatedTokenAddress(
            usdcMint.publicKey,
            devRecipient,
            true
        );

        // Create dev token account if needed
        try {
            const createDevAtaTx = new anchor.web3.Transaction().add(
                createAssociatedTokenAccountInstruction(
                    provider.wallet.publicKey,
                    devTokenAccount,
                    devRecipient,
                    usdcMint.publicKey
                )
            );
            await provider.sendAndConfirm(createDevAtaTx);
        } catch (e) {
            // Account may already exist
        }

        const userBalanceBefore = (
            await provider.connection.getTokenAccountBalance(userTokenAccount)
        ).value.amount;

        await program.methods
            .claimWinnings()
            .accounts({
                user: provider.wallet.publicKey,
                userTokenAccount: userTokenAccount,
                authorityFeeRecipient: authorityFeeRecipient,
                tokenMint: usdcMint.publicKey,
            })
            .rpc();

        const userBalanceAfter = (
            await provider.connection.getTokenAccountBalance(userTokenAccount)
        ).value.amount;

        const position = await program.account.position.fetch(positionPda);

        assert.equal(position.claimed, true);
        assert.ok(BigInt(userBalanceAfter) > BigInt(userBalanceBefore));

        console.log("✅ Winnings claimed. Payout:",
            (BigInt(userBalanceAfter) - BigInt(userBalanceBefore)) / BigInt(1_000000), "USDC");
    });
});