const { DataTypes } = require('sequelize');

module.exports = (sequelize) => {
    return sequelize.define('Guild', {
        id: {
            type: DataTypes.UUID,
            defaultValue: DataTypes.UUIDV4,
            primaryKey: true
        },
        name: {
            type: DataTypes.STRING,
            allowNull: false,
            unique: true
        },
        level: {
            type: DataTypes.INTEGER,
            defaultValue: 1
        },
        gmr: {
            type: DataTypes.INTEGER,
            defaultValue: 1000
        },
        treasuryBalance: {
            type: DataTypes.BIGINT,
            defaultValue: 0
        },
        leaderId: {
            type: DataTypes.UUID,
            allowNull: false
        },
        maxMembers: {
            type: DataTypes.INTEGER,
            defaultValue: 20
        }
    });
};
