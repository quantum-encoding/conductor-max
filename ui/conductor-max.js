// 🔱 Conductor Max - Frontend Orchestration
const { invoke } = window.__TAURI__ ? window.__TAURI__.core : { invoke: mockInvoke };

// Global state
let agents = new Map();
let terminals = new Map();
let sessionStartTime = Date.now();
let totalCommands = 0;

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    console.log('🔱 Conductor Max initializing...');
    initializeEventListeners();
    startMetricsUpdater();
    
    // Show initial help
    addStrategyMessage('system', `Welcome to Conductor Max!
    
Spawn Claude or Gemini agents using the buttons on the left.
Agents will use their native authentication methods:
• Claude: Browser-based auth or API key from ~/.config/claude
• Gemini: Google account auth or API key

You can run commands directly in each terminal or orchestrate from this Strategy panel.`);
});

// Event Listeners
function initializeEventListeners() {
    const strategyInput = document.getElementById('strategy-input');
    strategyInput?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && e.ctrlKey) {
            sendStrategyMessage();
        }
    });
}

// Agent Management
async function spawnAgent(type) {
    try {
        console.log(`Spawning ${type} agent...`);
        
        // No API key needed - agents handle their own auth
        const agentId = await invoke('spawn_agent', {
            agent_type: type,
            api_key: '', // Empty - not needed
            agent_id: null
        });
        
        console.log(`Agent spawned with ID: ${agentId}`);
        
        // Create agent entry
        const agent = {
            id: agentId,
            type: type,
            status: 'running',
            startTime: Date.now()
        };
        
        agents.set(agentId, agent);
        
        // Add to UI
        addAgentToSidebar(agent);
        addAgentTerminal(agent);
        
        // Update metrics
        updateMetrics();
        
        addStrategyMessage('system', `✅ ${type} agent spawned (ID: ${agentId})`);
        
    } catch (error) {
        console.error('Failed to spawn agent:', error);
        addStrategyMessage('error', `Failed to spawn ${type} agent: ${error}`);
    }
}

function addAgentToSidebar(agent) {
    const agentList = document.getElementById('agent-list');
    
    const agentEl = document.createElement('div');
    agentEl.className = `agent-instance ${agent.type}`;
    agentEl.dataset.agentId = agent.id;
    agentEl.innerHTML = `
        <div class="agent-header">
            <span class="agent-name">${agent.type.toUpperCase()}-${agent.id.slice(0, 8)}</span>
            <span class="agent-status running">●</span>
        </div>
        <div class="agent-info">
            <span class="agent-start">Started: ${new Date(agent.startTime).toLocaleTimeString()}</span>
        </div>
    `;
    
    agentEl.addEventListener('click', () => focusAgentTerminal(agent.id));
    agentList.appendChild(agentEl);
}

function addAgentTerminal(agent) {
    const grid = document.getElementById('terminal-grid');
    
    // Remove placeholder if it exists
    const placeholder = grid.querySelector('.grid-placeholder');
    if (placeholder) {
        placeholder.remove();
    }
    
    // Clone terminal template
    const template = document.getElementById('terminal-template');
    const terminalEl = template.content.cloneNode(true).querySelector('.terminal-container');
    
    terminalEl.dataset.agentId = agent.id;
    terminalEl.classList.add(agent.type);
    terminalEl.querySelector('.terminal-title').textContent = 
        `${agent.type.toUpperCase()} Terminal - ${agent.id.slice(0, 8)}`;
    
    grid.appendChild(terminalEl);
    
    // Initialize XTerm.js
    const term = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: 'Fira Code, monospace',
        theme: {
            background: '#000000',
            foreground: '#00ff00',
            cursor: '#00ff00',
            selection: 'rgba(0, 255, 0, 0.3)',
            black: '#000000',
            red: '#ff5555',
            green: '#00ff00',
            yellow: '#ffff55',
            blue: '#5555ff',
            magenta: '#ff55ff',
            cyan: '#55ffff',
            white: '#bbbbbb'
        }
    });
    
    const fitAddon = new FitAddon.FitAddon();
    const webLinksAddon = new WebLinksAddon.WebLinksAddon();
    
    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    
    const termBody = terminalEl.querySelector('.terminal-body');
    term.open(termBody);
    fitAddon.fit();
    
    // Welcome message
    term.writeln(`🔱 ${agent.type.toUpperCase()} Agent Terminal`);
    term.writeln(`ID: ${agent.id}`);
    term.writeln('─'.repeat(50));
    term.writeln('');
    
    if (agent.type === 'claude') {
        term.writeln('Starting Claude CLI...');
        term.writeln('Authentication will use your existing Claude setup.');
        term.writeln('Run "claude --help" for commands.');
    } else if (agent.type === 'gemini') {
        term.writeln('Starting Gemini CLI...');
        term.writeln('Authentication will use your Google account or API key.');
        term.writeln('Run "gemini --help" for commands.');
    }
    
    term.writeln('');
    term.write('$ ');
    
    // Handle input
    let currentLine = '';
    term.onData(data => {
        if (data === '\r') { // Enter
            term.writeln('');
            if (currentLine.trim()) {
                sendCommandToAgent(agent.id, currentLine);
                totalCommands++;
                updateMetrics();
            }
            currentLine = '';
            term.write('$ ');
        } else if (data === '\u007F') { // Backspace
            if (currentLine.length > 0) {
                currentLine = currentLine.slice(0, -1);
                term.write('\b \b');
            }
        } else if (data === '\u0003') { // Ctrl+C
            term.writeln('^C');
            sendCommandToAgent(agent.id, '\x03');
            currentLine = '';
            term.write('$ ');
        } else {
            currentLine += data;
            term.write(data);
        }
    });
    
    // Store terminal reference
    terminals.set(agent.id, term);
    
    // Start output streaming
    startOutputStreaming(agent.id, term);
    
    // Handle resize
    window.addEventListener('resize', () => fitAddon.fit());
}

