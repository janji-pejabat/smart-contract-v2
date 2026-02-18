require('dotenv').config();
const express = require('express');
const http = require('http');
const WebSocket = require('ws');
const cors = require('cors');

const WalletManager = require('./services/WalletManager');
const BattleEngine = require('./engine/BattleEngine');
const RankingService = require('./services/RankingService');
const { initDB } = require('./database');

const app = express();
const server = http.createServer(app);
const wss = new WebSocket.Server({ server });

app.use(cors());
app.use(express.json());

// Initialize services
initDB();
WalletManager.init().catch(err => console.error("WalletManager init failed", err));

// Store session to client mapping for scoped broadcasting
const sessionParticipants = new Map(); // sessionId -> Set of ws

// HTTP Routes
app.get('/api/status', (req, res) => {
    res.json({ status: 'Stickman RPG Arena Backend Online', time: new Date() });
});

// WebSocket for Real-time Battle
wss.on('connection', (ws) => {
    console.log('New WebSocket connection');

    ws.on('message', (message) => {
        const data = JSON.parse(message);

        switch (data.type) {
            case 'JOIN_QUEUE':
                const match = RankingService.addToQueue(data.userId);
                // Map current ws to this user
                ws.userId = data.userId;

                if (match) {
                    const sessionId = BattleEngine.startBattle(match.playerA, match.playerB);

                    // Scope participants
                    const participants = new Set();
                    wss.clients.forEach(client => {
                        if (client.userId === match.playerA || client.userId === match.playerB) {
                            participants.add(client);
                            client.send(JSON.stringify({ type: 'MATCH_FOUND', sessionId, players: match }));
                        }
                    });
                    sessionParticipants.set(sessionId, participants);
                } else {
                    ws.send(JSON.stringify({ type: 'QUEUE_WAITING' }));
                }
                break;

            case 'BATTLE_INPUT':
                const newState = BattleEngine.processInput(data.sessionId, data.playerId, data.input);
                if (newState) {
                    // Scoped broadcast: Only send to participants of this specific session
                    const participants = sessionParticipants.get(data.sessionId);
                    if (participants) {
                        participants.forEach(client => {
                            if (client.readyState === WebSocket.OPEN) {
                                client.send(JSON.stringify({ type: 'BATTLE_STATE_UPDATE', sessionId: data.sessionId, state: newState }));
                            }
                        });
                    }

                    if (newState.status === 'finished') {
                        // Cleanup session
                        sessionParticipants.delete(data.sessionId);

                        // Handle post-battle logic (MMR update, Rewards)
                        RankingService.updateMMR(newState.winner === 'A' ? data.match.playerA : data.match.playerB,
                                                newState.winner === 'A' ? data.match.playerB : data.match.playerA);
                    }
                }
                break;

            case 'GET_LEADERBOARD':
                // Return top players
                ws.send(JSON.stringify({ type: 'LEADERBOARD_DATA', data: [] }));
                break;
        }
    });
});

const PORT = process.env.PORT || 3001;
server.listen(PORT, () => {
    console.log(`Server listening on port ${PORT}`);
});
