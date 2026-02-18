class RankingService {
    constructor() {
        this.playerMMR = new Map(); // Use DB in production
        this.queue = [];
    }

    getMMR(userId) {
        return this.playerMMR.get(userId) || 1000;
    }

    /**
     * Updates MMR after a battle (ELO-based)
     */
    updateMMR(winnerId, loserId) {
        const winnerMMR = this.getMMR(winnerId);
        const loserMMR = this.getMMR(loserId);

        const K = 32;
        const expectedWinner = 1 / (1 + Math.pow(10, (loserMMR - winnerMMR) / 400));
        const expectedLoser = 1 / (1 + Math.pow(10, (winnerMMR - loserMMR) / 400));

        const newWinnerMMR = Math.round(winnerMMR + K * (1 - expectedWinner));
        const newLoserMMR = Math.round(loserMMR + K * (0 - expectedLoser));

        this.playerMMR.set(winnerId, newWinnerMMR);
        this.playerMMR.set(loserId, newLoserMMR);

        return { newWinnerMMR, newLoserMMR };
    }

    /**
     * Queue system for matchmaking
     */
    addToQueue(userId) {
        const mmr = this.getMMR(userId);
        this.queue.push({ userId, mmr, joinedAt: Date.now() });
        return this.findMatch(userId);
    }

    findMatch(userId) {
        const userEntry = this.queue.find(q => q.userId === userId);
        if (!userEntry) return null;

        const match = this.queue.find(q =>
            q.userId !== userId &&
            Math.abs(q.mmr - userEntry.mmr) < 100 // Match within 100 MMR
        );

        if (match) {
            // Remove both from queue
            this.queue = this.queue.filter(q => q.userId !== userId && q.userId !== match.userId);
            return { playerA: userEntry.userId, playerB: match.userId };
        }

        return null;
    }
}

module.exports = new RankingService();
