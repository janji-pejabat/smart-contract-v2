const { DataTypes } = require('sequelize');

module.exports = (sequelize) => {
    return sequelize.define('User', {
        id: {
            type: DataTypes.UUID,
            defaultValue: DataTypes.UUIDV4,
            primaryKey: true
        },
        walletAddress: {
            type: DataTypes.STRING,
            allowNull: false,
            unique: true
        },
        username: {
            type: DataTypes.STRING,
            allowNull: false
        },
        mmr: {
            type: DataTypes.INTEGER,
            defaultValue: 1000
        },
        internalBalance: {
            type: DataTypes.BIGINT,
            defaultValue: 0
        },
        depositAddress: {
            type: DataTypes.STRING
        }
    });
};
