import React from 'react';
import { Shield, Sword, Zap, Heart, TrendingUp, ChevronRight } from 'lucide-react';

const CharacterManagement = () => {
    return (
        <div className="bg-slate-900 min-h-screen text-white p-6">
            <div className="max-w-6xl mx-auto">
                <header className="mb-10">
                    <h1 className="text-4xl font-black mb-2 tracking-tight uppercase">My Stickman</h1>
                    <p className="text-slate-400 font-medium">Manage your roster and equipment</p>
                </header>

                <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                    {/* Character Preview & Equipment */}
                    <div className="lg:col-span-2 space-y-8">
                        <div className="bg-slate-800 rounded-3xl p-10 border border-slate-700 flex flex-col items-center justify-center relative min-h-[500px]">
                            {/* Equipment Slots */}
                            <Slot position="top-10 left-10" icon={<Shield size={24} />} label="Head" />
                            <Slot position="top-10 right-10" icon={<Sword size={24} />} label="Weapon" />
                            <Slot position="bottom-10 left-10" icon={<Shield size={24} />} label="Body" />
                            <Slot position="bottom-10 right-10" icon={<Zap size={24} />} label="Accessory" />

                            {/* Stickman Character */}
                            <div className="relative w-64 h-64 flex flex-col items-center justify-center scale-125">
                                <div className="w-20 h-20 border-8 border-white rounded-full mb-3" />
                                <div className="w-2 h-32 bg-white rounded-full" />
                                <div className="flex gap-20 -mt-24">
                                    <div className="w-2 h-24 bg-white rounded-full rotate-[30deg]" />
                                    <div className="w-2 h-24 bg-white rounded-full -rotate-[30deg]" />
                                </div>
                                <div className="flex gap-16 -mt-4">
                                    <div className="w-2 h-28 bg-white rounded-full rotate-[15deg]" />
                                    <div className="w-2 h-28 bg-white rounded-full -rotate-[15deg]" />
                                </div>
                            </div>

                            <div className="mt-12 text-center">
                                <h2 className="text-3xl font-black italic mb-2">VOID WALKER #001</h2>
                                <div className="flex items-center justify-center gap-4">
                                    <span className="bg-blue-600 px-4 py-1 rounded-full text-xs font-black">RANK EX</span>
                                    <span className="text-slate-400 font-bold uppercase tracking-widest text-sm">Level 185</span>
                                </div>
                            </div>
                        </div>

                        {/* Rank & Level Progress */}
                        <div className="bg-slate-800 rounded-3xl p-8 border border-slate-700">
                            <h3 className="text-xl font-bold mb-6 flex items-center gap-2">
                                <TrendingUp className="text-blue-400" /> Rank Progress
                            </h3>
                            <div className="space-y-6">
                                <div>
                                    <div className="flex justify-between mb-2">
                                        <span className="text-sm font-bold text-slate-400">EXP Progress (85%)</span>
                                        <span className="text-sm font-bold">12,500 / 15,000</span>
                                    </div>
                                    <div className="w-full bg-slate-900 h-4 rounded-full overflow-hidden border border-slate-700">
                                        <div className="bg-blue-500 h-full w-[85%] shadow-[0_0_15px_rgba(59,130,246,0.5)]" />
                                    </div>
                                </div>
                                <div className="flex items-center justify-between p-6 bg-blue-500/10 border border-blue-500/20 rounded-2xl">
                                    <div>
                                        <div className="font-black text-xl mb-1">NEXT RANK: GOD TIER</div>
                                        <p className="text-sm text-blue-400 font-medium">Reach level 200 and burn 5,000 PRC20 to ascend.</p>
                                    </div>
                                    <button className="bg-blue-600 hover:bg-blue-700 p-3 rounded-xl transition-all">
                                        <ChevronRight />
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Stats & Skills */}
                    <div className="space-y-8">
                        <div className="bg-slate-800 rounded-3xl p-8 border border-slate-700">
                            <h3 className="text-xl font-bold mb-6 uppercase tracking-tight">Base Statistics</h3>
                            <div className="space-y-4">
                                <StatItem icon={<Heart className="text-red-500" />} label="Health" value="12,500" />
                                <StatItem icon={<Sword className="text-orange-500" />} label="Attack" value="2,450" />
                                <StatItem icon={<Shield className="text-blue-500" />} label="Defense" value="840" />
                                <StatItem icon={<Zap className="text-yellow-500" />} label="Speed" value="145%" />
                            </div>
                        </div>

                        <div className="bg-slate-800 rounded-3xl p-8 border border-slate-700">
                            <h3 className="text-xl font-bold mb-6 uppercase tracking-tight">Active Skills</h3>
                            <div className="space-y-4">
                                <div className="p-4 bg-slate-900 rounded-2xl border border-slate-700 flex gap-4 items-center group hover:border-blue-500 transition-colors cursor-pointer">
                                    <div className="w-12 h-12 bg-blue-500/20 text-blue-400 rounded-xl flex items-center justify-center">
                                        <Zap size={24} />
                                    </div>
                                    <div>
                                        <div className="font-bold">Lightning Dash</div>
                                        <div className="text-xs text-slate-500 font-bold uppercase">CD: 5s | MULT: 1.8x</div>
                                    </div>
                                </div>
                                <div className="p-4 bg-slate-900 rounded-2xl border border-slate-700 flex gap-4 items-center group hover:border-blue-500 transition-colors cursor-pointer">
                                    <div className="w-12 h-12 bg-red-500/20 text-red-400 rounded-xl flex items-center justify-center">
                                        <Sword size={24} />
                                    </div>
                                    <div>
                                        <div className="font-bold">Berserk Strike</div>
                                        <div className="text-xs text-slate-500 font-bold uppercase">CD: 12s | MULT: 2.5x</div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

const Slot = ({ position, icon, label }) => (
    <div className={`absolute ${position} flex flex-col items-center gap-2 group`}>
        <div className="w-16 h-16 bg-slate-900 border-2 border-slate-700 rounded-2xl flex items-center justify-center text-slate-500 group-hover:border-blue-500 group-hover:text-blue-500 transition-all shadow-lg cursor-pointer">
            {icon}
        </div>
        <span className="text-[10px] font-black uppercase tracking-widest text-slate-500">{label}</span>
    </div>
);

const StatItem = ({ icon, label, value }) => (
    <div className="flex justify-between items-center p-4 bg-slate-900 rounded-2xl border border-slate-700">
        <div className="flex items-center gap-3">
            {icon}
            <span className="font-bold text-slate-400">{label}</span>
        </div>
        <span className="font-black text-lg">{value}</span>
    </div>
);

export default CharacterManagement;
