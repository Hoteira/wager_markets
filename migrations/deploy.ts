import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { WagerProtocol } from "../target/types/wager_protocol";
import { PublicKey, SystemProgram } from "@solana/web3.js";

module.exports = async function (provider) {
    anchor.setProvider(provider);
    const program = anchor.workspace.WagerProtocol as Program<WagerProtocol>;

    const [protocolPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("protocol")],
        program.programId
    );

    const tx = await program.methods
        .initializeProtocol()
        .accounts({
            authority: provider.wallet.publicKey,
        })
        .rpc();

    console.log("✅ Protocol initialized:", tx);
    console.log("Protocol PDA:", protocolPda.toBase58());
};