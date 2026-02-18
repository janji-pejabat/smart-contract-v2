const { DataTypes } = require('sequelize');

module.exports = (sequelize) => {
    return sequelize.define('Tournament', {
        id: {
            type: DataTypes.UUID,
            defaultValue: DataTypes.UUIDV4,
            primaryKey: true
        },
        name: {
            type: DataTypes.STRING,
            allowNull: false
        },
        creatorId: {
            type: DataTypes.UUID,
            allowNull: false
        },
        entryFee: {
            type: DataTypes.JSONB
        },
        rewardPool: {
            type: DataTypes.JSONB
        },
        feeConfig: {
            type: DataTypes.JSONB
        },
        maxParticipants: {
            type: DataTypes.INTEGER,
            allowNull: false
        },
        startTime: {
            type: DataTypes.DATE,
            allowNull: false
        },
        status: {
            type: DataTypes.ENUM('open', 'active', 'finished', 'cancelled'),
            defaultValue: 'open'
        },
        brackets: {
            type: DataTypes.JSONB
        }
    });
};
