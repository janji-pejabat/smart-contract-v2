const WalletManager = require('./WalletManager');
const NFTService = require('./NFTService');

class MarketplaceService {
    constructor() {
        this.listings = new Map(); // In production, use database
    }

    /**
     * Lists an NFT for sale
     */
    async listNFT(ownerId, nftId, contractAddress, price, denom = "upaxi") {
        const listingId = `list_${Date.now()}`;
        const listing = {
            id: listingId,
            ownerId,
            nftId,
            contractAddress,
            price,
            denom,
            status: 'active'
        };

        this.listings.set(listingId, listing);
        return listing;
    }

    /**
     * Executes NFT purchase
     */
    async buyNFT(buyerId, listingId) {
        const listing = this.listings.get(listingId);
        if (!listing || listing.status !== 'active') throw new Error("Listing not available");

        // 1. Calculate fees
        const platformFee = Math.floor(listing.price * 0.05); // 5% platform fee
        const sellerAmount = listing.price - platformFee;

        // 2. Transfer payment (handled via Wallet 3 User Vault)
        // WalletManager.sendTransaction(3, listing.ownerAddress, sellerAmount, listing.denom);
        // WalletManager.sendTransaction(3, WalletManager.getWallet(1).address, platformFee, listing.denom);

        // 3. Transfer NFT
        // await NFTService.transferNFT(listing.contractAddress, 5, buyerAddress, listing.nftId);

        listing.status = 'sold';
        listing.buyerId = buyerId;

        return listing;
    }
}

module.exports = new MarketplaceService();
