import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { WagerProtocol } from "../target/types/wager_protocol";
import { PublicKey } from "@solana/web3.js";

module.exports = async function (provider: anchor.Provider) {
    console.log("Migration script loaded ✅");

    anchor.setProvider(provider);
    const program = anchor.workspace.WagerProtocol as Program<WagerProtocol>;

    const [protocolPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("protocol")],
        program.programId
    );

    const tx = await program.methods
        .initializeProtocol(
            500,  // 5% protocol fee
            200,   // 2% cancel fee
            30, //0.3& AMM fee
            new PublicKey("8Nq7eMbvhZiPzZFeYutAoiHqF2uJTZZWwnBRzvkiUUid") //Replace with your wallet's address
        )
        .accounts({
            authority: provider.wallet.publicKey,
        }).rpc();

    console.log("✅ Protocol initialized:", tx);
    console.log("Protocol PDA:", protocolPda.toBase58());
    console.log("Authority:", provider.wallet.publicKey.toBase58());
    console.log("Dev Recipient: 8Nq7eMbvhZiPzZFeYutAoiHqF2uJTZZWwnBRzvkiUUid");
};