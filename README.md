# orbit

A CLI launcher for AI coding assistants (opencode, Gemini CLI, Claude Code) with multi-tenant workspace management, session tracking, and MCP server configuration.

Built in Rust. Runs on Linux and macOS.

---

## What it does

`orbit` resolves the right context for your AI session — tenant, project, repository, instructions, MCP servers — and launches the engine ready to work. It manages sessions via tmux and provides a terminal UI for navigating active sessions, launching new ones, and configuring the ecosystem.

```
orbit                    # open TUI dashboard
orbit launch             # launch with auto-resolved context
orbit launch --tenant acme --project api --engine opencode
orbit session list       # list active sessions
orbit session attach     # attach to a running session
```

---

## Prerequisites

- [tmux](https://github.com/tmux/tmux) — session management
- At least one AI engine installed:
  - [opencode](https://opencode.ai)
  - [Gemini CLI](https://github.com/google-gemini/gemini-cli)
  - [Claude Code](https://claude.ai/code)
- Node.js (optional — required for npx-based MCP servers)

---

## Installation

### Download binary (recommended)

Download the latest binary for your platform from [Releases](https://github.com/tensiply/orbit/releases):

```bash
# Linux (x86_64)
curl -fsSL https://github.com/tensiply/orbit/releases/latest/download/orbit-linux-x86_64 \
  -o ~/.local/bin/orbit && chmod +x ~/.local/bin/orbit
```

### Build from source

```bash
git clone https://github.com/tensiply/orbit.git
cd orbit
cargo build --release
cp target/release/orbit ~/.local/bin/
```

Requires Rust 1.75+.

---

## Setup

Run once after installation:

```bash
orbit setup
```

This creates `~/.config/orbit/config.toml` with your preferences:

```toml
[workspace]
ai_root = "~/AI"          # root of your AI workspace

[engine]
default = "opencode"      # default engine: opencode | gemini | claude
default_tenant = "work"   # default tenant

[install]
dir = "~/.local/bin"      # where the orbit binary lives
```

---

## Workspace structure

orbit resolves context from a directory tree under `ai_root` (default: `~/AI`):

```
~/AI/
├── mcp.json                          # global MCP servers
├── tenants/
│   └── acme/                         # tenant
│       ├── mcp.json                  # tenant-scoped MCP servers
│       ├── source-of-truth/          # tenant-level instructions & agents
│       └── projects/
│           └── api/                  # project
│               ├── source-of-truth/  # project-level instructions
│               └── repositories/
│                   └── backend/      # repository
│                       └── source-of-truth/
```

When you run `orbit launch --tenant acme --project api`, orbit:
1. Resolves the workspace path to the repository directory
2. Merges MCP servers from all scope layers (global → tenant → project → repo)
3. Assembles instructions and agent configs for the engine
4. Launches the engine inside a named tmux session

---

## MCP configuration

MCP servers can be configured at any scope level. The closer the scope, the higher the priority on conflict.

Edit `mcp.json` files directly, or use the TUI System tab (`orbit` → tab `[3]` → `[a]` to add):

```json
{
  "mcpServers": {
    "context7": {
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp"]
    },
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
    }
  }
}
```

---

## TUI

Run `orbit` with no arguments to open the interactive dashboard:

```
[1] Sessions   [2] Launch   [3] System
```

| Tab | Keys |
|-----|------|
| Sessions | `↑↓` navigate · `a/↵` attach · `K` kill · `d` details · `c` clean · `r` refresh |
| Launch | `↑↓` fields · `←→` engine · `Space` toggle tmux · `↵` launch |
| System | `↑↓` MCP nav · `a` add MCP · `x` remove · `s` daemon toggle · `r` refresh |

Press `?` for full keybindings.

---

## Session management

```bash
orbit session list              # list all tracked sessions
orbit session attach [id]       # attach to session (defaults to most recent)
orbit session kill <id>         # send SIGTERM to session process
orbit session clean             # remove files for dead sessions
```

---

## Development mode

orbit supports swapping between a stable binary and a local development build:

```bash
orbit dev enable   # symlink orbit → orbit-dev (requires token)
orbit dev disable  # restore stable binary
orbit dev status   # show current mode
```

Development binaries are token-gated to prevent accidental activation.

---

## Daemon

The orbit daemon runs in the background and provides session state via a Unix socket:

```bash
orbit daemon start   # start daemon in background
orbit daemon stop    # graceful shutdown
orbit daemon status  # uptime, session count, PID
```

Socket: `~/.local/share/orbit/orbit.sock`

---

## License

MIT — Copyright (c) 2026 Eloir Corona
