// Configuration
const RPC = 'https://mainnet-rpc.paxinet.io';
const LCD = 'https://mainnet-lcd.paxinet.io';
const DENOM = 'upaxi';
const VESTING_CONTRACT = "paxi19h08k9v2x4r7z7s99unwuxsmnvj5853vst5pnd9m6xntnvepg0dqj8h52s"; // Placeholder

// Global State
let userAddress = null;
let userBalance = "0";
let activeTab = 'my-vestings';
let vestingType = 'linear';
let vestings = [];

// DOM Elements
const btnConnect = document.getElementById('btn-connect');
const walletInfo = document.getElementById('wallet-info');
const userAddressEl = document.getElementById('user-address');
const userBalanceEl = document.getElementById('user-balance');
const vestingListEl = document.getElementById('vesting-list');
const noVestingsEl = document.getElementById('no-vestings');
const createVestingForm = document.getElementById('create-vesting-form');

// Initialization
document.addEventListener('DOMContentLoaded', () => {
    initWallet();
    setupEventListeners();
});

function setupEventListeners() {
    btnConnect.addEventListener('click', connectWallet);
    createVestingForm.addEventListener('submit', handleCreateVesting);
}

// --- Wallet & Connection ---

async function initWallet() {
    if (typeof window.paxihub !== 'undefined') {
        console.log("PaxiHub detected");
        const addr = await window.paxihub.paxi.getAddress();
        if (addr && addr.address) {
            userAddress = addr.address;
            updateWalletUI();
            refreshData();
        }
    }
}

async function connectWallet() {
    if (typeof window.paxihub === 'undefined') {
        if (/Mobi/.test(navigator.userAgent)) {
            window.location.href = `paxi://hub/explorer?url=${encodeURIComponent(window.location.href)}`;
            return;
        }
        alert("PaxiHub wallet not detected. Please install it on mobile.");
        return;
    }

    try {
        const addr = await window.paxihub.paxi.getAddress();
        userAddress = addr.address;
        updateWalletUI();
        refreshData();
    } catch (e) {
        console.error("Connection failed", e);
    }
}

function updateWalletUI() {
    if (userAddress) {
        btnConnect.classList.add('hidden');
        walletInfo.classList.remove('hidden');
        userAddressEl.textContent = shortenAddress(userAddress);
        fetchBalance();
    }
}

async function fetchBalance() {
    try {
        const res = await fetch(`${LCD}/cosmos/bank/v1beta1/balances/${userAddress}/by_denom?denom=${DENOM}`);
        const data = await res.json();
        userBalance = (parseFloat(data.balance.amount) / 1_000_000).toFixed(2);
        userBalanceEl.textContent = `${userBalance} PAXI`;
    } catch (e) {
        console.error("Fetch balance failed", e);
    }
}

// --- Navigation ---

function switchTab(tab) {
    activeTab = tab;
    document.getElementById('tab-my-vestings').className = tab === 'my-vestings' ? 'tab-active py-5 px-1 font-medium transition-colors' : 'py-5 px-1 font-medium text-gray-400 hover:text-white transition-colors';
    document.getElementById('tab-admin-panel').className = tab === 'admin-panel' ? 'tab-active py-5 px-1 font-medium transition-colors' : 'py-5 px-1 font-medium text-gray-400 hover:text-white transition-colors';

    document.getElementById('section-my-vestings').classList.toggle('hidden', tab !== 'my-vestings');
    document.getElementById('section-admin-panel').classList.toggle('hidden', tab !== 'admin-panel');

    if (tab === 'admin-panel') {
        fetchAdminData();
    }
}

function setVestingType(type) {
    vestingType = type;
    document.getElementById('btn-type-linear').className = type === 'linear' ? 'px-4 py-2 rounded-lg bg-dark-600 text-white text-sm font-medium transition-all shadow-lg' : 'px-4 py-2 rounded-lg text-gray-400 text-sm font-medium hover:text-white transition-all';
    document.getElementById('btn-type-custom').className = type === 'custom' ? 'px-4 py-2 rounded-lg bg-dark-600 text-white text-sm font-medium transition-all shadow-lg' : 'px-4 py-2 rounded-lg text-gray-400 text-sm font-medium hover:text-white transition-all';

    document.getElementById('linear-config').classList.toggle('hidden', type !== 'linear');
    document.getElementById('custom-config').classList.toggle('hidden', type !== 'custom');
}

// --- Data Fetching ---

async function refreshData() {
    if (!userAddress) return;

    vestingListEl.innerHTML = '';
    showLoader();

    try {
        const query = {
            vestings_by_beneficiary: { beneficiary: userAddress }
        };
        const res = await queryContract(query);
        vestings = res;

        hideLoader();
        if (vestings.length === 0) {
            noVestingsEl.classList.remove('hidden');
        } else {
            noVestingsEl.classList.add('hidden');
            renderVestings();
        }
    } catch (e) {
        console.error("Refresh failed", e);
        hideLoader();
    }
}

