/**
 * Paxi LP Locker DApp Logic
 * Integrates with Paxi Network using PaxiCosmJS
 */

// --- Configuration ---
const CONFIG = {
    rpc: 'https://mainnet-rpc.paxinet.io',
    lcd: 'https://mainnet-lcd.paxinet.io',
    denom: 'upaxi',
    chainId: 'paxi-1', // Update based on actual chain
    contracts: {
        locker: 'paxi1...', // PLACEHOLDER: LP Locker Contract Address
        reward: 'paxi1...'  // PLACEHOLDER: Reward Controller Contract Address
    }
};

// --- State ---
let userAddress = null;
let isConnecting = false;

// --- Helpers ---
const toBase64 = bytes => btoa(String.fromCharCode(...bytes));

async function getAccountInfo(address) {
    try {
        const res = await fetch(`${CONFIG.lcd}/cosmos/auth/v1beta1/accounts/${address}`);
        const data = await res.json();
        const ba = data.account.base_account || data.account;
        return {
            accountNumber: Number(ba.account_number),
            sequence: Number(ba.sequence)
        };
    } catch (e) {
        console.error('Error fetching account info:', e);
        return { accountNumber: 0, sequence: 0 };
    }
}

// --- Core Functions ---

/**
 * Connect to PaxiHub Wallet
 */
async function connectWallet() {
    if (typeof window.paxihub === 'undefined') {
        if (/Mobi/.test(navigator.userAgent)) {
            window.location.href = `paxi://hub/explorer?url=${encodeURIComponent(window.location.href)}`;
        } else {
            alert('Please install PaxiHub wallet extension or use the mobile app.');
        }
        return;
    }

    try {
        isConnecting = true;
        const btn = document.getElementById('connectBtn');
        if (btn) btn.textContent = 'Connecting...';

        const sender = await window.paxihub.paxi.getAddress();
        userAddress = sender.address;

        console.log('Connected:', userAddress);

        if (btn) {
            btn.textContent = userAddress.substring(0, 8) + '...' + userAddress.substring(userAddress.length - 4);
            btn.classList.add('bg-green-600');
        }

        // Initialize App Data
        refreshAppData();
    } catch (e) {
        console.error('Connection failed:', e);
        alert('Failed to connect wallet.');
        if (btn) btn.textContent = 'Connect Wallet';
    } finally {
        isConnecting = false;
    }
}

/**
 * Refresh all dashboard data
 */
async function refreshAppData() {
    if (!userAddress) return;

    await Promise.all([
        updateUserLockers(),
        updatePendingRewards(),
        updateGlobalStats()
    ]);
}

/**
 * Query user's active lockers
 */
async function updateUserLockers() {
    const listEl = document.getElementById('lockersList');
    if (!listEl) return;

    try {
        const queryMsg = {
            lockers_by_owner: {
                owner: userAddress,
                limit: 10
            }
        };
        const res = await queryContract(CONFIG.contracts.locker, queryMsg);

        if (res && res.lockers && res.lockers.length > 0) {
            listEl.innerHTML = res.lockers.map(locker => `
                <tr class="text-white text-sm border-b border-slate-800/50 hover:bg-slate-800/30 transition">
                    <td class="py-4 font-mono">#${locker.id}</td>
                    <td class="py-4">${locker.lp_token.substring(0, 10)}...</td>
                    <td class="py-4 font-bold">${(locker.amount / 1e6).toFixed(2)} LP</td>
                    <td class="py-4 text-slate-400">${new Date(locker.unlock_time * 1000).toLocaleDateString()}</td>
                    <td class="py-4 text-right">
                        <button onclick="unlockLP(${locker.id})" class="text-blue-500 hover:underline">Unlock</button>
                    </td>
                </tr>
            `).join('');

            // Update summary
            const total = res.lockers.reduce((acc, curr) => acc + Number(curr.amount), 0);
            document.getElementById('userLockedDisplay').textContent = (total / 1e6).toFixed(2) + ' LP';
        } else {
            listEl.innerHTML = '<tr><td colspan="5" class="py-12 text-center text-slate-500">No active lockers found</td></tr>';
        }
    } catch (e) {
        console.error('Error fetching lockers:', e);
    }
}

/**
 * Query pending rewards
 */
