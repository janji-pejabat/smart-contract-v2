import React, { useState, useEffect } from 'react';
import { Shield, Swords, Users, Zap } from 'lucide-react';

const LandingPage = () => {
    const [stats, setStats] = useState({
        totalNFT: 1250,
        totalBurned: "450,000 PRC20",
        totalRewards: "1,200,000 PAXI",
        activeTournaments: 3
    });

    return (
        <div className="bg-slate-900 text-white min-h-screen">
            {/* Hero Section */}
            <header className="py-20 px-4 text-center">
                <h1 className="text-6xl font-black mb-4 tracking-tighter italic">
                    STICKMAN <span className="text-blue-500">RPG</span> ARENA
                </h1>
                <p className="text-xl text-slate-400 mb-8 max-w-2xl mx-auto">
                    The ultimate Web3 2D combat experience on Paxi Network.
                    Own, level up, and battle with unique Stickman NFTs.
                </p>
                <div className="flex justify-center gap-4">
                    <button className="bg-blue-600 hover:bg-blue-700 px-8 py-4 rounded-xl font-bold text-lg transition-all transform hover:scale-105">
                        Start Battle
                    </button>
                    <button className="bg-slate-800 hover:bg-slate-700 px-8 py-4 rounded-xl font-bold text-lg border border-slate-700">
                        View Marketplace
                    </button>
                </div>
            </header>

            {/* Real-time Stats */}
            <section className="max-w-6xl mx-auto grid grid-cols-1 md:grid-cols-4 gap-6 px-4 py-12">
                <StatCard icon={<Shield className="text-blue-400" />} label="Total NFT Supply" value={stats.totalNFT} />
                <StatCard icon={<Zap className="text-orange-400" />} label="Tokens Burned" value={stats.totalBurned} />
                <StatCard icon={<Swords className="text-red-400" />} label="Reward Distributed" value={stats.totalRewards} />
                <StatCard icon={<Users className="text-green-400" />} label="Active Tournaments" value={stats.activeTournaments} />
            </section>

            {/* Leaderboard Preview */}
            <section className="max-w-4xl mx-auto py-20 px-4">
                <h2 className="text-3xl font-bold mb-8 text-center">Top Arena Warriors</h2>
                <div className="bg-slate-800 rounded-2xl overflow-hidden border border-slate-700">
                    <table className="w-full text-left">
                        <thead className="bg-slate-700/50">
                            <tr>
                                <th className="p-4">Rank</th>
                                <th className="p-4">Warrior</th>
                                <th className="p-4 text-right">MMR</th>
                            </tr>
                        </thead>
                        <tbody>
                            {[1, 2, 3, 4, 5].map((i) => (
                                <tr key={i} className="border-t border-slate-700 hover:bg-slate-700/30 transition-colors">
                                    <td className="p-4 font-bold text-slate-400">#{i}</td>
                                    <td className="p-4 flex items-center gap-3">
                                        <div className="w-8 h-8 bg-slate-600 rounded-full" />
                                        Warrior_{i}
                                    </td>
                                    <td className="p-4 text-right font-mono text-blue-400">2,450</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            </section>
        </div>
    );
};

const StatCard = ({ icon, label, value }) => (
    <div className="bg-slate-800 p-6 rounded-2xl border border-slate-700 shadow-xl">
        <div className="mb-4">{icon}</div>
        <div className="text-slate-400 text-sm mb-1">{label}</div>
        <div className="text-2xl font-bold">{value}</div>
    </div>
);

export default LandingPage;