async function fetchAdminData() {
    try {
        const configRes = await queryContract({ config: {} });
        document.getElementById('contract-admin').textContent = shortenAddress(configRes.admin);
        document.getElementById('contract-status').textContent = configRes.paused ? 'Paused' : 'Active';
        document.getElementById('contract-status').className = configRes.paused ?
            'bg-red-900/30 text-red-400 px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider' :
            'bg-green-900/30 text-green-400 px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider';

        // Placeholder for global stats since we need a token address
        // In a real app, we might query for the most common token or all tokens
    } catch (e) {
        console.error("Fetch admin data failed", e);
    }
}

// --- Rendering ---

function renderVestings() {
    vestingListEl.innerHTML = vestings.map(v => `
        <div class="bg-dark-800 rounded-2xl p-6 border border-gray-800 hover:border-paxi-500/50 transition-all group shadow-xl">
            <div class="flex justify-between items-start mb-4">
                <span class="text-xs font-bold uppercase tracking-widest text-paxi-500 bg-paxi-500/10 px-2 py-1 rounded">${v.category}</span>
                <span class="text-xs text-gray-500 font-mono">ID: ${v.id}</span>
            </div>

            <h3 class="text-xl font-bold text-white mb-1">${formatAmount(v.total_amount)} <span class="text-sm font-normal text-gray-400">Tokens</span></h3>
            <p class="text-xs text-gray-500 font-mono truncate mb-6">${v.token_address}</p>

            <div class="space-y-4">
                <div>
                    <div class="flex justify-between text-xs mb-1">
                        <span class="text-gray-400">Vesting Progress</span>
                        <span class="text-white">${calculateProgress(v)}%</span>
                    </div>
                    <div class="w-full bg-dark-700 h-2 rounded-full overflow-hidden">
                        <div class="bg-paxi-500 h-full rounded-full transition-all duration-1000" style="width: ${calculateProgress(v)}%"></div>
                    </div>
                </div>

                <div class="grid grid-cols-2 gap-4 pt-4 border-t border-gray-700">
                    <div>
                        <p class="text-[10px] text-gray-500 uppercase font-bold">Claimable</p>
                        <p class="text-sm font-bold text-green-400">${formatAmount(v.claimable_amount)}</p>
                    </div>
                    <div>
                        <p class="text-[10px] text-gray-500 uppercase font-bold">Released</p>
                        <p class="text-sm font-bold text-gray-300">${formatAmount(v.released_amount)}</p>
                    </div>
                </div>

                <button onclick="claimVesting(${v.id})" ${v.claimable_amount === "0" ? 'disabled' : ''}
                    class="w-full mt-2 bg-paxi-600 hover:bg-paxi-500 disabled:opacity-30 disabled:hover:bg-paxi-600 text-white py-3 rounded-xl font-bold transition-all active:scale-[0.98]">
                    Claim Tokens
                </button>
            </div>
        </div>
    `).join('');
}

// --- Contract Interactions ---

async function handleCreateVesting(e) {
    e.preventDefault();
    if (!userAddress) return connectWallet();

    const token = document.getElementById('token-address').value;
    const beneficiary = document.getElementById('beneficiary-address').value;
    const amount = document.getElementById('vesting-amount').value;
    const category = document.getElementById('vesting-category').value;
    const isRevocable = document.getElementById('revocable').checked;

    let schedule;
    if (vestingType === 'linear') {
        const start = Math.floor(new Date(document.getElementById('start-time').value).getTime() / 1000);
        const end = Math.floor(new Date(document.getElementById('end-time').value).getTime() / 1000);
        const interval = parseInt(document.getElementById('interval').value);
        schedule = {
            linear: {
                start_time: start,
                end_time: end,
                release_interval: interval
            }
        };
    } else {
        schedule = {
            custom: {
                milestones: JSON.parse(document.getElementById('milestones-json').value)
            }
        };
    }

    // Step 1: Send CW20 Receive Hook
    // We send to the token contract, telling it to "send" to the vesting contract with a hook
    const msgObj = {
        send: {
            contract: VESTING_CONTRACT,
            amount: toBase64Amount(amount),
            msg: btoa(JSON.stringify({
                create_vesting: {
                    beneficiary,
                    schedule,
                    category,
                    revocable: isRevocable
                }
            }))
        }
    };

    await executeContract(token, msgObj, "Create Vesting");
}

async function claimVesting(id) {
    const msg = { claim: { ids: [id] } };
    await executeContract(VESTING_CONTRACT, msg, "Claim Tokens");
}

// --- Core Utils ---

async function queryContract(query) {
    const encoded = btoa(JSON.stringify(query));
    const url = `${LCD}/cosmwasm/wasm/v1/contract/${VESTING_CONTRACT}/smart/${encoded}`;
    const res = await fetch(url);
    const data = await res.json();
    return data.data;
}

