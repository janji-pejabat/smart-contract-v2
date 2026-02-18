const { DataTypes } = require('sequelize');

module.exports = (sequelize) => {
    return sequelize.define('NFT', {
        id: {
            type: DataTypes.STRING, // tokenId
            primaryKey: true
        },
        ownerId: {
            type: DataTypes.UUID,
            allowNull: false
        },
        type: {
            type: DataTypes.ENUM('character', 'cosplay'),
            allowNull: false
        },
        rank: {
            type: DataTypes.STRING, // F, D, C, B, A, SS, SSS, UR, EX
            defaultValue: 'F'
        },
        level: {
            type: DataTypes.INTEGER,
            defaultValue: 1
        },
        exp: {
            type: DataTypes.INTEGER,
            defaultValue: 0
        },
        baseAtk: {
            type: DataTypes.INTEGER
        },
        baseHp: {
            type: DataTypes.INTEGER
        },
        def: {
            type: DataTypes.INTEGER
        },
        metadata: {
            type: DataTypes.JSONB
        },
        isRented: {
            type: DataTypes.BOOLEAN,
            defaultValue: false
        }
    });
};
