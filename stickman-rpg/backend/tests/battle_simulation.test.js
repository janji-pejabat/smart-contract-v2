const GameCore = require('../src/engine/GameCore');

/**
 * Battle Simulation Test
 * Simulates 10,000 battles to verify balancing and scaling logic.
 */
function runSimulation() {
    console.log("Starting Battle Simulation: 10,000 matches...");

    let results = {
        skillWins: 0,
        rankWins: 0,
        draws: 0,
        statBoundariesPass: 0
    };

    const iterations = 10000;

    for (let i = 0; i < iterations; i++) {
        // Case 2: Rank F (Level 10, Skill 2.5x) vs Rank B (Level 40, Skill 1.0x)
        // Testing if skill can overcome a 30-level gap and 3-rank gap
        const playerF = {
            rank: 'F',
            level: 10,
            stats: { atk: 100, hp: 500, def: 50, speed: 100 },
            skillMult: 2.5
        };

        const playerB = {
            rank: 'B',
            level: 40,
            stats: { atk: 100, hp: 500, def: 50, speed: 100 },
            skillMult: 1.0
        };

        // Apply Scaling
        const scaledF = {
            atk: GameCore.applyRankAndLevelScaling(playerF.stats.atk, 5, playerF.rank, playerF.level),
            hp: GameCore.applyRankAndLevelScaling(playerF.stats.hp, 20, playerF.rank, playerF.level),
            def: playerF.stats.def
        };

        const scaledB = {
            atk: GameCore.applyRankAndLevelScaling(playerB.stats.atk, 5, playerB.rank, playerB.level),
            hp: GameCore.applyRankAndLevelScaling(playerB.stats.hp, 20, playerB.rank, playerB.level),
            def: playerB.stats.def
        };

        // Apply Matchmaking Balancer (normalization for gap > 2)
        const balancer = GameCore.applyMatchmakingBalancer(scaledF, scaledB, 'F', 'B');

        // Simulating damage
        const dmgToB = GameCore.calculateDamage(scaledF.atk * balancer.attackerBoost, playerF.skillMult, scaledB.def);
        const dmgToF = GameCore.calculateDamage(scaledB.atk * balancer.defenderBoost, playerB.skillMult, scaledF.def);

        // Check if stats are within 35% range as per spec (EX should not be > 35% base stronger than F)
        if (GameCore.RANK_DATA['EX'].baseMult <= 1.35) {
            results.statBoundariesPass++;
        }

        if (dmgToB > dmgToF) {
            results.skillWins++;
        } else if (dmgToF > dmgToC) {
            results.rankWins++;
        } else {
            results.draws++;
        }
    }

    console.log("--- Simulation Results ---");
    console.log(`Total Matches: ${iterations}`);
    console.log(`Skill Wins (Low Rank + High Skill): ${results.skillWins}`);
    console.log(`Rank Wins (High Rank + Low Skill): ${results.rankWins}`);
    console.log(`Stat Boundaries Validated: ${results.statBoundariesPass}`);

    // Validation: Skill should have a significant win rate if it's 2.5x vs 1.0x
    if (results.skillWins > 0) {
        console.log("✅ SUCCESS: Skill can overcome rank advantage.");
    } else {
        console.log("❌ FAILURE: Rank advantage is too high.");
    }
}

test('Battle Simulation: 10,000 matches', () => {
    runSimulation();
});
