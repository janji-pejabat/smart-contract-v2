const WalletManager = require('./WalletManager');
const NFTService = require('./NFTService');

class RentalService {
    constructor() {
        this.activeRentals = new Map(); // In production, use database (PostgreSQL)
    }

    /**
     * Creates a rental order
     */
    async createRental(ownerId, nftId, config) {
        // config: { type, duration, price, rewardSplit }
        const rentalId = `rent_${Date.now()}`;
        const order = {
            id: rentalId,
            ownerId,
            nftId,
            renterId: null,
            type: config.type, // 'time', 'match', 'tournament'
            duration: config.duration,
            price: config.price,
            rewardSplit: config.rewardSplit, // { renter: 70, owner: 30 }
            status: 'listed'
        };

        // Save to DB (mocked here)
        this.activeRentals.set(rentalId, order);
        return order;
    }

    /**
     * Executes the rental (renter pays and gets access)
     */
    async rentNFT(renterId, rentalId) {
        const order = this.activeRentals.get(rentalId);
        if (!order || order.status !== 'listed') throw new Error("Rental not available");

        // Logic: Renter pays price to Wallet 3 or directly to owner
        // For simplicity, we assume internal balance check

        order.renterId = renterId;
        order.status = 'active';
        order.startTime = Date.now();
        order.endTime = order.type === 'time' ? Date.now() + (order.duration * 24 * 60 * 60 * 1000) : null;
        order.matchLimit = order.type === 'match' ? order.duration : null;
        order.matchUsed = 0;

        return order;
    }

    /**
     * Validates if an NFT can be used by a user (owner or renter)
     */
    async canUseNFT(userId, nftId) {
        // 1. Check if user is owner
        // 2. Check if user has an active rental for this NFT

        for (const order of this.activeRentals.values()) {
            if (order.nftId === nftId && order.status === 'active') {
                if (order.renterId === userId) {
                    // Check if expired
                    if (this.isExpired(order)) {
                        order.status = 'expired';
                        return false;
                    }
                    return true;
                }
                return false; // Rented by someone else
            }
        }
        return true; // Not rented, assume owner check passes elsewhere
    }

    isExpired(order) {
        if (order.type === 'time' && Date.now() > order.endTime) return true;
        if (order.type === 'match' && order.matchUsed >= order.matchLimit) return true;
        return false;
    }

    /**
     * Distributes rewards based on split
     */
    async distributeReward(orderId, totalReward) {
        const order = this.activeRentals.get(orderId);
        if (!order) return;

        const renterAmount = Math.floor(totalReward * (order.rewardSplit.renter / 100));
        const ownerAmount = totalReward - renterAmount;

        // Call WalletManager to distribute from Game Pool (Wallet 2)
        // WalletManager.sendTransaction(2, order.renterAddress, renterAmount);
        // WalletManager.sendTransaction(2, order.ownerAddress, ownerAmount);

        return { renterAmount, ownerAmount };
    }
}

module.exports = new RentalService();
