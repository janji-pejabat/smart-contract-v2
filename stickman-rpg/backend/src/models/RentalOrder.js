const { DataTypes } = require('sequelize');

module.exports = (sequelize) => {
    return sequelize.define('RentalOrder', {
        id: {
            type: DataTypes.UUID,
            defaultValue: DataTypes.UUIDV4,
            primaryKey: true
        },
        nftId: {
            type: DataTypes.STRING,
            allowNull: false
        },
        ownerId: {
            type: DataTypes.UUID,
            allowNull: false
        },
        renterId: {
            type: DataTypes.UUID
        },
        type: {
            type: DataTypes.ENUM('time', 'match', 'tournament'),
            allowNull: false
        },
        price: {
            type: DataTypes.BIGINT,
            allowNull: false
        },
        rewardSplitRenter: {
            type: DataTypes.INTEGER,
            defaultValue: 70
        },
        endTime: {
            type: DataTypes.DATE
        },
        matchLimit: {
            type: DataTypes.INTEGER
        },
        matchUsed: {
            type: DataTypes.INTEGER,
            defaultValue: 0
        },
        status: {
            type: DataTypes.ENUM('listed', 'active', 'expired', 'cancelled'),
            defaultValue: 'listed'
        }
    });
};
