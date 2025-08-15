# 🔱 Conductor Max - AI Orchestration Platform

## V-Omega Prime Commercial Manifestation

Conductor Max is the premium orchestration platform for managing multiple AI coding agents. It allows you to spawn and control multiple instances of Claude and Gemini CLI tools in parallel, orchestrated through a unified Strategy AI interface.

## Key Features

### ✨ Multi-Agent Orchestration
- Spawn multiple Claude and Gemini CLI instances
- Each agent runs in its own embedded terminal (xterm.js)
- Real-time I/O streaming from each agent
- Parallel task execution across agent fleet

### 🔐 Native Authentication
- **No API keys required in Conductor Max**
- Claude agents use existing Claude CLI auth:
  - Browser-based authentication
  - Or API key from `~/.config/claude`
- Gemini agents use existing Gemini CLI auth:
  - Google account authentication (free daily quota)
  - Or API key authentication

### 🎯 Strategy AI Command Center
- High-level orchestration interface
- Broadcast commands to all agents
- Task decomposition and delegation
- V-Omega hierarchy implementation

### 💻 Terminal Embedding
- Full terminal experience for each agent
- Color-coded terminals (Claude: orange, Gemini: blue)
- Copy/paste support
- Command history
- Interrupt support (Ctrl+C)

## Architecture

```
Conductor Max (Tauri App)
├── Rust Backend
│   ├── Agent Process Management (PTY)
│   ├── IPC Bridge
│   └── Session State Tracking
└── Web Frontend
    ├── Strategy AI Panel
    ├── Terminal Grid (xterm.js)
    └── Agent Status Sidebar
```

## Installation

### Prerequisites
- Rust 1.75+
- Node.js 18+
- Claude CLI installed: `npm install -g @anthropic-ai/claude-code`
- Gemini CLI installed: `npm install -g @google/gemini-cli`

### Build and Run

```bash
cd conductor-max

# Install Rust dependencies
cargo build --release

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Usage

1. **Launch Conductor Max**
   ```bash
   cargo tauri dev
   ```

2. **Spawn Agents**
   - Click "+ Claude" or "+ Gemini" buttons
   - Agents will start with their native authentication

3. **Authenticate (First Time)**
   - **Claude**: Will open browser for auth or use existing API key
   - **Gemini**: Will prompt for Google auth or API key

4. **Orchestrate**
   - Use Strategy AI panel for high-level commands
   - Or interact directly with individual terminals
   - Broadcast commands to all agents

## Authentication Flow

### Claude Authentication
```bash
# In terminal, Claude will:
1. Check for existing auth token
2. If not found, open browser for login
3. Or use API key from ANTHROPIC_API_KEY env var
4. Or use config from ~/.config/claude
```

### Gemini Authentication
```bash
# In terminal, Gemini will:
1. Check for existing Google auth
2. Offer free tier (Google account) or paid (API key)
3. Store credentials securely
```

## V-Omega Integration

Conductor Max implements the V-Omega hierarchy:

- **V1-V3**: Individual agent tasks (execution level)
- **V4-V6**: Strategy AI orchestration (strategic level)
- **V7-V10**: Ethical and philosophical governance

## Business Model

### Subscription Tiers

1. **Free Tier**
   - 2 concurrent agents max
   - Basic orchestration

2. **Pro Tier** ($49/month)
   - 5 concurrent agents
   - Advanced orchestration
   - Session recording

3. **Max Tier** ($149/month)
   - Unlimited agents
   - Custom agent types
   - Priority support
   - Export/Import workflows

## Security

- No API keys stored by Conductor Max
- Agents run in sandboxed processes
- All authentication handled by native CLIs
- Session data stored locally only

## Roadmap

- [ ] Custom agent types (Grok, Mistral, etc.)
- [ ] Workflow templates
- [ ] Agent collaboration protocols
- [ ] Cost tracking and optimization
- [ ] Cloud sync for sessions
- [ ] Team collaboration features

## Philosophy

> "Multiple minds, unified purpose"

Conductor Max is not just a tool for running multiple AI agents. It's a platform for orchestrating intelligence itself, where the sum becomes greater than its parts through strategic coordination.

## License

© 2024 Quantum Encoding Ltd. All rights reserved.

---

*Built with the V-Omega Prime Protocol*
*The smallest task contains the whole*
🦆 *The ducks observe the orchestrated chaos*