async function sendCommandToAgent(agentId, command) {
    try {
        await invoke('send_to_agent', {
            agent_id: agentId,
            command: command
        });
        
        console.log(`Sent command to ${agentId}: ${command}`);
    } catch (error) {
        console.error('Failed to send command:', error);
        const term = terminals.get(agentId);
        if (term) {
            term.writeln(`\r\n\x1b[31mError: ${error}\x1b[0m`);
        }
    }
}

async function startOutputStreaming(agentId, term) {
    // Poll for output
    const pollOutput = async () => {
        try {
            const output = await invoke('get_agent_output', {
                agent_id: agentId,
                lines: 50
            });
            
            if (output && output.length > 0) {
                // Clear and rewrite (simple approach)
                // In production, would track last position
                term.clear();
                output.forEach(line => {
                    term.writeln(line);
                });
                term.write('$ ');
            }
        } catch (error) {
            console.error('Failed to get output:', error);
        }
    };
    
    // Poll every 500ms
    const interval = setInterval(() => {
        if (!agents.has(agentId)) {
            clearInterval(interval);
            return;
        }
        pollOutput();
    }, 500);
}

async function killAgent(button) {
    const container = button.closest('.terminal-container');
    const agentId = container.dataset.agentId;
    
    if (!confirm(`Kill agent ${agentId}?`)) return;
    
    try {
        await invoke('kill_agent', { agent_id: agentId });
        
        // Remove from maps
        agents.delete(agentId);
        terminals.delete(agentId);
        
        // Remove from UI
        container.remove();
        document.querySelector(`[data-agent-id="${agentId}"]`)?.remove();
        
        // Add placeholder if no agents
        if (agents.size === 0) {
            addTerminalPlaceholder();
        }
        
        updateMetrics();
        addStrategyMessage('system', `Agent ${agentId} terminated`);
        
    } catch (error) {
        console.error('Failed to kill agent:', error);
        addStrategyMessage('error', `Failed to kill agent: ${error}`);
    }
}

async function killAllAgents() {
    if (!confirm('Stop all agents?')) return;
    
    for (const [agentId] of agents) {
        try {
            await invoke('kill_agent', { agent_id: agentId });
        } catch (error) {
            console.error(`Failed to kill agent ${agentId}:`, error);
        }
    }
    
    // Clear everything
    agents.clear();
    terminals.clear();
    document.getElementById('agent-list').innerHTML = '';
    document.getElementById('terminal-grid').innerHTML = '';
    addTerminalPlaceholder();
    
    updateMetrics();
    addStrategyMessage('system', 'All agents stopped');
}

function addTerminalPlaceholder() {
    const grid = document.getElementById('terminal-grid');
    grid.innerHTML = `
        <div class="grid-placeholder">
            <div class="placeholder-content">
                <span class="placeholder-icon">🚀</span>
                <h3>No Agents Active</h3>
                <p>Spawn Claude or Gemini agents to see their terminals here</p>
                <p class="auth-note">
                    Agents use their native authentication:<br>
                    • Claude: Browser auth or API key<br>
                    • Gemini: Google account or API key
                </p>
            </div>
        </div>
    `;
}

