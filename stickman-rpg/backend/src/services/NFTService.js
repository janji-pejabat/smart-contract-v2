const axios = require('axios');
const WalletManager = require('./WalletManager');
const { SigningStargateClient, toBinary } = require("@cosmjs/stargate");

class NFTService {
    constructor() {
        this.lcdEndpoint = process.env.PAXI_LCD || "https://mainnet-lcd.paxinet.io";
        this.characterContract = process.env.CHARACTER_NFT_CONTRACT;
        this.cosplayContract = process.env.COSPLAY_NFT_CONTRACT;
    }

    /**
     * Mints a new Stickman Character NFT (Admin Only)
     */
    async mintCharacter(ownerAddress, tokenId, metadata) {
        const adminWallet = WalletManager.getWallet(5); // Core Transaction wallet for operational
        const client = await SigningStargateClient.connectWithSigner(WalletManager.rpcEndpoint, adminWallet.wallet);

        const msg = {
            mint: {
                token_id: tokenId,
                owner: ownerAddress,
                token_uri: metadata.uri,
                extension: {
                    rank: metadata.rank,
                    level: 1,
                    base_stats: metadata.baseStats,
                    skill_type: metadata.skillType,
                    rarity: metadata.rarity,
                    upgrade_history: []
                }
            }
        };

        const result = await client.execute(
            adminWallet.address,
            this.characterContract,
            msg,
            "auto",
            "Minting Stickman Character"
        );
        return result.transactionHash;
    }

    /**
     * Queries NFT details from the contract
     */
    async getNFTDetails(contractAddress, tokenId) {
        const query = Buffer.from(JSON.stringify({
            nft_info: { token_id: tokenId }
        })).toString('base64');

        try {
            const response = await axios.get(`${this.lcdEndpoint}/cosmwasm/wasm/v1/contract/${contractAddress}/smart/${query}`);
            return response.data.data;
        } catch (error) {
            console.error("Error querying NFT details:", error.message);
            return null;
        }
    }

    /**
     * Transfers NFT from one address to another (Internal logic for marketplace)
     * In a real DApp, the user signs this, but if we use a custodial wallet for the marketplace:
     */
    async transferNFT(contractAddress, fromWalletIndex, toAddress, tokenId) {
        const wallet = WalletManager.getWallet(fromWalletIndex);
        const client = await SigningStargateClient.connectWithSigner(WalletManager.rpcEndpoint, wallet.wallet);

        const msg = {
            transfer_nft: {
                recipient: toAddress,
                token_id: tokenId
            }
        };

        const result = await client.execute(
            wallet.address,
            contractAddress,
            msg,
            "auto",
            "NFT Transfer"
        );
        return result.transactionHash;
    }
}

module.exports = new NFTService();
