import { SigningStargateClient } from "@cosmjs/stargate";

const RPC_ENDPOINT = "https://mainnet-rpc.paxinet.io";
const LCD_ENDPOINT = "https://mainnet-lcd.paxinet.io";

class BlockchainService {
    async connectWallet() {
        if (!window.paxihub) {
            alert("Please install PaxiHub extension");
            return null;
        }
        await window.paxihub.enable("paxi-mainnet-1");
        const offlineSigner = window.paxihub.getOfflineSigner("paxi-mainnet-1");
        const accounts = await offlineSigner.getAccounts();
        return { signer: offlineSigner, address: accounts[0].address };
    }

    async getBalance(address) {
        const response = await fetch(`${LCD_ENDPOINT}/cosmos/bank/v1beta1/balances/${address}`);
        const data = await response.json();
        return data.balances;
    }

    async sendTokens(sender, recipient, amount, denom = "upaxi") {
        const { signer } = await this.connectWallet();
        const client = await SigningStargateClient.connectWithSigner(RPC_ENDPOINT, signer);
        const amountInCoins = [{ denom, amount: amount.toString() }];
        const fee = {
            amount: [{ denom: "upaxi", amount: "30000" }],
            gas: "200000",
        };
        const result = await client.sendTokens(sender, recipient, amountInCoins, fee, "Stickman RPG Payment");
        return result.transactionHash;
    }
}

export default new BlockchainService();
