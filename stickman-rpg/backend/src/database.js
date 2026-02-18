const { Sequelize } = require('sequelize');
const UserModel = require('./models/User');
const NFTModel = require('./models/NFT');
const RentalOrderModel = require('./models/RentalOrder');
const GuildModel = require('./models/Guild');
const TournamentModel = require('./models/Tournament');
const BattleLogModel = require('./models/BattleLog');

const sequelize = new Sequelize(process.env.DATABASE_URL || 'postgres://user:pass@localhost:5432/stickman_rpg', {
    logging: false,
    dialectOptions: {
        ssl: process.env.NODE_ENV === 'production' ? { rejectUnauthorized: false } : false
    }
});

const User = UserModel(sequelize);
const NFT = NFTModel(sequelize);
const RentalOrder = RentalOrderModel(sequelize);
const Guild = GuildModel(sequelize);
const Tournament = TournamentModel(sequelize);
const BattleLog = BattleLogModel(sequelize);

// Define Associations
User.hasMany(NFT, { foreignKey: 'ownerId' });
NFT.belongsTo(User, { foreignKey: 'ownerId' });

const initDB = async () => {
    try {
        await sequelize.authenticate();
        console.log('Database connection established.');
        await sequelize.sync({ alter: true });
        console.log('Database models synchronized.');
    } catch (error) {
        console.error('Unable to connect to the database:', error);
    }
};

module.exports = {
    sequelize,
    User,
    NFT,
    RentalOrder,
    Guild,
    Tournament,
    BattleLog,
    initDB
};
