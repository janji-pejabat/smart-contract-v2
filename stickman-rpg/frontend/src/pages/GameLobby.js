import React, { useState } from 'react';
import { Trophy, Swords, Clock, User, Flame, Award } from 'lucide-react';

const GameLobby = () => {
    const [queueing, setQueueing] = useState(false);

    return (
        <div className="bg-slate-900 min-h-screen text-white p-6">
            <div className="max-w-6xl mx-auto">
                <header className="flex justify-between items-end mb-12">
                    <div>
                        <h1 className="text-4xl font-black mb-2 tracking-tight uppercase italic">Battle Lobby</h1>
                        <p className="text-slate-400 font-medium">Prepare for the next challenge</p>
                    </div>
                    <div className="text-right">
                        <div className="text-sm font-black text-slate-500 uppercase tracking-widest mb-1">Your Arena Rating</div>
                        <div className="text-3xl font-black text-blue-500 italic">2,450 MMR</div>
                    </div>
                </header>

                <div className="grid grid-cols-1 lg:grid-cols-2 gap-10">
                    {/* Ranked Mode */}
                    <div className="bg-gradient-to-br from-blue-600/20 to-slate-800 p-1 rounded-3xl border border-blue-500/20 shadow-2xl overflow-hidden group">
                        <div className="bg-slate-800/80 backdrop-blur-xl p-10 rounded-[calc(1.5rem-1px)] h-full relative overflow-hidden">
                            <div className="absolute -top-20 -right-20 w-64 h-64 bg-blue-500/10 rounded-full blur-3xl group-hover:bg-blue-500/20 transition-all" />

                            <div className="relative z-10">
                                <div className="p-4 bg-blue-500/20 rounded-2xl w-fit mb-6 text-blue-400">
                                    <Swords size={40} />
                                </div>
                                <h2 className="text-3xl font-black mb-4 italic uppercase">Ranked Matchmaking</h2>
                                <p className="text-slate-400 mb-8 leading-relaxed font-medium">
                                    Compete against players of similar skill level. Gain MMR and climb the global leaderboard to earn seasonal rewards.
                                </p>

                                <div className="space-y-4 mb-10">
                                    <div className="flex items-center gap-4 text-sm font-bold text-slate-300 bg-slate-900/50 p-3 rounded-xl border border-slate-700">
                                        <Clock size={18} className="text-blue-400" /> Est. Wait Time: ~45s
                                    </div>
                                    <div className="flex items-center gap-4 text-sm font-bold text-slate-300 bg-slate-900/50 p-3 rounded-xl border border-slate-700">
                                        <Award size={18} className="text-yellow-400" /> Reward: 10 - 50 PAXI per win
                                    </div>
                                </div>

                                <button
                                    onClick={() => setQueueing(!queueing)}
                                    className={`w-full py-5 rounded-2xl font-black text-xl tracking-tighter uppercase transition-all shadow-xl ${queueing ? 'bg-red-600 hover:bg-red-700' : 'bg-blue-600 hover:bg-blue-700'}`}
                                >
                                    {queueing ? 'Cancel Queue' : 'Enter Arena'}
                                </button>

                                {queueing && (
                                    <div className="mt-4 flex items-center justify-center gap-2 text-blue-400 font-bold animate-pulse">
                                        <div className="w-2 h-2 bg-blue-400 rounded-full" /> Searching for opponent...
                                    </div>
                                )}
                            </div>
                        </div>
                    </div>

                    {/* Tournament Mode */}
                    <div className="bg-gradient-to-br from-orange-600/20 to-slate-800 p-1 rounded-3xl border border-orange-500/20 shadow-2xl overflow-hidden group">
                        <div className="bg-slate-800/80 backdrop-blur-xl p-10 rounded-[calc(1.5rem-1px)] h-full relative overflow-hidden">
                            <div className="absolute -top-20 -right-20 w-64 h-64 bg-orange-500/10 rounded-full blur-3xl group-hover:bg-orange-500/20 transition-all" />

                            <div className="relative z-10">
                                <div className="p-4 bg-orange-500/20 rounded-2xl w-fit mb-6 text-orange-400">
                                    <Trophy size={40} />
                                </div>
                                <h2 className="text-3xl font-black mb-4 italic uppercase">Paxi Grand Slam</h2>
                                <p className="text-slate-400 mb-8 leading-relaxed font-medium">
                                    Join massive tournaments with huge prize pools. Bracket-style elimination where only the strongest survive.
                                </p>

                                <div className="bg-slate-900/80 p-6 rounded-2xl border border-slate-700 mb-10">
                                    <div className="flex justify-between items-center mb-4">
                                        <div className="text-xs font-black text-slate-500 uppercase tracking-widest">Prize Pool</div>
                                        <div className="text-2xl font-black text-orange-500">25,000 PAXI</div>
                                    </div>
                                    <div className="space-y-3">
                                        <div className="flex justify-between text-sm">
                                            <span className="text-slate-400 font-bold">Participants</span>
                                            <span className="font-black">48 / 64</span>
                                        </div>
                                        <div className="w-full bg-slate-800 h-2 rounded-full overflow-hidden">
                                            <div className="bg-orange-500 h-full w-[75%]" />
                                        </div>
                                        <div className="flex justify-between text-sm mt-2">
                                            <span className="text-slate-400 font-bold">Entry Fee</span>
                                            <span className="font-black text-blue-400">100 PRC20</span>
                                        </div>
                                    </div>
                                </div>

                                <button className="w-full py-5 rounded-2xl bg-orange-600 hover:bg-orange-700 font-black text-xl tracking-tighter uppercase transition-all shadow-xl">
                                    Join Tournament
                                </button>
                            </div>
                        </div>
                    </div>
                </div>

                {/* Quick News / Events */}
                <div className="mt-12 grid grid-cols-1 md:grid-cols-3 gap-6">
                    <div className="bg-slate-800 p-6 rounded-2xl border border-slate-700 flex items-center gap-4">
                        <div className="p-3 bg-red-500/20 text-red-500 rounded-xl"><Flame /></div>
                        <div>
                            <div className="font-bold">Hot Event</div>
                            <div className="text-xs text-slate-400 font-medium">Double EXP Weekend</div>
                        </div>
                    </div>
                    <div className="bg-slate-800 p-6 rounded-2xl border border-slate-700 flex items-center gap-4">
                        <div className="p-3 bg-purple-500/20 text-purple-500 rounded-xl"><Clock /></div>
                        <div>
                            <div className="font-bold">Season Ends</div>
                            <div className="text-xs text-slate-400 font-medium">12 Days 04:22:15</div>
                        </div>
                    </div>
                    <div className="bg-slate-800 p-6 rounded-2xl border border-slate-700 flex items-center gap-4">
                        <div className="p-3 bg-green-500/20 text-green-500 rounded-xl"><User /></div>
                        <div>
                            <div className="font-bold">Online Players</div>
                            <div className="text-xs text-slate-400 font-medium">1,422 Stickmen</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default GameLobby;
