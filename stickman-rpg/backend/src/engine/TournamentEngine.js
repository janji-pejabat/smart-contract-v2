const WalletManager = require('../services/WalletManager');

class TournamentEngine {
    constructor() {
        this.tournaments = new Map();
    }

    /**
     * Creates a new tournament
     */
    async createTournament(creatorId, config) {
        // config: { name, entryFee, rewardPool, feeConfig, maxParticipants, startTime }
        const tournamentId = `tour_${Date.now()}`;
        const tournament = {
            id: tournamentId,
            creatorId,
            name: config.name,
            entryFee: config.entryFee, // { amount, denom }
            rewardPool: config.rewardPool,
            feeConfig: config.feeConfig, // { platform: 5, burn: 2, lp: 3 }
            maxParticipants: config.maxParticipants,
            participants: [],
            brackets: [],
            status: 'open',
            startTime: config.startTime
        };

        this.tournaments.set(tournamentId, tournament);
        return tournament;
    }

    /**
     * User joins tournament
     */
    async joinTournament(userId, tournamentId) {
        const tournament = this.tournaments.get(tournamentId);
        if (!tournament || tournament.status !== 'open') throw new Error("Tournament not available");
        if (tournament.participants.length >= tournament.maxParticipants) throw new Error("Tournament full");

        // Logic: Check user balance and transfer entry fee
        // Fee distribution according to feeConfig

        tournament.participants.push(userId);

        if (tournament.participants.length === tournament.maxParticipants) {
            this.generateBrackets(tournamentId);
        }

        return tournament;
    }

    generateBrackets(tournamentId) {
        const tournament = this.tournaments.get(tournamentId);
        // Simple single elimination bracket generation
        const players = [...tournament.participants];
        const brackets = [];

        for (let i = 0; i < players.length; i += 2) {
            brackets.push({
                round: 1,
                matchId: `${tournamentId}_r1_${i}`,
                playerA: players[i],
                playerB: players[i+1] || null, // null means bye
                winner: null
            });
        }

        tournament.brackets = brackets;
        tournament.status = 'active';
    }

    async submitMatchResult(tournamentId, matchId, winnerId) {
        const tournament = this.tournaments.get(tournamentId);
        const match = tournament.brackets.find(m => m.matchId === matchId);
        if (!match) throw new Error("Match not found");

        match.winner = winnerId;

        // Logic: Move winner to next round bracket
        // ... (standard bracket logic)
    }

    async finalizeTournament(tournamentId) {
        const tournament = this.tournaments.get(tournamentId);
        // Distribute rewards from Wallet 2 (Game Pool)
        // Distribution logic based on rank in tournament
    }
}

module.exports = new TournamentEngine();