async function updatePendingRewards() {
    const rewardsEl = document.getElementById('rewardsList');
    if (!rewardsEl) return;

    try {
        // This usually requires iterating over whitelisted pools
        // For simplicity, we query a specific pool or use AllRewardPools
        const poolsRes = await queryContract(CONFIG.contracts.reward, { all_reward_pools: {} });

        if (poolsRes && poolsRes.length > 0) {
            let totalRewards = 0;
            const rewardItems = [];

            for (const pool of poolsRes) {
                const pendingRes = await queryContract(CONFIG.contracts.reward, {
                    pending_rewards: {
                        user: userAddress,
                        pool_id: pool.pool_id
                    }
                });

                if (pendingRes && Number(pendingRes.pending_amount) > 0) {
                    const amount = Number(pendingRes.pending_amount) / 1e6;
                    totalRewards += amount;
                    rewardItems.push(`
                        <div class="flex justify-between items-center p-4 bg-slate-900/50 rounded-2xl border border-slate-800">
                            <div>
                                <p class="text-xs text-slate-500">Pool #${pool.pool_id}</p>
                                <p class="font-bold">${amount.toFixed(4)} PAXI</p>
                            </div>
                            <button onclick="claimRewards(${pool.pool_id})" class="bg-blue-600/20 text-blue-400 px-4 py-2 rounded-xl text-sm font-bold hover:bg-blue-600 transition">
                                Claim
                            </button>
                        </div>
                    `);
                }
            }

            if (rewardItems.length > 0) {
                rewardsEl.innerHTML = rewardItems.join('');
                document.getElementById('rewardsDisplay').textContent = totalRewards.toFixed(2) + ' PAXI';
            } else {
                rewardsEl.innerHTML = '<p class="text-slate-500 text-center py-8">No rewards available to claim</p>';
            }
        }
    } catch (e) {
        console.error('Error fetching rewards:', e);
    }
}

/**
 * Lock LP tokens
 */
async function lockLP() {
    const lpToken = document.getElementById('lpTokenAddr').value;
    const amount = document.getElementById('lockAmount').value;
    const unlockDate = document.getElementById('unlockDate').value;

    if (!lpToken || !amount || !unlockDate) {
        alert('Please fill all fields');
        return;
    }

    const unlockTimestamp = Math.floor(new Date(unlockDate).getTime() / 1000);
    const amountU128 = (parseFloat(amount) * 1e6).toString();

    try {
        const lockMsg = {
            lock_lp: {
                unlock_time: unlockTimestamp
            }
        };

        const msgExecute = {
            send: {
                contract: CONFIG.contracts.locker,
                amount: amountU128,
                msg: btoa(JSON.stringify(lockMsg))
            }
        };

        const msg = PaxiCosmJS.MsgExecuteContract.fromPartial({
            sender: userAddress,
            contract: lpToken,
            msg: new TextEncoder().encode(JSON.stringify(msgExecute))
        });

        const anyMsg = PaxiCosmJS.Any.fromPartial({
            typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
            value: PaxiCosmJS.MsgExecuteContract.encode(msg).finish()
        });

        await sendTransaction([anyMsg], "Lock LP Tokens");
        alert('Transaction successful!');
        refreshAppData();
    } catch (e) {
        console.error('Lock failed:', e);
        alert('Transaction failed: ' + e.message);
    }
}

/**
 * Claim rewards from a pool
 */
async function claimRewards(poolId) {
    try {
        // We need the locker_id. In v2, rewards are associated with locker_ids.
        // For a full implementation, we'd prompt the user or claim for all lockers.
        const res = await queryContract(CONFIG.contracts.reward, {
            user_stake: { user: userAddress, locker_id: 1 } // Simplified for demo
        });

        const msgExecute = {
            claim_rewards: {
                locker_id: 1, // Placeholder
                pool_ids: [poolId]
            }
        };

        const msg = PaxiCosmJS.MsgExecuteContract.fromPartial({
            sender: userAddress,
            contract: CONFIG.contracts.reward,
            msg: new TextEncoder().encode(JSON.stringify(msgExecute))
        });

        const anyMsg = PaxiCosmJS.Any.fromPartial({
            typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
            value: PaxiCosmJS.MsgExecuteContract.encode(msg).finish()
        });

        await sendTransaction([anyMsg], "Claim Rewards");
        refreshAppData();
    } catch (e) {
        console.error('Claim failed:', e);
    }
}

/**
 * Admin: Whitelist LP Token
 */
