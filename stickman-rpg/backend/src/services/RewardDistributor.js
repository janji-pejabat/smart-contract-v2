const WalletManager = require('./WalletManager');

class RewardDistributor {
    /**
     * Distributes battle/tournament rewards with fees and burn logic
     */
    async distribute(winnerAddress, amount, config = {}) {
        // config: { platformFeeBps: 500, burnBps: 200, lpBps: 300 }
        const platformFeeBps = config.platformFeeBps || 500; // 5%
        const burnBps = config.burnBps || 0;
        const lpBps = config.lpBps || 0;

        const platformFee = Math.floor(amount * platformFeeBps / 10000);
        const burnAmount = Math.floor(amount * burnBps / 10000);
        const lpAmount = Math.floor(amount * lpBps / 10000);
        const netReward = amount - platformFee - burnAmount - lpAmount;

        // Wallet 2 (Game Pool) -> Winner
        const txs = [];

        if (netReward > 0) {
            txs.push(await WalletManager.sendTransaction(2, winnerAddress, netReward, "upaxi", "Battle Reward"));
        }

        if (platformFee > 0) {
            const adminWallet = WalletManager.getWallet(1);
            txs.push(await WalletManager.sendTransaction(2, adminWallet.address, platformFee, "upaxi", "Platform Fee"));
        }

        if (burnAmount > 0 || lpAmount > 0) {
            const burnLPWallet = WalletManager.getWallet(4);
            txs.push(await WalletManager.sendTransaction(2, burnLPWallet.address, burnAmount + lpAmount, "upaxi", "Burn/LP Allocation"));
        }

        return {
            txHashes: txs,
            distributed: {
                winner: netReward,
                platform: platformFee,
                burn: burnAmount,
                lp: lpAmount
            }
        };
    }
}

module.exports = new RewardDistributor();