async function executeContract(contract, msgObj, title) {
    showModal();
    try {
        const sender = await window.paxihub.paxi.getAddress();
        const msg = PaxiCosmJS.MsgExecuteContract.fromPartial({
            sender: sender.address,
            contract: contract,
            msg: new TextEncoder().encode(JSON.stringify(msgObj))
        });

        const anyMsg = PaxiCosmJS.Any.fromPartial({
            typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
            value: PaxiCosmJS.MsgExecuteContract.encode(msg).finish()
        });

        // Use helper from Paxi docs
        const result = await buildAndSendTx([ anyMsg ], title);

        if (result && result.tx_response && result.tx_response.code === 0) {
            showSuccess(result.tx_response.txhash);
            refreshData();
        } else {
            showError(result?.tx_response?.raw_log || "Transaction failed");
        }
    } catch (e) {
        showError(e.message);
    }
}

// Re-using the buildAndSendTx logic from documentation with slight adjustments
async function buildAndSendTx(messages, memo = "") {
    const chainId = await fetch(`${RPC}/status`)
        .then(r => r.json())
        .then(d => d.result.node_info.network);

    const sender = await window.paxihub.paxi.getAddress();
    const { accountNumber, sequence } = await fetchAccountInfo(sender.address);

    const txBody = PaxiCosmJS.TxBody.fromPartial({ messages, memo });
    const fee = {
        amount: [ PaxiCosmJS.coins("30000", DENOM)[0] ],
        gasLimit: 600_000
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

    const signResult = await window.paxihub.paxi.signAndSendTransaction(txObj);
    const sigBytes = Uint8Array.from(atob(signResult.success), c => c.charCodeAt(0));

    const txRaw = PaxiCosmJS.TxRaw.fromPartial({
        bodyBytes: signDoc.bodyBytes,
        authInfoBytes: signDoc.authInfoBytes,
        signatures: [ sigBytes ]
    });

    const txBytes = PaxiCosmJS.TxRaw.encode(txRaw).finish();
    const base64Tx = btoa(String.fromCharCode(...txBytes));

    const broadcastResult = await fetch(`${LCD}/cosmos/tx/v1beta1/txs`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tx_bytes: base64Tx, mode: "BROADCAST_MODE_SYNC" })
    }).then(r => r.json());

    return broadcastResult;
}

async function fetchAccountInfo(address) {
    const res = await fetch(`${LCD}/cosmos/auth/v1beta1/accounts/${address}`);
    const { account } = await res.json();
    const ba = account.base_account || account;
    return {
        accountNumber: Number(ba.account_number),
        sequence: Number(ba.sequence)
    };
}

// --- Helpers ---

function shortenAddress(addr) {
    if (!addr) return "...";
    return addr.slice(0, 8) + "..." + addr.slice(-4);
}

function formatAmount(amount) {
    return (parseFloat(amount) / 1_000_000).toLocaleString(undefined, { minimumFractionDigits: 2 });
}

function toBase64Amount(amount) {
    return (parseFloat(amount) * 1_000_000).toString();
}

function calculateProgress(v) {
    const total = parseFloat(v.total_amount);
    const released = parseFloat(v.released_amount);
    const claimable = parseFloat(v.claimable_amount);
    const vested = released + claimable;
    return Math.min(100, Math.floor((vested / total) * 100));
}

// --- UI Logic ---

function showLoader() {
    vestingListEl.innerHTML = `
        <div class="bg-dark-800 rounded-2xl p-6 border border-gray-800 animate-pulse">
            <div class="h-4 bg-dark-700 rounded w-1/4 mb-4"></div>
            <div class="h-8 bg-dark-700 rounded w-3/4 mb-6"></div>
            <div class="space-y-3">
                <div class="h-3 bg-dark-700 rounded w-full"></div>
                <div class="h-3 bg-dark-700 rounded w-full"></div>
            </div>
        </div>
    `.repeat(3);
}

function hideLoader() {
    // handled by refreshData
}

function showModal() {
    document.getElementById('tx-modal').classList.remove('hidden');
    document.getElementById('tx-loading').classList.remove('hidden');
    document.getElementById('tx-success').classList.add('hidden');
    document.getElementById('tx-error').classList.add('hidden');
}

function closeModal() {
    document.getElementById('tx-modal').classList.add('hidden');
}

function showSuccess(hash) {
    document.getElementById('tx-loading').classList.add('hidden');
    document.getElementById('tx-success').classList.remove('hidden');
    document.getElementById('tx-hash').textContent = hash;
}

function showError(msg) {
    document.getElementById('tx-loading').classList.add('hidden');
    document.getElementById('tx-error').classList.remove('hidden');
    document.getElementById('error-message').textContent = msg;
}
