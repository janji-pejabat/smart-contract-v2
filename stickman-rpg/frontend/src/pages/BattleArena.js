import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Zap, Heart, Shield, Activity } from 'lucide-react';

const BattleArena = () => {
    const [playerA, setPlayerA] = useState({ hp: 100, maxHp: 100, x: -150, action: 'idle' });
    const [playerB, setPlayerB] = useState({ hp: 100, maxHp: 100, x: 150, action: 'idle' });
    const [combo, setCombo] = useState(0);
    const [damageNumbers, setDamageNumbers] = useState([]);

    // Mock battle loop
    useEffect(() => {
        const interval = setInterval(() => {
            // Randomly simulate an attack
            if (Math.random() > 0.7) {
                triggerAttack('A');
            }
        }, 2000);
        return () => clearInterval(interval);
    }, []);

    const triggerAttack = (who) => {
        const damage = Math.floor(Math.random() * 15) + 5;
        const newDmg = { id: Date.now(), val: damage, x: who === 'A' ? 150 : -150 };
        setDamageNumbers(prev => [...prev, newDmg]);

        if (who === 'A') {
            setPlayerA(p => ({ ...p, action: 'attack' }));
            setTimeout(() => setPlayerA(p => ({ ...p, action: 'idle' })), 300);
            setPlayerB(p => ({ ...p, hp: Math.max(0, p.hp - damage) }));
            setCombo(c => c + 1);
        }

        setTimeout(() => {
            setDamageNumbers(prev => prev.filter(d => d.id !== newDmg.id));
        }, 1000);
    };

    return (
        <div className="bg-slate-950 min-h-screen text-white overflow-hidden relative">
            {/* Top HUD */}
            <div className="absolute top-0 inset-x-0 p-8 flex justify-between items-start z-20">
                <HealthBar side="left" name="VOID WALKER" hp={playerA.hp} max={playerA.maxHp} rank="EX" />

                <div className="flex flex-col items-center gap-4">
                    <div className="bg-slate-900/80 backdrop-blur px-6 py-2 rounded-full border border-slate-700 font-mono text-blue-400 font-bold">
                        ROUND 1 - 01:45
                    </div>
                    {combo > 0 && (
                        <motion.div
                            initial={{ scale: 0.5, opacity: 0 }}
                            animate={{ scale: 1.2, opacity: 1 }}
                            key={combo}
                            className="text-yellow-500 font-black text-4xl italic tracking-tighter"
                        >
                            {combo} HIT COMBO!
                        </motion.div>
                    )}
                </div>

                <HealthBar side="right" name="FIRE SOUL" hp={playerB.hp} max={playerB.maxHp} rank="SSS" />
            </div>

            {/* Battle Ground */}
            <div className="absolute inset-0 flex items-center justify-center pt-20">
                {/* Background Atmosphere */}
                <div className="absolute w-[800px] h-[400px] bg-blue-500/5 blur-[120px] rounded-full" />

                {/* Floor */}
                <div className="absolute bottom-[20%] w-[120%] h-32 bg-slate-900 skew-x-[-45deg] border-t border-slate-800" />

                {/* Players */}
                <div className="relative w-full max-w-4xl h-[400px] flex items-center justify-center">
                    {/* Player A */}
                    <StickmanModel side="left" pos={playerA.x} action={playerA.action} />

                    {/* Player B */}
                    <StickmanModel side="right" pos={playerB.x} action={playerB.action} isOpponent />

                    {/* Damage Numbers */}
                    <AnimatePresence>
                        {damageNumbers.map(d => (
                            <motion.div
                                key={d.id}
                                initial={{ y: 0, opacity: 1 }}
                                animate={{ y: -100, opacity: 0 }}
                                exit={{ opacity: 0 }}
                                className="absolute font-black text-3xl text-red-500 italic z-30"
                                style={{ left: `calc(50% + ${d.x}px)` }}
                            >
                                -{d.val}
                            </motion.div>
                        ))}
                    </AnimatePresence>
                </div>
            </div>

            {/* Bottom Controls (Mock) */}
            <div className="absolute bottom-8 inset-x-0 flex justify-center gap-6 z-20">
                <SkillButton icon={<Zap />} label="Dash" cd="5s" keybind="Q" color="blue" />
                <SkillButton icon={<Activity />} label="Combo" cd="8s" keybind="W" color="red" />
                <SkillButton icon={<Shield />} label="Guard" cd="3s" keybind="E" color="green" />
            </div>

            {/* Network / Ping */}
            <div className="absolute bottom-4 right-4 text-[10px] font-bold text-slate-500 flex items-center gap-2">
                <div className="w-2 h-2 bg-green-500 rounded-full" /> PAXI MAINNET | 42ms
            </div>
        </div>
    );
};

