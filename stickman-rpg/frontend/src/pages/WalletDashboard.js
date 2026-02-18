import React, { useState, useEffect } from 'react';
import { Wallet, ArrowDownCircle, ArrowUpCircle, History, ExternalLink } from 'lucide-react';
import BlockchainService from '../services/BlockchainService';

const WalletDashboard = () => {
    const [account, setAccount] = useState(null);
    const [balances, setBalances] = useState([]);
    const [loading, setLoading] = useState(false);

    const connect = async () => {
        setLoading(true);
        const res = await BlockchainService.connectWallet();
        if (res) {
            setAccount(res.address);
            const bal = await BlockchainService.getBalance(res.address);
            setBalances(bal);
        }
        setLoading(false);
    };

    return (
        <div className="bg-slate-900 min-h-screen text-white p-6">
            <div className="max-w-4xl mx-auto">
                <div className="flex justify-between items-center mb-10">
                    <h1 className="text-3xl font-bold flex items-center gap-3">
                        <Wallet className="text-blue-500" /> Wallet Dashboard
                    </h1>
                    {!account ? (
                        <button
                            onClick={connect}
                            className="bg-blue-600 hover:bg-blue-700 px-6 py-2 rounded-lg font-bold transition-all"
                            disabled={loading}
                        >
                            {loading ? "Connecting..." : "Connect PaxiHub"}
                        </button>
                    ) : (
                        <div className="bg-slate-800 px-4 py-2 rounded-lg border border-slate-700 font-mono text-sm">
                            {account.substring(0, 8)}...{account.substring(account.length - 4)}
                        </div>
                    )}
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
                    {/* Balances Section */}
                    <div className="space-y-6">
                        <div className="bg-slate-800 p-8 rounded-3xl border border-slate-700 shadow-2xl relative overflow-hidden">
                            <div className="absolute top-0 right-0 p-4 opacity-10">
                                <Wallet size={120} />
                            </div>
                            <div className="relative z-10">
                                <h3 className="text-slate-400 mb-2 uppercase tracking-widest text-xs font-bold">Total Assets</h3>
                                <div className="space-y-4">
                                    <div className="flex justify-between items-end">
                                        <div>
                                            <span className="text-4xl font-black">1,250.00</span>
                                            <span className="text-blue-500 ml-2 font-bold">PAXI</span>
                                        </div>
                                    </div>
                                    <div className="flex justify-between items-end">
                                        <div>
                                            <span className="text-2xl font-black">50,000</span>
                                            <span className="text-orange-500 ml-2 font-bold">PRC20</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>

                        <div className="bg-slate-800 p-6 rounded-3xl border border-slate-700">
                            <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
                                <History className="text-slate-400" size={20} /> Transaction History
                            </h3>
                            <div className="space-y-4">
                                {[1, 2, 3].map((i) => (
                                    <div key={i} className="flex justify-between items-center p-3 hover:bg-slate-700/50 rounded-xl transition-colors cursor-pointer group">
                                        <div className="flex items-center gap-3">
                                            <div className="p-2 bg-green-500/20 text-green-500 rounded-lg">
                                                <ArrowDownCircle size={18} />
                                            </div>
                                            <div>
                                                <div className="font-bold">Deposit</div>
                                                <div className="text-xs text-slate-500">2026-10-15 14:22</div>
                                            </div>
                                        </div>
                                        <div className="text-right">
                                            <div className="font-bold text-green-400">+500 PAXI</div>
                                            <ExternalLink size={14} className="ml-auto text-slate-600 group-hover:text-blue-400" />
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </div>
                    </div>

                    {/* Actions Section */}
                    <div className="bg-slate-800 p-8 rounded-3xl border border-slate-700">
                        <div className="flex gap-2 mb-8 p-1 bg-slate-900 rounded-xl">
                            <button className="flex-1 py-2 rounded-lg bg-blue-600 font-bold">Deposit</button>
                            <button className="flex-1 py-2 rounded-lg hover:bg-slate-700 font-bold transition-colors">Withdraw</button>
                        </div>

                        <div className="space-y-6">
                            <div>
                                <label className="block text-slate-400 text-sm mb-2 font-bold">Your Unique Deposit Address</label>
                                <div className="bg-slate-900 p-4 rounded-xl border border-slate-700 break-all font-mono text-xs text-blue-300">
                                    paxi1qpdz7v8z2z2z2z2z2z2z2z2z2z2z2z2z2z2z2z
                                </div>
                                <button className="w-full mt-4 bg-slate-700 hover:bg-slate-600 py-3 rounded-xl font-bold transition-colors">
                                    Copy Address
                                </button>
                            </div>

                            <div className="pt-6 border-t border-slate-700">
                                <div className="bg-blue-500/10 border border-blue-500/20 p-4 rounded-xl">
                                    <p className="text-sm text-blue-400 leading-relaxed">
                                        <strong>Pro Tip:</strong> All deposits are verified via Paxi LCD REST API.
                                        Please wait for 1-2 minutes for the balance to reflect.
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default WalletDashboard;
