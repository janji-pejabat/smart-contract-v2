const { DataTypes } = require('sequelize');

module.exports = (sequelize) => {
    return sequelize.define('BattleLog', {
        id: {
            type: DataTypes.UUID,
            defaultValue: DataTypes.UUIDV4,
            primaryKey: true
        },
        sessionId: {
            type: DataTypes.STRING,
            allowNull: false
        },
        playerA: {
            type: DataTypes.UUID,
            allowNull: false
        },
        playerB: {
            type: DataTypes.UUID,
            allowNull: false
        },
        winner: {
            type: DataTypes.UUID
        },
        type: {
            type: DataTypes.STRING // ranked, tournament, guild_war
        },
        log: {
            type: DataTypes.JSONB
        },
        timestamp: {
            type: DataTypes.DATE,
            defaultValue: DataTypes.NOW
        }
    });
};