// Terminal controls
function clearTerminal(button) {
    const container = button.closest('.terminal-container');
    const agentId = container.dataset.agentId;
    const term = terminals.get(agentId);
    if (term) {
        term.clear();
        term.write('$ ');
    }
}

function resizeTerminal(button) {
    const container = button.closest('.terminal-container');
    container.classList.toggle('expanded');
    
    // Refit terminal
    const agentId = container.dataset.agentId;
    const term = terminals.get(agentId);
    if (term && term._addonManager) {
        const fitAddon = term._addonManager._addons.find(a => a.instance.constructor.name === 'FitAddon');
        if (fitAddon) fitAddon.instance.fit();
    }
}

function focusAgentTerminal(agentId) {
    const terminal = document.querySelector(`.terminal-container[data-agent-id="${agentId}"]`);
    if (terminal) {
        terminal.scrollIntoView({ behavior: 'smooth' });
        terminal.classList.add('highlight');
        setTimeout(() => terminal.classList.remove('highlight'), 1000);
    }
}

// Strategy AI functions
function sendStrategyMessage() {
    const input = document.getElementById('strategy-input');
    const message = input.value.trim();
    
    if (!message) return;
    
    addStrategyMessage('user', message);
    input.value = '';
    
    // Process strategic command
    processStrategyCommand(message);
}

function addStrategyMessage(type, content) {
    const messages = document.getElementById('strategy-messages');
    const messageEl = document.createElement('div');
    messageEl.className = `message ${type}`;
    
    const sender = type === 'user' ? 'Architect' : 
                   type === 'system' ? 'System' : 
                   type === 'error' ? 'Error' : 'Strategy AI';
    
    messageEl.innerHTML = `<strong>${sender}:</strong> ${content}`;
    messages.appendChild(messageEl);
    messages.scrollTop = messages.scrollHeight;
}

async function processStrategyCommand(command) {
    // Parse high-level commands
    if (command.toLowerCase().includes('all agents') || command.toLowerCase().includes('broadcast')) {
        await broadcastToAgents();
    } else if (command.toLowerCase().includes('spawn')) {
        if (command.toLowerCase().includes('claude')) {
            await spawnAgent('claude');
        }
        if (command.toLowerCase().includes('gemini')) {
            await spawnAgent('gemini');
        }
    } else {
        // Send to specific agent or all
        addStrategyMessage('ai', `Processing strategic directive: "${command}"`);
        
        // Decompose and delegate
        setTimeout(() => {
            addStrategyMessage('ai', `Decomposing task into V2 operations...`);
            // In production, would actually decompose and route
        }, 500);
    }
}

async function broadcastToAgents() {
    const input = document.getElementById('strategy-input');
    const message = input.value.trim() || 'Status check';
    
    addStrategyMessage('system', `Broadcasting to all agents: "${message}"`);
    
    for (const [agentId] of agents) {
        await sendCommandToAgent(agentId, message);
    }
}

// Session management
async function exportSession() {
    try {
        const sessionData = {
            startTime: sessionStartTime,
            duration: Date.now() - sessionStartTime,
            agents: Array.from(agents.values()),
            totalCommands: totalCommands,
            timestamp: new Date().toISOString()
        };
        
        const blob = new Blob([JSON.stringify(sessionData, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `conductor-max-session-${Date.now()}.json`;
        a.click();
        
        addStrategyMessage('system', 'Session exported successfully');
    } catch (error) {
        console.error('Failed to export session:', error);
        addStrategyMessage('error', 'Failed to export session');
    }
}

// Metrics updater
function startMetricsUpdater() {
    setInterval(updateMetrics, 1000);
}

function updateMetrics() {
    // Update duration
    const duration = Date.now() - sessionStartTime;
    const hours = Math.floor(duration / 3600000);
    const minutes = Math.floor((duration % 3600000) / 60000);
    const seconds = Math.floor((duration % 60000) / 1000);
    document.getElementById('session-duration').textContent = 
        `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
    
    // Update counts
    document.getElementById('total-commands').textContent = totalCommands;
    document.getElementById('agent-count').textContent = agents.size;
    document.getElementById('active-agents').textContent = agents.size;
}

// Mock invoke for development
async function mockInvoke(cmd, args) {
    console.log('Mock invoke:', cmd, args);
    
    switch (cmd) {
        case 'spawn_agent':
            return `mock-${args.agent_type}-${Date.now()}`;
        case 'send_to_agent':
            return null;
        case 'kill_agent':
            return null;
        case 'get_agent_output':
            return [`Mock output for ${args.agent_id}`];
        default:
            throw new Error(`Unknown command: ${cmd}`);
    }
}