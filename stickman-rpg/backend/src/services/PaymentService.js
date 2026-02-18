const axios = require('axios');
const WalletManager = require('./WalletManager');
const { DirectSecp256k1HdWallet } = require("@cosmjs/proto-signing");

class PaymentService {
    constructor() {
        this.lcdEndpoint = process.env.PAXI_LCD || "https://mainnet-lcd.paxinet.io";
        this.usedNonces = new Set();
    }

    /**
     * Generates a unique deposit address for a user.
     * In a production environment, this might derive addresses from a master seed.
     */
    async generateUserDepositAddress() {
        const wallet = await DirectSecp256k1HdWallet.generate(12, { prefix: "paxi" });
        const [account] = await wallet.getAccounts();
        return {
            address: account.address,
            mnemonic: wallet.mnemonic // MUST be stored securely or encrypted
        };
    }

    /**
     * Validates a transaction via LCD REST API with Nonce check
     */
    async validateTransaction(txHash, nonce) {
        if (nonce && this.usedNonces.has(nonce)) {
            throw new Error("Transaction replay detected (Nonce already used)");
        }

        try {
            const response = await axios.get(`${this.lcdEndpoint}/cosmos/tx/v1beta1/txs/${txHash}`);
            const txResponse = response.data.tx_response;

            if (txResponse && txResponse.code === 0) {
                if (nonce) this.usedNonces.add(nonce);
                return {
                    success: true,
                    amount: txResponse.tx.body.messages[0].amount[0].amount,
                    denom: txResponse.tx.body.messages[0].amount[0].denom,
                    sender: txResponse.tx.body.messages[0].from_address,
                    recipient: txResponse.tx.body.messages[0].to_address
                };
            }
            return { success: false, error: "Transaction failed or not found" };
        } catch (error) {
            console.error("Error validating transaction:", error.message);
            return { success: false, error: error.message };
        }
    }

    /**
     * Process withdrawal from Wallet 3 (User Deposit & Withdraw)
     */
    async processWithdraw(userAddress, amount, denom = "upaxi") {
        // Apply withdrawal fee preview logic if needed
        const fee = Math.floor(amount * 0.02); // example 2% fee
        const netAmount = amount - fee;

        const txHash = await WalletManager.sendTransaction(3, userAddress, netAmount, denom, "Withdrawal from Stickman RPG");

        // Send fee to Admin Revenue wallet (Wallet 1)
        if (fee > 0) {
            const adminWallet = WalletManager.getWallet(1);
            await WalletManager.sendTransaction(3, adminWallet.address, fee, denom, "Withdrawal Fee");
        }

        return txHash;
    }
}

module.exports = new PaymentService();
