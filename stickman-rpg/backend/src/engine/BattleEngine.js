const GameCore = require('./GameCore');

class BattleEngine {
    constructor() {
        this.activeSessions = new Map();
    }

    /**
     * Starts a new battle session
     */
    startBattle(playerA, playerB, type = 'ranked') {
        const sessionId = `battle_${Date.now()}`;
        const session = {
            id: sessionId,
            type,
            players: {
                A: { ...playerA, hp: playerA.stats.hp, pos: { x: -100, y: 0 }, lastInput: Date.now(), cooldowns: {} },
                B: { ...playerB, hp: playerB.stats.hp, pos: { x: 100, y: 0 }, lastInput: Date.now(), cooldowns: {} }
            },
            status: 'running',
            startTime: Date.now(),
            log: []
        };

        this.activeSessions.set(sessionId, session);
        return sessionId;
    }

    /**
     * Processes player input and updates state (Server-side)
     */
    processInput(sessionId, playerId, input) {
        const session = this.activeSessions.get(sessionId);
        if (!session || session.status !== 'running') return;

        const player = session.players[playerId];
        const opponentId = playerId === 'A' ? 'B' : 'A';
        const opponent = session.players[opponentId];

        // 1. Validation (Anti-cheat)
        if (!this.validatePosition(player, input.pos) || !this.validateSpeed(player, input.pos)) {
            console.warn(`Cheating detected for player ${playerId}: movement anomaly`);
            return;
        }

        if (input.action === 'skill' && !this.validateCooldown(player, input.skillId)) {
            console.warn(`Cheating detected for player ${playerId}: cooldown bypass`);
            return;
        }

        // 2. Update Position
        player.pos = input.pos;
        player.lastInput = Date.now();

        // 3. Handle Actions (Attack/Skill)
        if (input.action === 'attack') {
            this.handleAttack(session, playerId, opponentId);
        } else if (input.action === 'skill') {
            this.handleSkill(session, playerId, opponentId, input.skillId);
            player.cooldowns[input.skillId] = Date.now() + 5000; // 5s cooldown
        }

        // 4. Check Victory
        if (opponent.hp <= 0) {
            session.status = 'finished';
            session.winner = playerId;
        }

        return this.getGameState(sessionId);
    }

    validatePosition(player, newPos) {
        // Arena boundaries check
        if (newPos.x < -500 || newPos.x > 500 || newPos.y < -100 || newPos.y > 300) {
            return false;
        }
        return true;
    }

    validateSpeed(player, newPos) {
        const dt = (Date.now() - player.lastInput) / 1000;
        if (dt === 0) return true;

        const dx = newPos.x - player.pos.x;
        const dy = newPos.y - player.pos.y;
        const distance = Math.sqrt(dx * dx + dy * dy);

        const maxSpeed = player.stats.speed || 300; // units per second
        const allowedDistance = (maxSpeed * dt) * 1.2; // 20% tolerance for latency

        return distance <= allowedDistance + 5;
    }

    validateCooldown(player, skillId) {
        if (!player.cooldowns) player.cooldowns = {};
        const readyAt = player.cooldowns[skillId] || 0;
        return Date.now() >= readyAt;
    }

    handleAttack(session, attackerId, targetId) {
        const attacker = session.players[attackerId];
        const target = session.players[targetId];

        // Simple range check
        const dist = Math.abs(attacker.pos.x - target.pos.x);
        if (dist < 50) {
            const damage = GameCore.calculateDamage(attacker.stats.atk, 1.0, target.stats.def);
            target.hp -= damage;
            session.log.push({ t: Date.now(), type: 'damage', from: attackerId, to: targetId, val: damage });
        }
    }

    handleSkill(session, attackerId, targetId, skillId) {
        const attacker = session.players[attackerId];
        const target = session.players[targetId];

        // Find skill config (mocked)
        const skillMult = 2.0;
        const damage = GameCore.calculateDamage(attacker.stats.atk, skillMult, target.stats.def);
        target.hp -= damage;
        session.log.push({ t: Date.now(), type: 'skill', from: attackerId, to: targetId, val: damage, skill: skillId });
    }

    getGameState(sessionId) {
        const session = this.activeSessions.get(sessionId);
        if (!session) return null;
        return {
            players: {
                A: { hp: session.players.A.hp, pos: session.players.A.pos },
                B: { hp: session.players.B.hp, pos: session.players.B.pos }
            },
            status: session.status,
            winner: session.winner
        };
    }
}

module.exports = new BattleEngine();
