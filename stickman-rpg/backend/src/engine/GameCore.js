/**
 * Game Core Logic for Stickman RPG Arena
 * Handles stat scaling, balancing, and damage calculation.
 */

const RANK_DATA = {
    'F':   { maxLv: 10,  baseMult: 1.00, growthMult: 1.00 },
    'D':   { maxLv: 20,  baseMult: 1.05, growthMult: 1.03 },
    'C':   { maxLv: 30,  baseMult: 1.08, growthMult: 1.05 },
    'B':   { maxLv: 40,  baseMult: 1.12, growthMult: 1.07 },
    'A':   { maxLv: 60,  baseMult: 1.18, growthMult: 1.10 },
    'SS':  { maxLv: 80,  baseMult: 1.22, growthMult: 1.12 },
    'SSS': { maxLv: 120, baseMult: 1.27, growthMult: 1.15 },
    'UR':  { maxLv: 160, baseMult: 1.32, growthMult: 1.17 },
    'EX':  { maxLv: 200, baseMult: 1.35, growthMult: 1.20 }
};

const SOFT_CAPS = {
    CRIT_RATE: 0.40,
    SPEED: 1.50,
    SKILL_POWER: 2.00
};

const SOFT_CAP_DIMINISHING_FACTOR = 0.3;

/**
 * Applies soft cap logic to a stat.
 * If value > cap, excess is multiplied by factor.
 */
function applySoftCap(value, cap) {
    if (value <= cap) return value;
    const excess = value - cap;
    return cap + (excess * SOFT_CAP_DIMINISHING_FACTOR);
}

/**
 * Calculates base stats based on rank and level.
 */
function applyRankAndLevelScaling(baseStat, growthStat, rank, level) {
    const rankConfig = RANK_DATA[rank] || RANK_DATA['F'];

    // Level cap enforcement
    const effectiveLevel = Math.min(level, rankConfig.maxLv);

    // HP/ATK = (Base × RankMultiplier) + (Level × Growth × RankGrowthMultiplier)
    const scaledStat = (baseStat * rankConfig.baseMult) +
                       (effectiveLevel * growthStat * rankConfig.growthMult);

    return scaledStat;
}

/**
 * Effective Defense using diminishing returns: EffectiveDEF = DEF / (DEF + 100)
 */
function calculateEffectiveDefense(def) {
    return def / (def + 100);
}

/**
 * Damage = ATK × SkillMultiplier × (1 - EffectiveDEF)
 */
function calculateDamage(atk, skillMultiplier, targetDef) {
    const effectiveDef = calculateEffectiveDefense(targetDef);
    const multiplier = Math.min(skillMultiplier, 2.5); // Cap skill multiplier at 2.5x
    const damage = atk * multiplier * (1 - effectiveDef);
    return Math.max(0, damage);
}

/**
 * Ensures stats stay within reasonable boundaries.
 */
function validateStatBoundary(stats) {
    const validated = { ...stats };

    if (validated.critRate) {
        validated.critRate = applySoftCap(validated.critRate, SOFT_CAPS.CRIT_RATE);
    }

    if (validated.speed) {
        validated.speed = applySoftCap(validated.speed, SOFT_CAPS.SPEED);
    }

    if (validated.skillPower) {
        validated.skillPower = applySoftCap(validated.skillPower, SOFT_CAPS.SKILL_POWER);
    }

    return validated;
}

/**
 * Matchmaking Balancer
 * Normalizes stats if there's a large rank gap.
 */
function applyMatchmakingBalancer(attackerStats, defenderStats, attackerRank, defenderRank) {
    const rankList = Object.keys(RANK_DATA);
    const attackerIdx = rankList.indexOf(attackerRank);
    const defenderIdx = rankList.indexOf(defenderRank);
    const gap = attackerIdx - defenderIdx;

    let attackerBoost = 1.0;
    let defenderBoost = 1.0;

    if (Math.abs(gap) > 2) {
        if (gap > 0) {
            // Attacker is much higher rank
            attackerBoost = 0.90; // 10% normalization
            defenderBoost = 1.05; // 5% boost
        } else {
            // Attacker is much lower rank
            attackerBoost = 1.03; // 3% boost
            defenderBoost = 0.95; // 5% normalization
        }
    }

    return { attackerBoost, defenderBoost };
}

module.exports = {
    RANK_DATA,
    SOFT_CAPS,
    applySoftCap,
    applyRankAndLevelScaling,
    calculateDamage,
    validateStatBoundary,
    applyMatchmakingBalancer
};
