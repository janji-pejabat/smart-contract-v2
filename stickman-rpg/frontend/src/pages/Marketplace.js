import React, { useState } from 'react';
import { Filter, Search, Tag, Star, ArrowUpDown } from 'lucide-react';

const Marketplace = () => {
    const [activeTab, setActiveTab] = useState('stickman');
    const [filterRank, setFilterRank] = useState('All');

    const stickmans = [
        { id: 1, name: "Shadow Master", rank: "EX", level: 145, price: "2,500", rarity: "Mythic", color: "blue" },
        { id: 2, name: "Fire Soul", rank: "SSS", level: 110, price: "1,800", rarity: "Legendary", color: "red" },
        { id: 3, name: "Wind Walker", rank: "SS", level: 75, price: "950", rarity: "Epic", color: "green" },
        { id: 4, name: "Stone Heart", rank: "A", level: 55, price: "400", rarity: "Rare", color: "orange" },
    ];

    return (
        <div className="bg-slate-900 min-h-screen text-white p-6">
            <div className="max-w-7xl mx-auto">
                <header className="flex flex-col md:flex-row justify-between items-start md:items-center gap-6 mb-10">
                    <div>
                        <h1 className="text-4xl font-black mb-2 tracking-tight uppercase">NFT Marketplace</h1>
                        <p className="text-slate-400 font-medium">Trade unique Stickman warriors and equipment</p>
                    </div>

                    <div className="flex bg-slate-800 p-1 rounded-2xl border border-slate-700 w-full md:w-auto">
                        <button
                            onClick={() => setActiveTab('stickman')}
                            className={`flex-1 md:flex-none px-8 py-3 rounded-xl font-bold transition-all ${activeTab === 'stickman' ? 'bg-blue-600 shadow-lg shadow-blue-900/20' : 'hover:bg-slate-700'}`}
                        >
                            Stickman
                        </button>
                        <button
                            onClick={() => setActiveTab('cosplay')}
                            className={`flex-1 md:flex-none px-8 py-3 rounded-xl font-bold transition-all ${activeTab === 'cosplay' ? 'bg-blue-600 shadow-lg shadow-blue-900/20' : 'hover:bg-slate-700'}`}
                        >
                            Cosplay
                        </button>
                    </div>
                </header>

                <div className="flex flex-col lg:flex-row gap-8">
                    {/* Sidebar Filters */}
                    <aside className="w-full lg:w-64 space-y-8">
                        <div>
                            <h3 className="text-sm uppercase font-black tracking-widest text-slate-500 mb-4 flex items-center gap-2">
                                <Filter size={16} /> Filters
                            </h3>
                            <div className="space-y-6">
                                <div>
                                    <label className="block text-sm font-bold mb-3">Rank Tier</label>
                                    <div className="grid grid-cols-3 gap-2">
                                        {['EX', 'UR', 'SSS', 'SS', 'A', 'B', 'C', 'D', 'F'].map(rank => (
                                            <button
                                                key={rank}
                                                className="py-2 rounded-lg bg-slate-800 border border-slate-700 text-xs font-bold hover:border-blue-500 transition-colors"
                                            >
                                                {rank}
                                            </button>
                                        ))}
                                    </div>
                                </div>
                                <div>
                                    <label className="block text-sm font-bold mb-3">Price Range (PAXI)</label>
                                    <div className="flex gap-2">
                                        <input type="text" placeholder="Min" className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500" />
                                        <input type="text" placeholder="Max" className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500" />
                                    </div>
                                </div>
                            </div>
                        </div>
                    </aside>

                    {/* Main Content Grid */}
                    <main className="flex-1">
                        <div className="flex justify-between items-center mb-6">
                            <div className="relative flex-1 max-w-md">
                                <Search className="absolute left-4 top-1/2 -translate-y-1/2 text-slate-500" size={18} />
                                <input
                                    type="text"
                                    placeholder="Search by name or ID..."
                                    className="w-full bg-slate-800 border border-slate-700 rounded-2xl pl-12 pr-4 py-3 text-sm focus:outline-none focus:border-blue-500"
                                />
                            </div>
                            <button className="ml-4 p-3 bg-slate-800 border border-slate-700 rounded-2xl hover:bg-slate-700 transition-colors">
                                <ArrowUpDown size={18} />
                            </button>
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-6">
                            {stickmans.map(nft => (
                                <div key={nft.id} className="bg-slate-800 rounded-3xl border border-slate-700 overflow-hidden hover:border-blue-500/50 transition-all group cursor-pointer shadow-xl">
                                    <div className="aspect-square bg-slate-900 relative p-8 flex items-center justify-center overflow-hidden">
                                        {/* Rank Glow Effect */}
                                        <div className={`absolute inset-0 opacity-10 bg-gradient-to-br from-${nft.color}-500 to-transparent`} />

                                        {/* Stickman Figure (Mock) */}
                                        <div className="relative z-10 w-48 h-48 flex flex-col items-center justify-center grayscale group-hover:grayscale-0 transition-all">
                                            <div className="w-16 h-16 border-4 border-white rounded-full mb-2" />
                                            <div className="w-1 h-24 bg-white rounded-full" />
                                            <div className="flex gap-12 -mt-20">
                                                <div className="w-1 h-20 bg-white rounded-full rotate-45" />
                                                <div className="w-1 h-20 bg-white rounded-full -rotate-45" />
                                            </div>
                                        </div>

                                        <div className="absolute top-4 right-4 bg-slate-900/80 backdrop-blur px-3 py-1 rounded-full border border-slate-700 flex items-center gap-1">
                                            <Star size={14} className="text-yellow-400 fill-yellow-400" />
                                            <span className="text-xs font-black uppercase">{nft.rarity}</span>
                                        </div>

                                        <div className="absolute bottom-4 left-4 bg-blue-600 px-3 py-1 rounded-lg text-xs font-black italic">
                                            RANK {nft.rank}
                                        </div>
                                    </div>

                                    <div className="p-5">
                                        <div className="flex justify-between items-start mb-4">
                                            <div>
                                                <h4 className="font-bold text-lg mb-1">{nft.name}</h4>
                                                <p className="text-slate-400 text-xs font-bold uppercase tracking-widest">Level {nft.level}</p>
                                            </div>
                                            <div className="text-right">
                                                <div className="text-blue-400 font-black text-xl">{nft.price}</div>
                                                <div className="text-[10px] text-slate-500 uppercase font-bold">PAXI</div>
                                            </div>
                                        </div>

                                        <button className="w-full bg-slate-700 hover:bg-blue-600 py-3 rounded-xl font-bold transition-all flex items-center justify-center gap-2">
                                            <Tag size={18} /> Buy Now
                                        </button>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </main>
                </div>
            </div>
        </div>
    );
};

export default Marketplace;