const HealthBar = ({ side, name, hp, max, rank }) => (
    <div className={`w-80 ${side === 'right' ? 'text-right' : 'text-left'}`}>
        <div className={`flex items-center gap-2 mb-2 ${side === 'right' ? 'flex-row-reverse' : ''}`}>
            <div className="text-xl font-black italic">{name}</div>
            <span className="bg-slate-800 border border-slate-700 px-2 py-0.5 rounded text-[10px] font-bold">{rank}</span>
        </div>
        <div className="w-full bg-slate-900 h-6 rounded-lg border border-slate-800 overflow-hidden p-1 shadow-inner">
            <motion.div
                initial={{ width: "100%" }}
                animate={{ width: `${(hp/max)*100}%` }}
                className={`h-full rounded-sm ${hp < 30 ? 'bg-red-500 shadow-[0_0_10px_rgba(239,68,68,0.5)]' : 'bg-blue-500 shadow-[0_0_10px_rgba(59,130,246,0.5)]'}`}
            />
        </div>
        <div className="text-[10px] font-black text-slate-500 mt-1 uppercase tracking-widest">{hp} / {max} HP</div>
    </div>
);

const StickmanModel = ({ side, pos, action, isOpponent }) => (
    <motion.div
        animate={{
            x: pos,
            scaleX: side === 'right' ? -1 : 1,
            rotate: action === 'attack' ? (side === 'left' ? 15 : -15) : 0
        }}
        className="absolute w-32 h-64 flex flex-col items-center justify-center"
    >
        {/* Simple Stickman Figure */}
        <div className={`w-12 h-12 border-4 ${isOpponent ? 'border-red-500' : 'border-white'} rounded-full mb-2`} />
        <div className={`w-1 h-20 ${isOpponent ? 'bg-red-500' : 'bg-white'} rounded-full`} />
        <div className="absolute top-20 flex gap-12">
            <div className={`w-1 h-16 ${isOpponent ? 'bg-red-500' : 'bg-white'} rounded-full rotate-45`} />
            <div className={`w-1 h-16 ${isOpponent ? 'bg-red-500' : 'bg-white'} rounded-full -rotate-45`} />
        </div>
        <div className="absolute bottom-0 flex gap-10">
            <div className={`w-1 h-20 ${isOpponent ? 'bg-red-500' : 'bg-white'} rounded-full rotate-12`} />
            <div className={`w-1 h-20 ${isOpponent ? 'bg-red-500' : 'bg-white'} rounded-full -rotate-12`} />
        </div>
    </motion.div>
);

const SkillButton = ({ icon, label, cd, keybind, color }) => {
    const colorClasses = {
        blue: 'border-blue-500/30 text-blue-400 hover:bg-blue-500/10',
        red: 'border-red-500/30 text-red-400 hover:bg-red-500/10',
        green: 'border-green-500/30 text-green-400 hover:bg-green-500/10'
    };

    return (
        <div className={`w-20 h-24 bg-slate-900 border-2 rounded-2xl flex flex-col items-center justify-center gap-1 cursor-pointer transition-all active:scale-95 ${colorClasses[color]}`}>
            {icon}
            <span className="text-[10px] font-black uppercase">{label}</span>
            <div className="mt-1 px-1.5 py-0.5 bg-slate-800 rounded text-[8px] font-bold text-slate-500">{keybind}</div>
        </div>
    );
};

export default BattleArena;
