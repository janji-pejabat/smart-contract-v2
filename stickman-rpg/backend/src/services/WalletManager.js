/**
 * WalletManager handles the 5-wallet system for Paxi Network.
 * Wallet 1: ADMIN REVENUE
 * Wallet 2: GAME POOL
 * Wallet 3: USER DEPOSIT & WITHDRAW
 * Wallet 4: BURN / LP ENGINE
 * Wallet 5: CORE TRANSACTION
 */

const { SigningStargateClient, coins } = require("@cosmjs/stargate");
const { DirectSecp256k1HdWallet } = require("@cosmjs/proto-signing");

class WalletManager {
    constructor() {
        this.rpcEndpoint = process.env.PAXI_RPC || "https://mainnet-rpc.paxinet.io";
        this.wallets = {}; // Map of wallet index to wallet instance
    }

    async init() {
        // In a real app, these mnemonics come from secure environment variables
        const mnemonics = [
            process.env.MNEMONIC_ADMIN_REVENUE,
            process.env.MNEMONIC_GAME_POOL,
            process.env.MNEMONIC_USER_VAULT,
            process.env.MNEMONIC_BURN_LP,
            process.env.MNEMONIC_CORE_TX
        ];

        for (let i = 0; i < 5; i++) {
            if (mnemonics[i]) {
                const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonics[i], { prefix: "paxi" });
                const [account] = await wallet.getAccounts();
                this.wallets[i + 1] = {
                    wallet,
                    address: account.address
                };
                console.log(`Wallet ${i + 1} initialized: ${account.address}`);
            }
        }
    }

    getWallet(index) {
        return this.wallets[index];
    }

    async sendTransaction(fromIndex, toAddress, amount, denom = "upaxi", memo = "") {
        const source = this.wallets[fromIndex];
        if (!source) throw new Error(`Wallet ${fromIndex} not initialized`);

        const client = await SigningStargateClient.connectWithSigner(this.rpcEndpoint, source.wallet);
        const fee = {
            amount: coins(30000, "upaxi"),
            gas: "200000",
        };

        const result = await client.sendTokens(source.address, toAddress, coins(amount, denom), fee, memo);
        return result.transactionHash;
    }

    /**
     * Rebalancing logic between wallets
     */
    async rebalance(fromIndex, toIndex, amount) {
        const dest = this.wallets[toIndex];
        return this.sendTransaction(fromIndex, dest.address, amount, "upaxi", "Rebalancing");
    }
}

module.exports = new WalletManager();