async function whitelistToken() {
    const token = document.getElementById('whitelistTokenAddr').value;
    const name = document.getElementById('whitelistTokenName').value;
    const symbol = document.getElementById('whitelistTokenSymbol').value;
    const minDays = document.getElementById('minLockDays').value;
    const maxDays = document.getElementById('maxLockDays').value;

    try {
        const msgObj = {
            whitelist_lp: {
                lp_token: token,
                name: name,
                symbol: symbol,
                min_lock_duration: Number(minDays) * 86400,
                max_lock_duration: Number(maxDays) * 86400,
                bonus_multiplier: "1.0"
            }
        };

        const msg = PaxiCosmJS.MsgExecuteContract.fromPartial({
            sender: userAddress,
            contract: CONFIG.contracts.locker,
            msg: new TextEncoder().encode(JSON.stringify(msgObj))
        });

        const anyMsg = PaxiCosmJS.Any.fromPartial({
            typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
            value: PaxiCosmJS.MsgExecuteContract.encode(msg).finish()
        });

        await sendTransaction([anyMsg], "Whitelist LP Token");
        alert('Whitelisted successfully!');
    } catch (e) {
        console.error('Whitelist failed:', e);
        alert('Failed: ' + e.message);
    }
}

// --- Utils ---

async function queryContract(contract, msg) {
    const queryBase64 = btoa(JSON.stringify(msg));
    const url = `${CONFIG.lcd}/cosmwasm/wasm/v1/contract/${contract}/smart/${queryBase64}`;
    const res = await fetch(url);
    const data = await res.json();
    return data.data;
}

async function sendTransaction(messages, memo = "") {
    const sender = await window.paxihub.paxi.getAddress();
    const { accountNumber, sequence } = await getAccountInfo(sender.address);
    const chainId = await fetch(`${CONFIG.rpc}/status`)
        .then(r => r.json())
        .then(d => d.result.node_info.network);

    const txBody = PaxiCosmJS.TxBody.fromPartial({ messages, memo });
    const fee = {
        amount: [PaxiCosmJS.coins("50000", CONFIG.denom)[0]],
        gasLimit: 800_000
    };

    const pubkeyBytes = new Uint8Array(sender.public_key);
    const pubkeyAny = {
        typeUrl: "/cosmos.crypto.secp256k1.PubKey",
        value: PaxiCosmJS.PubKey.encode({ key: pubkeyBytes }).finish()
    };

    const authInfo = PaxiCosmJS.AuthInfo.fromPartial({
        signerInfos: [{
            publicKey: pubkeyAny,
            modeInfo: { single: { mode: 1 } },
            sequence: BigInt(sequence)
        }],
        fee
    });

    const signDoc = PaxiCosmJS.SignDoc.fromPartial({
        bodyBytes: PaxiCosmJS.TxBody.encode(txBody).finish(),
        authInfoBytes: PaxiCosmJS.AuthInfo.encode(authInfo).finish(),
        chainId,
        accountNumber: BigInt(accountNumber)
    });

    const txObj = {
        bodyBytes: btoa(String.fromCharCode(...signDoc.bodyBytes)),
        authInfoBytes: btoa(String.fromCharCode(...signDoc.authInfoBytes)),
        chainId,
        accountNumber: signDoc.accountNumber.toString()
    };

    const result = await window.paxihub.paxi.signAndSendTransaction(txObj);
    if (!result.success) throw new Error('Transaction signing failed');

    const sigBytes = Uint8Array.from(atob(result.success), c => c.charCodeAt(0));
    const txRaw = PaxiCosmJS.TxRaw.fromPartial({
        bodyBytes: signDoc.bodyBytes,
        authInfoBytes: signDoc.authInfoBytes,
        signatures: [sigBytes]
    });

    const broadcastResult = await fetch(`${CONFIG.lcd}/cosmos/tx/v1beta1/txs`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
            tx_bytes: toBase64(PaxiCosmJS.TxRaw.encode(txRaw).finish()),
            mode: "BROADCAST_MODE_SYNC"
        })
    }).then(r => r.json());

    console.log('Broadcast result:', broadcastResult);
    return broadcastResult;
}

// --- Initialization ---

document.addEventListener('DOMContentLoaded', () => {
    const connectBtn = document.getElementById('connectBtn');
    if (connectBtn) connectBtn.addEventListener('click', connectWallet);

    const lockBtn = document.getElementById('lockBtn');
    if (lockBtn) lockBtn.addEventListener('click', lockLP);

    const whitelistBtn = document.getElementById('whitelistBtn');
    if (whitelistBtn) whitelistBtn.addEventListener('click', whitelistToken);

    // Auto-detect wallet
    if (typeof window.paxihub !== 'undefined') {
        console.log('PaxiHub detected');
    }
});
