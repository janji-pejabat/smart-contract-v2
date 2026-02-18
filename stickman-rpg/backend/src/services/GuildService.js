class GuildService {
    constructor() {
        this.guilds = new Map();
    }

    async createGuild(ownerId, name) {
        // Biaya pembuatan guild dibagi: % ke Wallet 1, % ke Wallet 4
        const guildId = `guild_${Date.now()}`;
        const guild = {
            id: guildId,
            name,
            level: 1,
            gmr: 1000,
            treasury: 0,
            leader: ownerId,
            officers: [],
            members: [ownerId],
            maxMembers: 20
        };

        this.guilds.set(guildId, guild);
        return guild;
    }

    async joinGuild(userId, guildId) {
        const guild = this.guilds.get(guildId);
        if (!guild) throw new Error("Guild not found");
        if (guild.members.length >= guild.maxMembers) throw new Error("Guild full");

        guild.members.push(userId);
    }

    async upgradeGuild(guildId) {
        const guild = this.guilds.get(guildId);
        if (guild.level >= 10) throw new Error("Max level reached");

        // Logic: Burn PRC20 to upgrade
        guild.level += 1;
        guild.maxMembers += 5;

        return guild;
    }

    /**
     * Guild War Matching (GMR/ELO)
     */
    findGuildMatch(guildId) {
        const myGuild = this.guilds.get(guildId);
        // Logic to find another guild with similar GMR
    }
}

module.exports = new GuildService();
